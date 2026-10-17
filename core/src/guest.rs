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
    use crate::kernel::Kernel;
    use crate::nn::nvdrv::Nvdrv;
    use crate::tests::gpu::harness::vk_state;
    use std::cell::RefCell;
    use std::rc::Rc;

    // guest.bin is what scripts/build-guest.sh builds from guest/. The port stands in for sm:'s nvdrv.
    #[test]
    fn the_guest_image_draws_its_triangle_through_nvdrv() {
        let Some(state) = vk_state() else { return };
        let cpu = CpuManager::new();
        let image = include_bytes!("../tests/fixtures/guest/guest.bin");
        let entry = load_flat_image(&cpu.ram, image, BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        let gpu = Gpu::new(Box::leak(Box::new(state)), cpu.ram.clone());
        kernel.ports.insert("nvdrv".into(), Rc::new(RefCell::new(Nvdrv::new(gpu))));
        kernel.start(core, entry, image.len() as u64);
        assert!(kernel.run(core));
        assert_eq!(kernel.debug_output, ["pixel ff8000ff"]);
    }
}
