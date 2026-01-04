use sys_voice::{AecConfig, AecError, CaptureHandle, Channels};

#[test]
fn test_aec_config_creation() {
    let config = AecConfig {
        sample_rate: 48000,
        channels: Channels::Mono,
    };
    assert_eq!(config.sample_rate, 48000);
    assert_eq!(config.channels, Channels::Mono);
}

#[test]
fn test_aec_config_stereo() {
    let config = AecConfig {
        sample_rate: 48000,
        channels: Channels::Stereo,
    };
    assert_eq!(config.channels, Channels::Stereo);
}

#[test]
fn test_error_display() {
    let err = AecError::DeviceUnavailable;
    let msg = format!("{err}");
    assert!(msg.contains("unavailable") || msg.contains("device"));

    let err = AecError::PermissionDenied;
    let msg = format!("{err}");
    assert!(msg.contains("denied") || msg.contains("permission"));

    let err = AecError::AecNotSupported;
    let msg = format!("{err}");
    assert!(msg.contains("AEC") || msg.contains("supported"));

    let err = AecError::InvalidConfig("bad config".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("bad config"));

    let err = AecError::BackendError("backend failed".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("backend failed"));
}

#[tokio::test]
#[cfg(target_os = "macos")]
#[ignore] // Requires audio hardware - run locally with: cargo test -- --ignored
async fn test_macos_stream_creation() {
    let config = AecConfig {
        sample_rate: 48000,
        channels: Channels::Mono,
    };

    let result = CaptureHandle::new(config);

    match result {
        Ok(handle) => {
            drop(handle);
        }
        Err(AecError::PermissionDenied) => {}
        Err(AecError::DeviceUnavailable) => {}
        Err(AecError::BackendError(_)) => {}
        Err(e) => {
            panic!("Unexpected error: {e:?}");
        }
    }
}

#[tokio::test]
#[cfg(target_os = "windows")]
#[ignore] // Requires audio hardware - run locally with: cargo test -- --ignored
async fn test_windows_stream_creation() {
    let config = AecConfig {
        sample_rate: 48000,
        channels: Channels::Mono,
    };

    let result = CaptureHandle::new(config);

    match result {
        Ok(handle) => {
            drop(handle);
        }
        Err(AecError::PermissionDenied) => {}
        Err(AecError::DeviceUnavailable) => {}
        Err(AecError::AecNotSupported) => {}
        Err(e) => {
            panic!("Unexpected error: {e:?}");
        }
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
#[ignore] // Requires audio hardware - run locally with: cargo test -- --ignored
async fn test_linux_stream_creation() {
    let config = AecConfig {
        sample_rate: 48000,
        channels: Channels::Mono,
    };

    let result = CaptureHandle::new(config);

    match result {
        Ok(handle) => {
            drop(handle);
        }
        Err(AecError::DeviceUnavailable) => {}
        Err(AecError::BackendError(_)) => {}
        Err(e) => {
            panic!("Unexpected error: {e:?}");
        }
    }
}

/// Test that the requested sample rate is honored.
/// Bug: backends currently ignore config.sample_rate and use native rate instead.
#[tokio::test]
#[cfg(target_os = "macos")]
#[ignore] // Requires audio hardware - run locally with: cargo test -- --ignored
async fn test_sample_rate_is_honored() {
    let config = AecConfig {
        sample_rate: 16000,
        channels: Channels::Mono,
    };

    let result = CaptureHandle::new(config);

    match result {
        Ok(handle) => {
            assert_eq!(
                handle.native_sample_rate(),
                16000,
                "Sample rate should match requested rate (possibly after resampling)"
            );
        }
        Err(AecError::PermissionDenied) => {}
        Err(AecError::DeviceUnavailable) => {}
        Err(AecError::BackendError(_)) => {}
        Err(e) => {
            panic!("Unexpected error: {e:?}");
        }
    }
}
