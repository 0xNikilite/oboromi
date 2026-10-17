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

/// renders a gradient on the gpu and presents it, false if vulkan is missing
/// or the dispatch fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_test_frame() -> bool {
    matches!(
        catch_unwind(crate::gpu::frame::render_test_frame),
        Ok(Ok(()))
    )
}

/// renders the julia set demo at `seconds` and presents it, false if vulkan
/// is missing or the dispatch fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_demo_frame(seconds: f32) -> bool {
    matches!(
        catch_unwind(|| crate::gpu::frame::render_demo_frame(seconds)),
        Ok(Ok(()))
    )
}

/// draws the pushbuffer triangle demo at `seconds` and presents it, false if
/// vulkan is missing or the draw fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_triangle_demo(seconds: f32) -> bool {
    matches!(
        catch_unwind(|| crate::gpu::frame::render_triangle_demo(seconds)),
        Ok(Ok(()))
    )
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

/// copies up to `frames` queued stereo frames into `dst` and zero-fills the
/// rest, returns the frames copied.
///
/// # Safety
/// `dst` must be null or valid for `frames * 2` writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oboromi_audio_pull(dst: *mut i16, frames: usize) -> usize {
    if dst.is_null() {
        return 0;
    }
    crate::audio::pull(unsafe { std::slice::from_raw_parts_mut(dst, frames * 2) })
}

/// writes the output format, interleaved i16 samples produced `period` frames at a time.
///
/// # Safety
/// `rate`, `channels` and `period` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oboromi_audio_format(rate: *mut u32, channels: *mut u32, period: *mut u32) {
    unsafe {
        *rate = crate::audio::RATE;
        *channels = crate::audio::CHANNELS;
        *period = crate::audio::PERIOD;
    }
}

/// writes the frames waiting in the ring and the pulls that ran dry.
///
/// # Safety
/// `queued` and `underruns` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oboromi_audio_stats(queued: *mut usize, underruns: *mut u64) {
    unsafe {
        *queued = crate::audio::queued();
        *underruns = crate::audio::underruns();
    }
}

/// frames ever queued, changes whenever something is produced.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_audio_generation() -> u64 {
    crate::audio::generation()
}

/// queues `seconds` more of a 440 hz test tone, returns the frames that fit.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_audio_test_tone(seconds: f32) -> usize {
    crate::audio::test_tone(seconds)
}
