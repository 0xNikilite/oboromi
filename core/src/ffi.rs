//! ffi bridge for the qt/c++ gui. panics are caught and forwarded to the
//! log callback to avoid unwinding across the c boundary.

use crate::display::{CALLBACK, DISPLAY, FrameCallback, attach_surface};
use ash::vk;
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

/// draws the pbr BoomBox demo at `seconds` and presents it, false if vulkan is
/// missing or the draw fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_boombox_demo(seconds: f32) -> bool {
    matches!(catch_unwind(|| crate::gpu::demo::render_boombox_frame(seconds)), Ok(Ok(())))
}

/// draws the pbr FlightHelmet demo like the BoomBox one, false too when
/// demos/assets/fetch.sh has not fetched the asset.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_helmet_demo(seconds: f32) -> bool {
    matches!(catch_unwind(|| crate::gpu::demo::render_helmet_frame(seconds)), Ok(Ok(())))
}

/// draws the post-effect demo at `seconds` and presents it, false if vulkan
/// is missing or a draw fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_post_frame(seconds: f32) -> bool {
    matches!(
        catch_unwind(|| crate::gpu::demo_post::render_post_frame(seconds)),
        Ok(Ok(()))
    )
}

/// draws a frame of the particles demo at `seconds` and presents it, false if
/// vulkan is missing or the frame fails. blocks, so call it from a worker thread.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_render_particles_frame(seconds: f32) -> bool {
    matches!(
        catch_unwind(|| crate::gpu::demo_particles::render_particles_frame(seconds)),
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
    let mut frame = DISPLAY.lock().unwrap();
    unsafe {
        *width = frame.width;
        *height = frame.height;
    }
    let pixels = frame.pixels();
    let len = pixels.len();
    if !dst.is_null() && len <= cap {
        unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), dst, len) };
    }
    len
}

/// `callback` runs on the presenting thread after every frame, null clears it.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_frame_set_callback(callback: FrameCallback, user: *mut c_void) {
    *CALLBACK.lock().unwrap() = (callback, user as usize);
}

/// creates the vulkan device with window surface support and returns its VkInstance for the gui to
/// create surfaces with, null if vulkan or its surface extensions are missing.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_vk_instance() -> vk::Instance {
    match crate::gpu::surface_device() {
        Ok(state) if state.surface.is_some() => state.instance.handle(),
        _ => vk::Instance::null(),
    }
}

/// presents every frame into `surface`, a VkSurfaceKHR of that instance, sized `width` x `height`
/// pixels. call it again on resize and expose, and with a null surface before destroying it.
/// false if the device cannot present to it.
#[unsafe(no_mangle)]
pub extern "C" fn oboromi_display_surface(surface: vk::SurfaceKHR, width: u32, height: u32) -> bool {
    matches!(
        catch_unwind(|| attach_surface(surface, width, height)),
        Ok(Ok(()))
    )
}
