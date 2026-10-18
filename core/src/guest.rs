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
    use crate::nn::{hipc::Service, nvdrv::Nvdrv, vi};
    use crate::tests::gpu::harness::vk_state;
    use std::cell::RefCell;
    use std::rc::Rc;

    static DISPLAY_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // guest.bin is what scripts/build-guest.sh builds from guest/. nvdrv needs a GPU, which the kernel's own sm: lacks.
    #[test]
    fn the_guest_image_initializes_services_draws_and_presents() {
        let _display = DISPLAY_TEST.lock().unwrap();
        let Some(state) = vk_state() else { return };
        let cpu = CpuManager::new();
        let image = include_bytes!("../tests/fixtures/guest/guest.bin");
        let entry = load_flat_image(&cpu.ram, image, BASE).unwrap();
        let core = cpu.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        kernel.hid.borrow_mut().input = || InputState { buttons: 1, left: [0.5, -0.5], right: [0.0; 2], connected: true };
        let nvdrv = Rc::new(RefCell::new(Nvdrv::new(Gpu::new(Box::leak(Box::new(state)), cpu.ram.clone()))));
        kernel.sm.borrow_mut().0.insert("nvdrv".into(), nvdrv.clone() as Rc<RefCell<dyn Service>>);
        kernel.sm.borrow_mut().0.extend(vi::services(nvdrv));
        kernel.start(core, entry, image.len() as u64);
        assert!(kernel.run(core));
        assert_eq!(kernel.debug_output[0], "hello from guest");
        let time: i64 = kernel.debug_output[1].strip_prefix("focus 1 time ").unwrap().parse().unwrap();
        assert!((time - std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64).abs() < 5);
        println!("{}", kernel.debug_output[1]);
        assert_eq!(&kernel.debug_output[2..], ["buttons 0x1 left 16383 -16383", "pixel ff8000ff", "slots 0 1 0"]);
        let frame = crate::display::DISPLAY.lock().unwrap();
        assert_eq!((frame.width, frame.height), (128, 64));
        assert_eq!(&frame.pixels[(20 * 128 + 20) * 4..][..4], &[40, 80, 192, 255]);
    }

    #[test]
    fn guest_triangle_end_to_end() {
        let _display = DISPLAY_TEST.lock().unwrap();
        let state = crate::gpu::device().expect("guest triangle requires Vulkan");
        crate::gpu::frame::render_triangle_demo(0.0).unwrap();
        let (generation, expected) = {
            let frame = crate::display::DISPLAY.lock().unwrap();
            assert_eq!((frame.width, frame.height), (1920, 1080));
            (frame.generation, frame.pixels.clone())
        };
        let dir = std::env::temp_dir().join(format!("oboromi-triangle-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("triangle.nro");
        std::fs::write(&path, include_bytes!("../tests/fixtures/guest/triangle.nro")).unwrap();
        let cpus = CpuManager::new();
        let cpu = cpus.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        let nvdrv = Rc::new(RefCell::new(Nvdrv::new(Gpu::new(state, cpus.ram.clone()))));
        kernel.sm.borrow_mut().0.insert("nvdrv".into(), nvdrv.clone() as Rc<RefCell<dyn Service>>);
        kernel.sm.borrow_mut().0.extend(vi::services(nvdrv));
        crate::loader::nro::load(&cpus.ram, cpu, &mut kernel, &path).unwrap();
        assert!(!kernel.run_until(cpu, || crate::display::DISPLAY.lock().unwrap().generation > generation), "{:?}", kernel.debug_output);
        {
            let frame = crate::display::DISPLAY.lock().unwrap();
            assert_eq!(frame.generation, generation + 1);
            assert_eq!((frame.width, frame.height), (1920, 1080));
            let digest = |bytes: &[u8]| bytes.iter().fold(0xcbf29ce484222325u64, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3));
            assert_eq!(digest(&frame.pixels), digest(&expected));
            assert!(frame.pixels == expected);
            println!("guest and host triangle FNV-1a: {:016x}", digest(&frame.pixels));
        }
        assert!(kernel.run(cpu));
        assert!(kernel.debug_output.is_empty(), "{:?}", kernel.debug_output);
        std::fs::remove_dir_all(dir).unwrap();
    }

}
