//! Applet and time startup calls matching libnx nx/source/services/{applet,time}.c.

use crate::cmif::{self, Request};
use crate::svc::{self, Result};

const CURRENT_PROCESS: u32 = 0xffff_8001;

fn call(session: u32, request: Request) -> Result<(u32, [u32; 8])> {
    let ipc = unsafe { &mut *svc::ipc_buffer() };
    cmif::write_request(ipc, &request);
    svc::send_sync_request(session)?;
    let reply = cmif::read_reply(ipc).ok_or(u32::MAX)?;
    if reply.result != 0 {
        return Err(reply.result);
    }
    let handle = reply.copy_handles.iter().chain(reply.move_handles).next().copied().unwrap_or(0);
    let mut data = [0; 8];
    data.copy_from_slice(&reply.data[..8]);
    Ok((handle, data))
}

fn service(sm: u32, name: [u32; 2]) -> Result<u32> {
    call(sm, Request { command: 1, data: &name, ..Default::default() }).map(|reply| reply.0)
}

pub fn initialize() -> Result<(u32, i64)> {
    let sm = svc::connect_to_named_port(c"sm:")?;
    call(sm, Request { command: 0, send_pid: true, data: &[0, 0], ..Default::default() })?;
    let applet = service(sm, [u32::from_le_bytes(*b"appl"), u32::from_le_bytes(*b"etOE")])?;
    let proxy = call(applet, Request { command: 0, send_pid: true, copy_handles: &[CURRENT_PROCESS], data: &[0, 0], ..Default::default() })?.0;
    let common = call(proxy, Request { command: 0, ..Default::default() })?.0;
    let mut focus_message = false;
    loop {
        match call(common, Request { command: 1, ..Default::default() }) {
            Ok((_, data)) => focus_message |= data[0] == 15,
            Err(0x680) => break,
            Err(error) => return Err(error),
        }
    }
    if !focus_message {
        return Err(u32::MAX);
    }
    let focus = call(common, Request { command: 9, ..Default::default() })?.1[0];
    let time = service(sm, [u32::from_le_bytes(*b"time"), u32::from_le_bytes(*b":u\0\0")])?;
    let clock = call(time, Request { command: 0, ..Default::default() })?.0;
    let data = call(clock, Request { command: 0, ..Default::default() })?.1;
    Ok((focus, (u64::from(data[0]) | u64::from(data[1]) << 32) as i64))
}
