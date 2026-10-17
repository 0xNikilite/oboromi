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
    use crate::input::InputState;
    use crate::kernel::Kernel;

    // guest.bin is what scripts/build-guest.sh builds from guest/.
    #[test]
    fn the_guest_image_initializes_applet_time_and_hid() {
        let cpu = CpuManager::new();
        let image = include_bytes!("../tests/fixtures/guest/guest.bin");
        let entry = load_flat_image(&cpu.ram, image, BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        kernel.hid.borrow_mut().input = || InputState { buttons: 1, left: [0.5, -0.5], right: [0.0; 2], connected: true };
        kernel.start(core, entry, image.len() as u64);
        assert!(kernel.run(core));
        assert_eq!(kernel.debug_output[0], "hello from guest");
        let time: i64 = kernel.debug_output[1].strip_prefix("focus 1 time ").unwrap().parse().unwrap();
        assert!((time - std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64).abs() < 5);
        assert_eq!(kernel.debug_output[2], "buttons 0x1 left 16383 -16383");
        println!("{}", kernel.debug_output[1]);
    }
}
