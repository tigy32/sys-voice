#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "android")]
mod android;

use crate::AecError;

/// Handle for sending audio to the backend for playback.
/// Audio played through this handle goes through the same engine as capture,
/// enabling AEC to cancel it from the recorded audio.
#[derive(Clone)]
pub struct BackendHandle {
    playback_tx: flume::Sender<PlaybackRequest>,
}

pub(crate) struct PlaybackRequest {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl BackendHandle {
    pub fn play_audio(&self, samples: Vec<f32>, sample_rate: u32) -> Result<(), AecError> {
        self.playback_tx
            .send(PlaybackRequest {
                samples,
                sample_rate,
            })
            .map_err(|_| AecError::BackendError("playback channel closed".to_string()))
    }
}

/// Create the appropriate platform backend.
/// Spawns a capture task that owns audio resources.
/// Returns (sample_rate, buffer_size, handle). Task stops when sender disconnects.
pub(crate) fn create_backend(
    sender: flume::Sender<Vec<f32>>,
) -> Result<(u32, usize, BackendHandle), AecError> {
    let (playback_tx, playback_rx) = flume::bounded::<PlaybackRequest>(16);
    let handle = BackendHandle { playback_tx };

    #[cfg(target_os = "macos")]
    {
        let (rate, size) = macos::create_backend(sender, playback_rx)?;
        return Ok((rate, size, handle));
    }

    #[cfg(target_os = "ios")]
    {
        let (rate, size) = ios::create_backend(sender, playback_rx)?;
        return Ok((rate, size, handle));
    }

    #[cfg(target_os = "windows")]
    {
        let (rate, size) = windows::create_backend(sender, playback_rx)?;
        return Ok((rate, size, handle));
    }

    #[cfg(target_os = "linux")]
    {
        let (rate, size) = linux::create_backend(sender, playback_rx)?;
        return Ok((rate, size, handle));
    }

    #[cfg(target_os = "android")]
    {
        let (rate, size) = android::create_backend(sender, playback_rx)?;
        return Ok((rate, size, handle));
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = (sender, playback_rx);
        Err(AecError::AecNotSupported)
    }
}
