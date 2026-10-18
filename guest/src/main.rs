#![no_std]
#![no_main]

use core::fmt::Write;
use guest::{am_time, display, hid, svc};
use scene::Vertex;

core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "adrp x9, __stack_top",
    "mov sp, x9",
    "bl main",
    "b .",
);

const SIZE: u32 = 64;
const ORANGE: [u8; 4] = [255, 128, 0, 255];
// Pixel corners (4, 4), (60, 4) and (4, 60), as the host triangle test draws them.
const TRIANGLE: [Vertex; 3] = [([-0.875, -0.875], ORANGE), ([0.875, -0.875], ORANGE), ([-0.875, 0.875], ORANGE)];

fn guest_slice<T>(address: u64, len: usize) -> &'static mut [T] {
    unsafe { core::slice::from_raw_parts_mut(address as *mut T, len) }
}

/// Draws the scene crate's triangle through nvdrv, then queues three software frames.
fn draw() -> svc::Result<([u8; 4], [u32; 3])> {
    let heap = svc::set_heap_size(0x40_0000)?;
    let (nv, nvmap, _, target) = guest::nv::triangle(heap, [SIZE; 2], &TRIANGLE, [1.0, 0.0, 0.0, 1.0])?;
    let pixel = guest_slice::<[u8; 4]>(target + u64::from(20 * SIZE + 20) * 4, 1)[0];
    Ok((pixel, display::show(&nv, nvmap, heap + 0x30_0000)?))
}

struct Text {
    bytes: [u8; 64],
    len: usize,
}

impl Write for Text {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

fn print(line: core::fmt::Arguments) {
    let mut text = Text { bytes: [0; 64], len: 0 };
    let _ = text.write_fmt(line);
    let _ = svc::output_debug_string(core::str::from_utf8(&text.bytes[..text.len]).unwrap_or_default());
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    print(format_args!("hello from guest"));
    match am_time::initialize() {
        Ok((focus, time)) => print(format_args!("focus {focus} time {time}")),
        Err(code) => print(format_args!("am/time error {code:#x}")),
    }
    match hid::read_full_key() {
        Ok(pad) => print(format_args!("buttons {:#x} left {} {}", pad.buttons, pad.left[0], pad.left[1])),
        Err(code) => print(format_args!("hid error {code:#x}")),
    }
    match draw() {
        Ok((rgba, slots)) => {
            print(format_args!("pixel {:08x}", u32::from_be_bytes(rgba)));
            print(format_args!("slots {} {} {}", slots[0], slots[1], slots[2]));
        }
        Err(code) => print(format_args!("nvdrv error {code:#x}")),
    }
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    svc::exit_process()
}
