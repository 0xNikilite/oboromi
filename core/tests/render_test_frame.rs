use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::{VkError, frame::render_test_frame};

#[test]
fn test_frame_reads_back_as_gradient() {
    *oboromi_core::gpu::SPEED_HACKS.lock().unwrap() = oboromi_core::gpu::SpeedHacks::none();
    match render_test_frame() {
        Ok(()) => {}
        Err(e @ (VkError::Loader | VkError::NoDevice)) => return eprintln!("skipping: {e:?}"),
        Err(e) => panic!("test frame failed: {e:?}"),
    }
    let (mut width, mut height) = (0, 0);
    let len = unsafe { oboromi_frame_read(std::ptr::null_mut(), 0, &mut width, &mut height) };
    let mut pixels = vec![0; len];
    unsafe { oboromi_frame_read(pixels.as_mut_ptr(), len, &mut width, &mut height) };
    assert_eq!(pixels[..4], [0, 0, 128, 255]);
    let last = &pixels[len - 4..];
    assert!(last[0] >= 254 && last[1] >= 254, "{last:?}");
}
