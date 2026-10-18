#![no_std]
#![no_main]

use guest::{display, nv, svc};

core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "b main",
    ".word 0, 0, 0",
    ".ascii \"NRO0\"",
    ".word 0, __bss - _start, 0",
    ".word 0, __ro - _start, __ro - _start, __ro_size, __data - _start, __data_size",
    ".word __bss_size, 0",
    ".space 0x40",
);

fn draw() -> svc::Result<()> {
    let size = [1920, 1080];
    let heap = svc::set_heap_size(0x100_0000)?;
    let matrix = [size[1] as f32 / size[0] as f32, -0.0, 0.0, 1.0];
    let (nv, nvmap, target, _) = nv::triangle(heap, size, &scene::TRIANGLE_VERTICES, matrix)?;
    display::present(&nv, nvmap, target, size)
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    if draw().is_err() {
        let _ = svc::output_debug_string("triangle failed");
    }
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    let _ = svc::output_debug_string("triangle panicked");
    svc::exit_process()
}
