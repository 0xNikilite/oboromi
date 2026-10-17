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

    // guest.bin is what scripts/build-guest.sh builds from guest/. Without SVC dispatch Unicorn stops at the first svc.
    #[test]
    fn the_guest_image_reaches_its_debug_string_svc() {
        let cpu = CpuManager::new();
        let entry = load_flat_image(&cpu.ram, include_bytes!("../tests/fixtures/guest/guest.bin"), BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        core.set_pc(entry);
        core.run();
        let mut text = vec![0; core.get_x(1) as usize];
        cpu.ram.read(core.get_x(0), &mut text);
        assert_eq!(text, b"hello from guest");
    }
}
