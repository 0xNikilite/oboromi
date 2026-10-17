//! libnx's hid start-up (nx/source/services/hid.c, sm.c) and padUpdate's read of Npad No1's latest FullKey state.
use crate::cmif::{self, Buffer, Request};
use crate::svc::{self, Result};

/// No applet service hands out a resource user id yet.
const ARUID: [u32; 2] = [0, 0];
const SHARED_MEMORY: u64 = 0x1800_0000;
const FULL_KEY: u32 = 1;
const IS_CONNECTED: u32 = 1;

pub struct FullKey {
    pub buttons: u64,
    pub left: [i32; 2],
    pub right: [i32; 2],
}

/// Sends `request` and returns the reply's first handle, 0 without one.
fn call(session: u32, request: Request) -> Result<u32> {
    let ipc = unsafe { &mut *svc::ipc_buffer() };
    cmif::write_request(ipc, &request);
    svc::send_sync_request(session)?;
    let reply = cmif::read_reply(ipc).ok_or(u32::MAX)?;
    if reply.result != 0 {
        return Err(reply.result);
    }
    Ok(reply.copy_handles.iter().chain(reply.move_handles).next().copied().unwrap_or(0))
}

fn with_aruid(value: u32) -> [u32; 4] {
    [value, 0, ARUID[0], ARUID[1]]
}

/// A session to the service sm: has under `name`.
pub fn service(name: [u8; 8]) -> Result<u32> {
    let sm = svc::connect_to_named_port(c"sm:")?;
    call(sm, Request { command: 0, send_pid: true, data: &[0, 0], ..Default::default() })?;
    let name = [0, 4].map(|i| u32::from_le_bytes(name[i..i + 4].try_into().unwrap()));
    call(sm, Request { command: 1, data: &name, ..Default::default() })
}

pub fn read_full_key() -> Result<FullKey> {
    let hid = service(*b"hid\0\0\0\0\0")?;
    let resource = call(hid, Request { command: 0, send_pid: true, data: &ARUID, ..Default::default() })?;
    let shared = call(resource, Request { command: 0, ..Default::default() })?;
    svc::map_shared_memory(shared, SHARED_MEMORY, 0x40000, 1)?;
    call(hid, Request { command: 100, send_pid: true, data: &with_aruid(FULL_KEY), ..Default::default() })?;
    let ids = [0u32, 0x20];
    let pointer = [Buffer { address: ids.as_ptr() as u64, size: 8 }];
    call(hid, Request { command: 102, send_pid: true, data: &ARUID, pointer: &pointer, ..Default::default() })?;
    call(hid, Request { command: 109, send_pid: true, data: &with_aruid(5), ..Default::default() })?;

    let npad = SHARED_MEMORY + 0x9A00;
    let read = |at: u64| unsafe { core::ptr::read_volatile(at as *const u64) };
    let state = npad + 0x28 + 0x20 + read(npad + 0x28 + 0x10) * 0x30 + 8;
    let word = |at: u64| unsafe { core::ptr::read_volatile((state + at) as *const i32) };
    if read(npad) as u32 & FULL_KEY == 0 || word(0x20) as u32 & IS_CONNECTED == 0 {
        return Ok(FullKey { buttons: 0, left: [0; 2], right: [0; 2] });
    }
    Ok(FullKey { buttons: read(state + 8), left: [word(0x10), word(0x14)], right: [word(0x18), word(0x1c)] })
}
