//! Flutter bridge API for sys-voice.
//!
//! This module exposes sys-voice functionality to Flutter via flutter_rust_bridge.

use sys_voice::{AecConfig, CaptureHandle, Channels};

/// Result from polling audio.
pub struct AudioPollResult {
    pub samples: Vec<f32>,
}

/// Opaque wrapper for sys-voice CaptureHandle with its tokio runtime.
#[flutter_rust_bridge::frb(opaque)]
pub struct VoiceCaptureHandle {
    handle: CaptureHandle,
    runtime: tokio::runtime::Runtime,
}

/// Start audio capture at the specified sample rate.
///
/// Returns a handle that can be used to poll audio samples and play audio through AEC.
pub fn start_capture(sample_rate: u32) -> Result<VoiceCaptureHandle, String> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {e:?}"))?;

    let config = AecConfig {
        sample_rate,
        channels: Channels::Mono,
    };

    // Use block_on to ensure the runtime is fully started and worker threads are running.
    // CaptureHandle::new() calls tokio::spawn() internally which requires an active runtime.
    // Using enter() alone may not start the worker threads - block_on ensures they're running.
    let handle = runtime
        .block_on(async { CaptureHandle::new(config) })
        .map_err(|e| format!("Failed to start capture: {e:?}"))?;

    Ok(VoiceCaptureHandle { handle, runtime })
}

impl VoiceCaptureHandle {
    /// Poll for available audio samples without blocking.
    ///
    /// Returns Ok(None) if no samples are available yet.
    /// Returns (samples, debug_info) where debug_info contains what Rust sees.
    #[flutter_rust_bridge::frb(sync)]
    pub fn poll_audio(&self) -> Result<Option<AudioPollResult>, String> {
        match self.handle.try_recv() {
            Some(Ok(samples)) => Ok(Some(AudioPollResult { samples })),
            Some(Err(e)) => Err(format!("Failed to receive audio: {e:?}")),
            None => Ok(None),
        }
    }

    /// Get the native sample rate of the capture device.
    #[flutter_rust_bridge::frb(sync)]
    pub fn sample_rate(&self) -> u32 {
        self.handle.native_sample_rate()
    }

    /// Play audio through the capture engine (AEC).
    ///
    /// Audio played through this method will be subtracted from the recording,
    /// enabling acoustic echo cancellation.
    #[flutter_rust_bridge::frb(sync)]
    pub fn play_audio(&self, samples: Vec<f32>, sample_rate: u32) -> Result<(), String> {
        self.handle
            .play_audio(samples, sample_rate)
            .map_err(|e| format!("Failed to play audio: {e:?}"))
    }
}

/// Placeholder function to verify the bridge works.
#[flutter_rust_bridge::frb(sync)]
pub fn greet(name: String) -> String {
    format!("Hello, {}! Voice bridge is ready.", name)
}

/// Initialize the flutter_rust_bridge runtime.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
