//! Horizon supervisor calls: arguments in x0-x7, the result code back in w0 and outputs from x1 on.
use crate::cmif;
use core::arch::asm;
use core::ffi::CStr;

pub type Result<T> = core::result::Result<T, u32>;

unsafe fn call<const ID: u16>(x0: u64, x1: u64, x2: u64, x3: u64) -> Result<u64> {
    let (code, out): (u64, u64);
    unsafe {
        asm!("svc {id}", id = const ID,
            inlateout("x0") x0 => code, inlateout("x1") x1 => out, inlateout("x2") x2 => _, inlateout("x3") x3 => _,
            lateout("x4") _, lateout("x5") _, lateout("x6") _, lateout("x7") _, options(nostack));
    }
    if code as u32 == 0 { Ok(out) } else { Err(code as u32) }
}

pub fn set_heap_size(size: u64) -> Result<u64> {
    unsafe { call::<0x01>(0, size, 0, 0) }
}

pub fn exit_process() -> ! {
    unsafe { asm!("svc 0x07", options(noreturn, nostack)) }
}

pub fn map_shared_memory(handle: u32, address: u64, size: u64, permission: u32) -> Result<()> {
    unsafe { call::<0x13>(u64::from(handle), address, size, u64::from(permission)) }.map(drop)
}

pub fn create_transfer_memory(address: u64, size: u64, permission: u32) -> Result<u32> {
    unsafe { call::<0x15>(0, address, size, u64::from(permission)) }.map(|handle| handle as u32)
}

pub fn close_handle(handle: u32) -> Result<()> {
    unsafe { call::<0x16>(u64::from(handle), 0, 0, 0) }.map(drop)
}

pub fn connect_to_named_port(name: &CStr) -> Result<u32> {
    unsafe { call::<0x1f>(0, name.as_ptr() as u64, 0, 0) }.map(|handle| handle as u32)
}

/// Sends the CMIF message in `ipc_buffer` and leaves the reply there.
pub fn send_sync_request(session: u32) -> Result<()> {
    unsafe { call::<0x21>(u64::from(session), 0, 0, 0) }.map(drop)
}

/// Sends `request` and returns its reply, once the reply's result is zero.
pub fn request(session: u32, request: &cmif::Request) -> Result<cmif::Reply<'static>> {
    let ipc = unsafe { &mut *ipc_buffer() };
    cmif::write_request(ipc, request);
    send_sync_request(session)?;
    let reply = cmif::read_reply(ipc).ok_or(u32::MAX)?;
    if reply.result != 0 { Err(reply.result) } else { Ok(reply) }
}

pub fn output_debug_string(text: &str) -> Result<()> {
    unsafe { call::<0x27>(text.as_ptr() as u64, text.len() as u64, 0, 0) }.map(drop)
}

pub fn get_info(id: u32, handle: u32, sub_id: u64) -> Result<u64> {
    unsafe { call::<0x29>(0, u64::from(id), u64::from(handle), sub_id) }
}

pub fn map_physical_memory(address: u64, size: u64) -> Result<()> {
    unsafe { call::<0x2c>(address, size, 0, 0) }.map(drop)
}

/// The thread's IPC message buffer: the first 0x100 bytes of its TLS, whose base the kernel keeps in TPIDRRO_EL0.
pub fn ipc_buffer() -> *mut [u32; 64] {
    let tls: u64;
    unsafe { asm!("mrs {}, tpidrro_el0", out(reg) tls, options(nomem, nostack, preserves_flags)) };
    tls as *mut [u32; 64]
}
