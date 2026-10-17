//! Flat AArch64 guest images: the bytes go to guest RAM unchanged and execution starts at their first byte.
use crate::cpu::GuestRam;

/// The address guest/link.ld links the image for.
pub const BASE: u64 = 0x0800_0000;

/// Places `image` at `base` in `ram`, returning its entry point, or None when it does not fit.
pub fn load_flat_image(ram: &GuestRam, image: &[u8], base: u64) -> Option<u64> {
    let end = usize::try_from(base).ok()?.checked_add(image.len())?;
    (end <= ram.size()).then(|| {
        ram.write(base, image);
        base
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::cpu_manager::CpuManager;
    use crate::gpu::engine::Gpu;
    use crate::input::InputState;
    use crate::kernel::Kernel;
    use crate::nn::{nvdrv::Nvdrv, sm::Sm};
    use crate::tests::gpu::harness::vk_state;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // guest.bin is what scripts/build-guest.sh builds from guest/. nvdrv needs a GPU, which the kernel's own sm: lacks.
    #[test]
    fn the_guest_image_reads_the_host_pad_and_draws_its_triangle_through_nvdrv() {
        let Some(state) = vk_state() else { return };
        let cpu = CpuManager::new();
        let image = include_bytes!("../tests/fixtures/guest/guest.bin");
        let entry = load_flat_image(&cpu.ram, image, BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        kernel.hid.borrow_mut().input = || InputState { buttons: 1, left: [0.5, -0.5], right: [0.0; 2], connected: true };
        let nvdrv = Rc::new(RefCell::new(Nvdrv::new(Gpu::new(Box::leak(Box::new(state)), cpu.ram.clone()))));
        let services = HashMap::from([("hid".into(), kernel.hid.clone() as _), ("nvdrv".into(), nvdrv as _)]);
        kernel.ports.insert("sm:".into(), Rc::new(RefCell::new(Sm(services))));
        kernel.start(core, entry, image.len() as u64);
        assert!(kernel.run(core));
        assert_eq!(kernel.debug_output, ["hello from guest", "buttons 0x1 left 16383 -16383", "pixel ff8000ff"]);
    }
}
