//! ffi bridge for the qt/c++ gui. panics are caught and forwarded to the
//! log callback to avoid unwinding across the c boundary.

use std::ffi::CString;
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
                &format!(
                    "fatal: cpu test runner panicked: {}",
                    panic_message(&payload)
                ),
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
                &format!(
                    "fatal: gpu test runner panicked: {}",
                    panic_message(&payload)
                ),
            );
        }
    }
}

/// Runs one texture demo upload or readback.
/// # Safety
/// Input and output must each address len bytes for the duration of the call.
/// Output must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oboromi_texture_transfer(
    input: *const u8,
    output: *mut u8,
    len: usize,
    width: u32,
    height: u32,
    upload: bool,
) -> bool {
    if input.is_null() || output.is_null() || len > 512 * 512 * 4 {
        return false;
    }
    catch_unwind(|| {
        let bytes = unsafe { std::slice::from_raw_parts(input, len) };
        let Some(result) = crate::gpu::texture_demo::transfer(bytes, width, height, upload) else {
            return false;
        };
        unsafe {
            std::ptr::copy_nonoverlapping(result.as_ptr(), output, len);
        }
        true
    })
    .unwrap_or(false)
}
