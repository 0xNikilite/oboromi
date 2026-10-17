#![no_std]
#![no_main]

use core::fmt::Write;
use guest::{am_time, hid, svc};

core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "adrp x9, __stack_top",
    "mov sp, x9",
    "bl main",
    "b .",
);

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

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    let _ = svc::output_debug_string("hello from guest");
    let mut text = Text { bytes: [0; 64], len: 0 };
    let _ = match am_time::initialize() {
        Ok((focus, time)) => write!(text, "focus {focus} time {time}"),
        Err(code) => write!(text, "am/time error {code:#x}"),
    };
    let _ = svc::output_debug_string(core::str::from_utf8(&text.bytes[..text.len]).unwrap_or_default());
    text.len = 0;
    let _ = match hid::read_full_key() {
        Ok(pad) => write!(text, "buttons {:#x} left {} {}", pad.buttons, pad.left[0], pad.left[1]),
        Err(code) => write!(text, "hid error {code:#x}"),
    };
    let _ = svc::output_debug_string(core::str::from_utf8(&text.bytes[..text.len]).unwrap_or_default());
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    svc::exit_process()
}
