//! Flat AArch64 guest images: the bytes go to guest RAM unchanged and execution starts at their first byte.
use crate::cpu::GuestRam;

/// The address guest/link.ld links the image for.
pub const BASE: u64 = 0x0800_0000;

/// Places `image` at `base` in `ram`, returning its entry point, or None when it does not fit.
pub fn load_flat_image(ram: &GuestRam, image: &[u8], base: u64) -> Option<u64> {
    load_segments(ram, base, &[(0, image)]).map(|()| base)
}

/// Writes each (offset, bytes) segment at `base + offset` in `ram`, or nothing and None when one does not fit.
pub fn load_segments(ram: &GuestRam, base: u64, segments: &[(u64, &[u8])]) -> Option<()> {
    let end = |&(offset, bytes): &(u64, &[u8])| base.checked_add(offset)?.checked_add(bytes.len() as u64);
    segments.iter().all(|s| end(s).is_some_and(|end| end <= ram.size() as u64)).then(|| {
        for &(offset, bytes) in segments {
            ram.write(base + offset, bytes);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::cpu_manager::CpuManager;
    use crate::gpu::engine::Gpu;
    use crate::input::InputState;
    use crate::kernel::Kernel;
    use crate::nn::{hipc::Service, nvdrv::Nvdrv, sm::Sm, vi};
    use crate::tests::gpu::harness::vk_state;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // guest.bin is what scripts/build-guest.sh builds from guest/. nvdrv needs a GPU, which the kernel's own sm: lacks.
    #[test]
    fn the_guest_image_reads_the_pad_draws_a_triangle_and_presents_three_frames() {
        let Some(state) = vk_state() else { return };
        let cpu = CpuManager::new();
        let image = include_bytes!("../tests/fixtures/guest/guest.bin");
        let entry = load_flat_image(&cpu.ram, image, BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        kernel.hid.borrow_mut().input = || InputState { buttons: 1, left: [0.5, -0.5], right: [0.0; 2], connected: true };
        let nvdrv = Rc::new(RefCell::new(Nvdrv::new(Gpu::new(Box::leak(Box::new(state)), cpu.ram.clone()))));
        let mut services = HashMap::from([
            ("hid".into(), kernel.hid.clone() as Rc<RefCell<dyn Service>>),
            ("nvdrv".into(), nvdrv.clone() as Rc<RefCell<dyn Service>>),
        ]);
        services.extend(vi::services(nvdrv));
        kernel.ports.insert("sm:".into(), Rc::new(RefCell::new(Sm(services))));
        kernel.start(core, entry, image.len() as u64);
        assert!(kernel.run(core));
        assert_eq!(kernel.debug_output, ["hello from guest", "buttons 0x1 left 16383 -16383", "pixel ff8000ff", "slots 0 1 0"]);
        let frame = crate::display::DISPLAY.lock().unwrap();
        assert_eq!((frame.width, frame.height), (128, 64));
        assert_eq!(&frame.pixels[(20 * 128 + 20) * 4..][..4], &[40, 80, 192, 255]);
    }
}
