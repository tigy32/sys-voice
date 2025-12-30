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

/// Create the appropriate platform backend.
/// Spawns a capture task that owns audio resources.
/// Returns (sample_rate, buffer_size). Task stops when sender disconnects.
pub(crate) fn create_backend(sender: flume::Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    #[cfg(target_os = "macos")]
    {
        macos::create_backend(sender)
    }

    #[cfg(target_os = "ios")]
    {
        ios::create_backend(sender)
    }

    #[cfg(target_os = "windows")]
    {
        windows::create_backend(sender)
    }

    #[cfg(target_os = "linux")]
    {
        linux::create_backend(sender)
    }

    #[cfg(target_os = "android")]
    {
        android::create_backend(sender)
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = (config, sender);
        Err(AecError::AecNotSupported)
    }
}
