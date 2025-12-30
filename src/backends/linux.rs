use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::AecError;

const SAMPLE_RATE: u32 = 48000;
const BUFFER_FRAMES: usize = 480; // 10ms at 48kHz

/// Create PulseAudio capture backend.
/// Spawns a blocking task that owns all PulseAudio resources.
/// Returns (sample_rate, buffer_size).
pub fn create_backend(sender: flume::Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    // Verify PulseAudio connection works before spawning task
    let simple = create_simple_stream()?;

    tokio::task::spawn_blocking(move || {
        let mut buffer = vec![0.0f32; BUFFER_FRAMES];

        loop {
            let byte_slice = unsafe {
                std::slice::from_raw_parts_mut(
                    buffer.as_mut_ptr() as *mut u8,
                    buffer.len() * std::mem::size_of::<f32>(),
                )
            };

            if simple.read(byte_slice).is_err() {
                break;
            }

            // When receiver is dropped, send fails and we exit
            if sender.send(buffer.clone()).is_err() {
                break;
            }
        }
    });

    Ok((SAMPLE_RATE, BUFFER_FRAMES))
}

fn create_simple_stream() -> Result<Simple, AecError> {
    let spec = Spec {
        format: Format::F32le,
        channels: 1,
        rate: SAMPLE_RATE,
    };

    if !spec.is_valid() {
        return Err(AecError::InvalidConfig(
            "Invalid PulseAudio sample spec".into(),
        ));
    }

    Simple::new(
        None,
        "native-voice-io",
        Direction::Record,
        None,
        "AEC Capture",
        &spec,
        None,
        None,
    )
    .map_err(|e| match e {
        libpulse_binding::error::Code::ConnectionRefused => AecError::DeviceUnavailable,
        libpulse_binding::error::Code::Access => AecError::PermissionDenied,
        _ => AecError::BackendError(format!("PulseAudio error: {e:?}")),
    })
}
