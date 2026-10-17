//! A homebrew NRO (switchbrew NRO, Homebrew_ABI): prints its argv and applet type from the loader config, then writes
//! /hello.txt on the SD card through fsp-srv the way libnx's fs.c does.
#![no_std]
#![no_main]

use core::ffi::CStr;
use guest::cmif::{self, Buffer, Request};
use guest::svc::{self, Result};

// The kernel set sp, so _start only has to skip the NRO header and keep x0 (the loader config) for main.
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

/// Sends `request` and returns the reply's first handle, 0 without one.
fn call(session: u32, request: Request) -> Result<u32> {
    let ipc = unsafe { &mut *svc::ipc_buffer() };
    cmif::write_request(ipc, &request);
    svc::send_sync_request(session)?;
    let reply = cmif::read_reply(ipc).ok_or(u32::MAX)?;
    if reply.result != 0 {
        return Err(reply.result);
    }
    Ok(reply.move_handles.first().copied().unwrap_or(0))
}

fn write_sd_file(name: &[u8], bytes: &[u8]) -> Result<()> {
    let mut path = [0u8; 0x301];
    path[..name.len()].copy_from_slice(name);
    let path = [Buffer { address: path.as_ptr() as u64, size: path.len() as u64 }];
    let fsp = svc::connect_to_named_port(c"fsp-srv")?;
    call(fsp, Request { command: 1, send_pid: true, data: &[0, 0], ..Default::default() })?;
    let sd = call(fsp, Request { command: 18, ..Default::default() })?;
    call(sd, Request { command: 0, pointer: &path, data: &[0, 0, 0, 0], ..Default::default() })?;
    let file = call(sd, Request { command: 8, pointer: &path, data: &[2, 0], ..Default::default() })?;
    let send = [Buffer { address: bytes.as_ptr() as u64, size: bytes.len() as u64 }];
    call(file, Request { command: 1, send: &send, data: &[0, 0, 0, 0, bytes.len() as u32, 0], ..Default::default() })?;
    svc::close_handle(file)
}

#[unsafe(no_mangle)]
extern "C" fn main(config: *const [u64; 3]) -> ! {
    for entry in (0..).map(|i| unsafe { *config.add(i) }).take_while(|entry| entry[0] as u32 != 0) {
        match entry[0] as u32 {
            5 => _ = svc::output_debug_string(unsafe { CStr::from_ptr(entry[2] as *const _) }.to_str().unwrap_or_default()),
            7 => {
                let mut text = *b"applet type ?";
                text[12] = b'0' + entry[1] as u8;
                let _ = svc::output_debug_string(core::str::from_utf8(&text).unwrap_or_default());
            }
            _ => {}
        }
    }
    let _ = write_sd_file(b"/hello.txt", b"hello from the sd card");
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    svc::exit_process()
}
