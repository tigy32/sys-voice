use crate::AecError;
use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::types::IOType;
use coreaudio::audio_unit::{AudioUnit, Element, SampleFormat, Scope, StreamFormat};
use flume::Sender;

/// Create macOS backend. Spawns a task that owns audio resources.
/// Returns (sample_rate, buffer_size). Task stops when sender fails.
pub fn create_backend(public_sender: Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    let (callback_tx, callback_rx) = flume::bounded::<Vec<f32>>(32);
    // Create VoiceProcessingIO audio unit - this enables OS-level AEC
    // VoiceProcessingIO automatically monitors system output for echo reference
    let mut audio_unit = AudioUnit::new(IOType::VoiceProcessingIO).map_err(|e| {
        AecError::BackendError(format!("failed to create VoiceProcessingIO: {e:?}"))
    })?;

    // coreaudio-rs may auto-initialize; must uninitialize before configuring properties
    let _ = audio_unit.uninitialize();

    let enable_input: u32 = 1;
    audio_unit
        .set_property(
            coreaudio::sys::kAudioOutputUnitProperty_EnableIO,
            Scope::Input,
            Element::Input,
            Some(&enable_input),
        )
        .map_err(|e| AecError::BackendError(format!("failed to enable input: {e:?}")))?;

    // Query native format - VoiceProcessingIO has strict requirements
    let native_format = audio_unit
        .stream_format(Scope::Output, Element::Input)
        .map_err(|e| AecError::BackendError(format!("failed to get native format: {e:?}")))?;

    // Use native sample rate but request f32 mono
    let stream_format = StreamFormat {
        sample_rate: native_format.sample_rate,
        sample_format: SampleFormat::F32,
        flags: coreaudio::audio_unit::audio_format::LinearPcmFlags::IS_FLOAT
            | coreaudio::audio_unit::audio_format::LinearPcmFlags::IS_PACKED,
        channels: 1,
    };

    audio_unit
        .set_stream_format(stream_format, Scope::Output, Element::Input)
        .map_err(|e| AecError::BackendError(format!("failed to set stream format: {e:?}")))?;

    let native_rate = native_format.sample_rate as u32;

    audio_unit
        .set_input_callback(move |args: render_callback::Args<data::Interleaved<f32>>| {
            let _ = callback_tx.try_send(args.data.buffer.to_vec());
            Ok(())
        })
        .map_err(|e| AecError::BackendError(format!("failed to set input callback: {e:?}")))?;

    audio_unit
        .initialize()
        .map_err(|e| AecError::BackendError(format!("failed to initialize: {e:?}")))?;

    audio_unit
        .start()
        .map_err(|e| AecError::BackendError(format!("failed to start: {e:?}")))?;

    // Query buffer size from audio unit (frames per slice)
    let buffer_size: u32 = audio_unit
        .get_property(
            coreaudio::sys::kAudioUnitProperty_MaximumFramesPerSlice,
            Scope::Global,
            Element::Output,
        )
        .unwrap_or(512);

    // Spawn task that owns audio_unit - stops on sender disconnect
    tokio::spawn(async move {
        let _audio_unit = audio_unit; // Hold for RAII, Drop stops audio
        while let Ok(samples) = callback_rx.recv_async().await {
            if public_sender.send_async(samples).await.is_err() {
                break;
            }
        }
    });

    Ok((native_rate, buffer_size as usize))
}
