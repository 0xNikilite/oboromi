#![no_std]
#![no_main]

use guest::svc;

core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "adrp x9, __stack_top",
    "mov sp, x9",
    "bl main",
    "b .",
);

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    let _ = svc::output_debug_string("hello from guest");
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    svc::exit_process()
}
