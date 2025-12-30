//! C FFI wrapper for sys-voice iOS/Swift integration.
//!
//! This crate provides C-compatible bindings for the Swift iOS example.
//! Rust users should use sys-voice directly.

use std::ffi::c_void;
use sys_voice::{AecConfig, CaptureHandle, Channels};

struct FfiCaptureHandle {
    handle: CaptureHandle,
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
}

/// Start audio capture. Returns opaque handle or null on failure.
#[no_mangle]
pub extern "C" fn capture_start(sample_rate: u32) -> *mut c_void {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let config = AecConfig {
        sample_rate,
        channels: Channels::Mono,
    };

    let handle = match runtime.block_on(async { CaptureHandle::new(config) }) {
        Ok(h) => h,
        Err(_) => return std::ptr::null_mut(),
    };

    let ffi_handle = Box::new(FfiCaptureHandle { handle, runtime });
    Box::into_raw(ffi_handle) as *mut c_void
}

/// Receive audio samples. Returns sample count, 0 if none, -1 on error, -2 on capture error.
#[no_mangle]
pub extern "C" fn capture_recv(handle: *mut c_void, buffer: *mut f32, buffer_len: usize) -> i32 {
    if handle.is_null() || buffer.is_null() {
        return -1;
    }

    let ffi_handle = unsafe { &*(handle as *mut FfiCaptureHandle) };

    match ffi_handle.handle.try_recv() {
        Some(Ok(samples)) => {
            let copy_len = samples.len().min(buffer_len);
            unsafe {
                std::ptr::copy_nonoverlapping(samples.as_ptr(), buffer, copy_len);
            }
            copy_len as i32
        }
        Some(Err(_)) => -2,
        None => 0,
    }
}

/// Get the sample rate of the capture handle.
#[no_mangle]
pub extern "C" fn capture_sample_rate(handle: *mut c_void) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let ffi_handle = unsafe { &*(handle as *mut FfiCaptureHandle) };
    ffi_handle.handle.native_sample_rate()
}

/// Stop capture and release all resources.
#[no_mangle]
pub extern "C" fn capture_stop(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut FfiCaptureHandle);
        }
    }
}
