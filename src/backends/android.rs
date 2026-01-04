use oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioStream, AudioStreamBase, AudioStreamBuilder,
    AudioStreamSafe, DataCallbackResult, Input, InputPreset, Mono, PerformanceMode,
    SampleRateConversionQuality, SharingMode, Usage,
};

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

/// Create Android Oboe capture backend with hardware AEC.
/// Spawns a dedicated OS thread that owns the audio stream lifecycle.
/// Returns (sample_rate, buffer_size).
pub fn create_backend(public_sender: flume::Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    let (callback_tx, callback_rx) = flume::bounded::<Vec<f32>>(32);
    let (meta_tx, meta_rx) = flume::bounded::<Result<(u32, usize), AecError>>(1);

    std::thread::Builder::new()
        .name("android-audio".to_string())
        .spawn(move || {
            let handler = InputHandler {
                sender: callback_tx,
            };

            let mut stream = match AudioStreamBuilder::default()
                .set_direction::<Input>()
                .set_usage(Usage::VoiceCommunication)
                .set_input_preset(InputPreset::VoiceCommunication)
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sharing_mode(SharingMode::Exclusive)
                .set_sample_rate(48000)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Medium)
                .set_format::<f32>()
                .set_mono()
                .set_callback(handler)
                .open_stream()
            {
                Ok(s) => s,
                Err(e) => {
                    let _ = meta_tx.send(Err(AecError::BackendError(format!(
                        "Oboe stream open failed: {e:?}"
                    ))));
                    return;
                }
            };

            let sample_rate = stream.get_sample_rate() as u32;
            let buffer_size = stream.get_frames_per_burst() as usize;

            if let Err(e) = stream.start() {
                let _ = meta_tx.send(Err(AecError::BackendError(format!(
                    "Oboe stream start failed: {e:?}"
                ))));
                return;
            }

            let _ = meta_tx.send(Ok((sample_rate, buffer_size)));

            while let Ok(samples) = callback_rx.recv() {
                if public_sender.send(samples).is_err() {
                    break;
                }
            }

            let _ = stream.stop();
        })
        .map_err(|e| AecError::BackendError(format!("failed to spawn audio thread: {e:?}")))?;

    meta_rx.recv().map_err(|_| {
        AecError::BackendError("audio thread died before sending metadata".to_string())
    })?
}
