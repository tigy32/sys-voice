use std::sync::{Arc, Mutex};

use oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioOutputCallback, AudioOutputStreamSafe,
    AudioStream, AudioStreamBase, AudioStreamBuilder, DataCallbackResult, Input, InputPreset, Mono,
    Output, PerformanceMode, SampleRateConversionQuality, SharingMode, Usage,
};

use crate::backends::PlaybackRequest;
use crate::AecError;

struct InputHandler {
    sender: flume::Sender<Vec<f32>>,
}

impl AudioInputCallback for InputHandler {
    type FrameType = (f32, Mono);

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioInputStreamSafe,
        frames: &[f32],
    ) -> DataCallbackResult {
        let _ = self.sender.try_send(frames.to_vec());
        DataCallbackResult::Continue
    }
}

struct OutputHandler {
    playback_buffer: Arc<Mutex<Vec<f32>>>,
}

impl AudioOutputCallback for OutputHandler {
    type FrameType = (f32, Mono);

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioOutputStreamSafe,
        frames: &mut [f32],
    ) -> DataCallbackResult {
        let mut buffer = match self.playback_buffer.lock() {
            Ok(b) => b,
            Err(_) => {
                frames.fill(0.0);
                return DataCallbackResult::Continue;
            }
        };
        let available = buffer.len().min(frames.len());
        frames[..available].copy_from_slice(&buffer[..available]);
        buffer.drain(..available);
        frames[available..].fill(0.0);
        DataCallbackResult::Continue
    }
}

fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = (src_pos - src_idx as f64) as f32;
        let sample = if src_idx + 1 < input.len() {
            input[src_idx] * (1.0 - frac) + input[src_idx + 1] * frac
        } else if src_idx < input.len() {
            input[src_idx]
        } else {
            0.0
        };
        output.push(sample);
    }
    output
}

/// Create Android Oboe capture backend with hardware AEC.
/// Spawns a dedicated OS thread that owns the audio stream lifecycle.
/// Returns (sample_rate, buffer_size).
const STREAM_SAMPLE_RATE: i32 = 48000;

/// Create Android Oboe capture backend with hardware AEC.
/// Spawns a dedicated OS thread that owns both input and output audio streams.
/// Returns (sample_rate, buffer_size).
pub fn create_backend(
    public_sender: flume::Sender<Vec<f32>>,
    playback_rx: flume::Receiver<PlaybackRequest>,
) -> Result<(u32, usize), AecError> {
    let playback_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(48000)));
    let playback_buffer_for_thread = playback_buffer.clone();

    let (callback_tx, callback_rx) = flume::bounded::<Vec<f32>>(32);
    let (meta_tx, meta_rx) = flume::bounded::<Result<(u32, usize), AecError>>(1);

    std::thread::Builder::new()
        .name("android-playback".to_string())
        .spawn(move || {
            let target_rate = STREAM_SAMPLE_RATE as u32;
            while let Ok(request) = playback_rx.recv() {
                let samples = resample_linear(&request.samples, request.sample_rate, target_rate);
                if let Ok(mut buffer) = playback_buffer_for_thread.lock() {
                    buffer.extend(samples);
                }
            }
        })
        .map_err(|e| AecError::BackendError(format!("failed to spawn playback thread: {e:?}")))?;

    std::thread::Builder::new()
        .name("android-audio".to_string())
        .spawn(move || {
            let input_handler = InputHandler {
                sender: callback_tx,
            };
            let output_handler = OutputHandler { playback_buffer };

            let mut input_stream = match AudioStreamBuilder::default()
                .set_direction::<Input>()
                .set_usage(Usage::VoiceCommunication)
                .set_input_preset(InputPreset::VoiceCommunication)
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sharing_mode(SharingMode::Exclusive)
                .set_sample_rate(STREAM_SAMPLE_RATE)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Medium)
                .set_format::<f32>()
                .set_mono()
                .set_callback(input_handler)
                .open_stream()
            {
                Ok(s) => s,
                Err(e) => {
                    let _ = meta_tx.send(Err(AecError::BackendError(format!(
                        "Oboe input stream open failed: {e:?}"
                    ))));
                    return;
                }
            };

            let mut output_stream = match AudioStreamBuilder::default()
                .set_direction::<Output>()
                .set_usage(Usage::VoiceCommunication)
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sharing_mode(SharingMode::Exclusive)
                .set_sample_rate(STREAM_SAMPLE_RATE)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Medium)
                .set_format::<f32>()
                .set_mono()
                .set_callback(output_handler)
                .open_stream()
            {
                Ok(s) => s,
                Err(e) => {
                    let _ = meta_tx.send(Err(AecError::BackendError(format!(
                        "Oboe output stream open failed: {e:?}"
                    ))));
                    return;
                }
            };

            let sample_rate = input_stream.get_sample_rate() as u32;
            let buffer_size = input_stream.get_frames_per_burst() as usize;

            if let Err(e) = input_stream.start() {
                let _ = meta_tx.send(Err(AecError::BackendError(format!(
                    "Oboe input stream start failed: {e:?}"
                ))));
                return;
            }

            if let Err(e) = output_stream.start() {
                let _ = input_stream.stop();
                let _ = meta_tx.send(Err(AecError::BackendError(format!(
                    "Oboe output stream start failed: {e:?}"
                ))));
                return;
            }

            let _ = meta_tx.send(Ok((sample_rate, buffer_size)));

            while let Ok(samples) = callback_rx.recv() {
                if public_sender.send(samples).is_err() {
                    break;
                }
            }

            let _ = input_stream.stop();
            let _ = output_stream.stop();
        })
        .map_err(|e| AecError::BackendError(format!("failed to spawn audio thread: {e:?}")))?;

    meta_rx.recv().map_err(|_| {
        AecError::BackendError("audio thread died before sending metadata".to_string())
    })?
}
