use crate::AecError;
use block2::RcBlock;
use flume::Sender;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSError, NSString};
use std::ptr;

const BUFFER_SIZE: u32 = 1024; // ~21ms at 48kHz

const AV_AUDIO_SESSION_CATEGORY_PLAY_AND_RECORD: &str = "AVAudioSessionCategoryPlayAndRecord";
const AV_AUDIO_SESSION_MODE_VOICE_CHAT: &str = "AVAudioSessionModeVoiceChat";

/// Create iOS AVAudioEngine capture backend.
/// Spawns a dedicated OS thread that owns all audio resources.
/// Returns (sample_rate, buffer_size).
pub fn create_backend(public_sender: Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    let (callback_tx, callback_rx) = flume::bounded::<Vec<f32>>(32);
    let (meta_tx, meta_rx) = flume::bounded::<Result<(u32, usize), AecError>>(1);

    std::thread::Builder::new()
        .name("ios-audio".to_string())
        .spawn(move || {
            // Configure audio session
            let _session = match configure_audio_session() {
                Ok(s) => s,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            // Create engine
            let engine = match create_engine() {
                Ok(e) => e,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            // Get input node and format
            let input_node = match get_input_node(&engine) {
                Ok(n) => n,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            let format = match get_input_format(&input_node) {
                Ok(f) => f,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            // Query actual sample rate from format
            let sample_rate: f64 = unsafe { msg_send![&format, sampleRate] };
            let native_sample_rate = sample_rate as u32;

            // Install tap
            let _tap_block = match install_tap(&input_node, &format, callback_tx) {
                Ok(b) => b,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            // Start engine
            let mut error: *mut NSError = ptr::null_mut();
            let started: bool = unsafe { msg_send![&engine, startAndReturnError: &mut error] };

            if !started {
                let msg = extract_nserror_message(error);
                let _ = meta_tx.send(Err(AecError::BackendError(format!(
                    "engine start failed: {msg}"
                ))));
                return;
            }

            // Send metadata back
            let _ = meta_tx.send(Ok((native_sample_rate, BUFFER_SIZE as usize)));

            // Forward samples until public_sender disconnects
            while let Ok(samples) = callback_rx.recv() {
                if public_sender.send(samples).is_err() {
                    break;
                }
            }

            // Cleanup on same thread - remove tap first, then stop engine
            unsafe {
                let _: () = msg_send![&input_node, removeTapOnBus: 0u64];
                let _: () = msg_send![&engine, stop];
            }
            // engine, input_node, format, _tap_block, _session all drop here
        })
        .map_err(|e| AecError::BackendError(format!("failed to spawn audio thread: {e:?}")))?;

    // Wait for metadata from the thread
    meta_rx.recv().map_err(|_| {
        AecError::BackendError("audio thread died before sending metadata".to_string())
    })?
}

/// Configure AVAudioSession for voice chat mode which enables hardware AEC
fn configure_audio_session() -> Result<Retained<AnyObject>, AecError> {
    let session_class = class!(AVAudioSession);
    let session: Retained<AnyObject> = unsafe { msg_send![session_class, sharedInstance] };

    let category = NSString::from_str(AV_AUDIO_SESSION_CATEGORY_PLAY_AND_RECORD);
    let mode = NSString::from_str(AV_AUDIO_SESSION_MODE_VOICE_CHAT);

    let mut error: *mut NSError = ptr::null_mut();

    // setCategory:mode:options:error: with options=0 (default)
    let success: bool = unsafe {
        msg_send![&session, setCategory: &*category, mode: &*mode, options: 0u64, error: &mut error]
    };

    if !success {
        let msg = extract_nserror_message(error);
        return Err(AecError::BackendError(format!(
            "failed to set audio session category: {msg}"
        )));
    }

    let mut error: *mut NSError = ptr::null_mut();
    let activated: bool = unsafe { msg_send![&session, setActive: true, error: &mut error] };

    if !activated {
        let msg = extract_nserror_message(error);
        return Err(AecError::BackendError(format!(
            "failed to activate audio session: {msg}"
        )));
    }

    Ok(session)
}

fn create_engine() -> Result<Retained<AnyObject>, AecError> {
    let engine_class = class!(AVAudioEngine);
    let engine: Retained<AnyObject> = unsafe { msg_send![engine_class, new] };
    Ok(engine)
}

fn get_input_node(engine: &Retained<AnyObject>) -> Result<Retained<AnyObject>, AecError> {
    let input_node: Retained<AnyObject> = unsafe { msg_send![engine, inputNode] };
    Ok(input_node)
}

fn get_input_format(input_node: &Retained<AnyObject>) -> Result<Retained<AnyObject>, AecError> {
    // outputFormatForBus:0 gets the format of data coming from the input node
    let format: Retained<AnyObject> = unsafe { msg_send![input_node, outputFormatForBus: 0u64] };
    Ok(format)
}

fn install_tap(
    input_node: &Retained<AnyObject>,
    format: &Retained<AnyObject>,
    sender: Sender<Vec<f32>>,
) -> Result<RcBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>, AecError> {
    let block = RcBlock::new(move |buffer: *mut AnyObject, _when: *mut AnyObject| {
        if buffer.is_null() {
            return;
        }

        let Some(samples) = (unsafe { extract_f32_samples(buffer) }) else {
            return;
        };

        let _ = sender.try_send(samples);
    });

    unsafe {
        let _: () = msg_send![
            input_node,
            installTapOnBus: 0u64,
            bufferSize: BUFFER_SIZE,
            format: &**format,
            block: &*block
        ];
    }

    Ok(block)
}

/// Extract f32 sample data from AVAudioPCMBuffer
/// Returns None if buffer format is not float or no data available
unsafe fn extract_f32_samples(buffer: *mut AnyObject) -> Option<Vec<f32>> {
    let frame_length: u32 = msg_send![buffer, frameLength];
    if frame_length == 0 {
        return None;
    }

    // floatChannelData returns float** (pointer to array of channel pointers)
    let channel_data: *const *const f32 = msg_send![buffer, floatChannelData];
    if channel_data.is_null() {
        return None;
    }

    // Voice processing typically provides mono; take first channel
    let channel_ptr = *channel_data;
    if channel_ptr.is_null() {
        return None;
    }

    let samples = std::slice::from_raw_parts(channel_ptr, frame_length as usize);
    Some(samples.to_vec())
}

fn extract_nserror_message(error: *mut NSError) -> String {
    if error.is_null() {
        return "unknown error".to_string();
    }

    unsafe {
        let description: Retained<NSString> = msg_send![error, localizedDescription];
        description.to_string()
    }
}
