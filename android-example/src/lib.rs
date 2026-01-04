//! JNI bindings for sys-voice Android integration.

use jni::objects::{JClass, JFloatArray};
use jni::sys::{jfloat, jint};
use jni::JNIEnv;
use log::{error, info};
use std::sync::Mutex;
use sys_voice::{AecConfig, CaptureHandle, Channels};
use tokio::sync::oneshot;

#[no_mangle]
pub extern "system" fn JNI_OnLoad(
    _vm: jni::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jni::sys::jint {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("SysVoiceRust"),
    );
    info!("sys-voice-android-ffi native library loaded");
    jni::sys::JNI_VERSION_1_6
}

struct AndroidCaptureHandle {
    handle: CaptureHandle,
    // Sending on this channel stops the runtime
    shutdown_tx: Option<oneshot::Sender<()>>,
}

static CAPTURE: Mutex<Option<AndroidCaptureHandle>> = Mutex::new(None);

#[no_mangle]
pub extern "system" fn Java_com_example_sysvoice_VoiceCapture_nativeStart(
    _env: JNIEnv,
    _class: JClass,
    sample_rate: jint,
) -> jint {
    info!("nativeStart called with sample_rate={}", sample_rate);
    
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => {
            info!("Tokio runtime created successfully");
            rt
        }
        Err(e) => {
            error!("Failed to create tokio runtime: {:?}", e);
            return -1;
        }
    };

    // Get handle before moving runtime to background thread
    let rt_handle = runtime.handle().clone();
    
    // Channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    
    // Move runtime to background thread that DRIVES async tasks
    std::thread::Builder::new()
        .name("tokio-runtime".to_string())
        .spawn(move || {
            info!("Runtime background thread started, driving async tasks");
            // block_on DRIVES the runtime - spawned tasks will execute!
            runtime.block_on(async {
                // Wait until shutdown_tx is dropped or sends
                let _ = shutdown_rx.await;
            });
            info!("Runtime background thread stopping");
        })
        .expect("failed to spawn runtime thread");

    let config = AecConfig {
        sample_rate: sample_rate as u32,
        channels: Channels::Mono,
    };

    info!("Creating CaptureHandle...");
    let handle = match rt_handle.block_on(async { CaptureHandle::new(config) }) {
        Ok(h) => {
            info!("CaptureHandle created successfully, sample_rate={}", h.native_sample_rate());
            h
        }
        Err(e) => {
            error!("Failed to create CaptureHandle: {:?}", e);
            return -2;
        }
    };

    let mut guard = CAPTURE.lock().unwrap();
    *guard = Some(AndroidCaptureHandle { handle, shutdown_tx: Some(shutdown_tx) });

    0
}

#[no_mangle]
pub extern "system" fn Java_com_example_sysvoice_VoiceCapture_nativeRecv<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    buffer: JFloatArray<'local>,
) -> jint {
    let guard = CAPTURE.lock().unwrap();
    let capture = match guard.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    match capture.handle.try_recv() {
        Some(Ok(samples)) => {
            let buffer_len = match env.get_array_length(&buffer) {
                Ok(len) => len as usize,
                Err(_) => return -3,
            };

            let copy_len = samples.len().min(buffer_len);
            let floats: Vec<jfloat> = samples.iter().map(|&s| s as jfloat).collect();

            if env
                .set_float_array_region(&buffer, 0, &floats[..copy_len])
                .is_err()
            {
                return -4;
            }

            copy_len as jint
        }
        Some(Err(_)) => -2,
        None => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_sysvoice_VoiceCapture_nativeGetSampleRate(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let guard = CAPTURE.lock().unwrap();
    match guard.as_ref() {
        Some(c) => c.handle.native_sample_rate() as jint,
        None => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_sysvoice_VoiceCapture_nativeStop(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut guard = CAPTURE.lock().unwrap();
    *guard = None;
}
