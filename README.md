# sys-voice

Cross-platform native voice I/O with OS-level Acoustic Echo Cancellation (AEC).

## Current Status

This is slop by claude and has only been tested on osx and ios (and even those likely have bugs). Do not use this 
unless you are willing to test and help fix bugs.

## Platform Support

| Platform | Backend | AEC Method |
|----------|---------|------------|
| macOS | CoreAudio VoiceProcessingIO | Full hardware AEC |
| iOS | AVAudioEngine voiceChat mode | Full hardware AEC |
| Windows | WASAPI IAcousticEchoCancellationControl | Full hardware AEC |
| Linux | PulseAudio | Depends on module-echo-cancel |
| Android | Oboe VoiceCommunication | Hardware AEC |

## Quick Start

```rust
use sys_voice::{AecConfig, CaptureHandle, Channels};

let config = AecConfig {
    sample_rate: 48000,
    channels: Channels::Mono,
};

let handle = CaptureHandle::new(config)?;

// Receive samples (async, blocking, or non-blocking)
while let Some(result) = handle.recv_blocking() {
    match result {
        Ok(samples) => { /* Process AEC-enabled audio samples */ }
        Err(e) => { /* Handle audio error */ }
    }
}

// Handle automatically stops capture on drop
```

## Testing AEC

Run the included test tool to verify AEC is working on your system:

```bash
cargo run --example aec_test
```

The test tool:
1. Plays a 440Hz tone through your speakers
2. Records from the microphone with AEC enabled for 10 seconds
3. Saves the recording to `aec_recording.wav`

**Expected result:** The recording should contain your voice but NOT the 440Hz tone. If you hear the tone clearly in the recording, AEC may not be active on your system.

## Platform-Specific Notes

### macOS
- Requires microphone permission (System Preferences → Security & Privacy → Microphone)
- Uses VoiceProcessingIO audio unit which automatically monitors system output for echo reference
- macOS pauses/ducks other audio (Spotify, Apple Music, etc.) when VoiceProcessingIO is active. This is a system-level behavior that cannot be disabled.

### iOS
- Requires `NSMicrophoneUsageDescription` in Info.plist
- Uses AVAudioSession voiceChat mode which enables hardware AEC
- Permission must be granted before stream creation

### Windows
- Requires audio device with AEC support
- Uses WASAPI with IAcousticEchoCancellationControl
- Automatically links capture to render device for echo reference

### Linux
- Requires PulseAudio daemon running
- For AEC, load `module-echo-cancel`: `pactl load-module module-echo-cancel`
- The Simple API cannot pass media.role hints; AEC depends on system configuration

### Android
- Requires `RECORD_AUDIO` permission in AndroidManifest.xml
- Uses Oboe with VoiceCommunication usage which triggers hardware AEC
- Permission must be granted at runtime before stream creation

## iOS Testing

See [docs/ios-testing.md](docs/ios-testing.md) for detailed instructions on building and testing on iOS devices and simulators.

## Limitations

- **Linux**: The PulseAudio Simple API cannot pass media.role hints. AEC depends on whether `module-echo-cancel` is loaded in the system configuration.
- **Hardware AEC availability**: Some devices may not support hardware AEC. The library will still capture audio, but without echo cancellation.

## API Reference

### Channels

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Channels {
    #[default]
    Mono,
    Stereo,
}
```

### AecConfig

```rust
pub struct AecConfig {
    pub sample_rate: u32,   // Target sample rate (48000 recommended)
    pub channels: Channels, // Mono or Stereo (stereo = duplicated mono)
}
```

### CaptureHandle

```rust
impl CaptureHandle {
    pub fn new(config: AecConfig) -> Result<Self, AecError>;
    
    // Async receive (requires async runtime)
    pub async fn recv(&self) -> Option<Result<Vec<f32>, AecError>>;
    
    // Blocking receive
    pub fn recv_blocking(&self) -> Option<Result<Vec<f32>, AecError>>;
    
    // Non-blocking receive
    pub fn try_recv(&self) -> Option<Result<Vec<f32>, AecError>>;
    
    // Get the native sample rate
    pub fn native_sample_rate(&self) -> u32;
}
// Capture stops automatically on drop
```

### AecError

```rust
pub enum AecError {
    DeviceUnavailable,        // No capture device found
    PermissionDenied,         // Microphone access denied
    AecNotSupported,          // Platform doesn't support AEC
    InvalidConfig(String),    // Invalid configuration
    BackendError(String),     // Platform-specific error
}
```

## License

MIT
