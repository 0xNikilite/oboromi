//! ffi bridge for the qt/c++ gui. panics are caught and forwarded to the
//! log callback to avoid unwinding across the c boundary.

use crate::display::{CALLBACK, DISPLAY, FrameCallback};
use std::ffi::{CString, c_void};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// called once per output line. the pointer is only valid during the call.
type LineCallback = extern "C" fn(*const c_char);

fn emit(cb: LineCallback, line: &str) {
    // silently drop lines containing nul bytes, they can't be passed to c.
    if let Ok(c) = CString::new(line) {
        cb(c.as_ptr());
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// runs the arm64 cpu test suite. blocks the calling thread, so call it from
/// a worker thread on the gui side, never from the ui thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_run_cpu_tests(cb: LineCallback) {
    match catch_unwind(AssertUnwindSafe(crate::tests::run::run_tests)) {
        Ok(lines) => {
            for line in lines {
                emit(cb, &line);
            }
        }
        Err(payload) => {
            emit(
                cb,
                &format!("fatal: cpu test runner panicked: {}", panic_message(&payload)),
            );
        }
    }
}

/// runs the sm86 gpu decoder/translation test suite. same threading rules
/// as "oboromi_run_cpu_tests"
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_run_gpu_tests(cb: LineCallback) {
    match catch_unwind(AssertUnwindSafe(crate::tests::run_gpu_tests)) {
        Ok(lines) => {
            for line in lines {
                emit(cb, &line);
            }
        }
        Err(payload) => {
            emit(
                cb,
                &format!("fatal: gpu test runner panicked: {}", panic_message(&payload)),
            );
        }
    }
}

/// generation of the last presented frame, 0 until the guest presents one.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_frame_generation() -> u64 {
    DISPLAY.lock().unwrap().generation
}

/// writes the frame size and copies its pixels into `dst` if `cap` is large
/// enough. returns the number of bytes the frame needs. pixels are linear
/// rgba8, stride `width * 4`.
///
/// # Safety
/// `dst` must be null or valid for `cap` bytes, `width` and `height` must be
/// valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oboromi_frame_read(
    dst: *mut u8,
    cap: usize,
    width: *mut u32,
    height: *mut u32,
) -> usize {
    let frame = DISPLAY.lock().unwrap();
    let len = frame.pixels.len();
    unsafe {
        if !dst.is_null() && len <= cap {
            std::ptr::copy_nonoverlapping(frame.pixels.as_ptr(), dst, len);
        }
        *width = frame.width;
        *height = frame.height;
    }
    len
}

/// `callback` runs on the presenting thread after every frame, null clears it.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_frame_set_callback(callback: FrameCallback, user: *mut c_void) {
    *CALLBACK.lock().unwrap() = (callback, user as usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presented_frame_reads_back_through_ffi() {
        let generation = oboromi_frame_generation();
        let pixels: Vec<u8> = (0..16).collect();
        crate::display::present(2, 2, &pixels);
        assert_eq!(oboromi_frame_generation(), generation + 1);

        let (mut width, mut height) = (0, 0);
        let mut dst = vec![0u8; 16];
        let len = unsafe { oboromi_frame_read(dst.as_mut_ptr(), 16, &mut width, &mut height) };
        assert_eq!((len, width, height), (16, 2, 2));
        assert_eq!(dst, pixels);
    }
}
