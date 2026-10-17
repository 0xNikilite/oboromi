//! The nvdrv client libnx's nvInitialize, nvOpen, nvIoctl and nvQueryEvent are: one session to the nvdrv service,
//! whose replies carry an NvError word that turns into the `Err` here.
use crate::cmif::{self, Buffer, Request};
use crate::svc;
use core::mem::size_of;

const CURRENT_PROCESS: u32 = 0xFFFF_8001;
const NO_BUFFER: Buffer = Buffer { address: 0, size: 0 };

pub struct Nv(u32);

impl Nv {
    /// Connects and initializes the driver with `size` bytes of transfer memory at `address`.
    pub fn initialize(address: u64, size: u64) -> svc::Result<Self> {
        let nv = Self(svc::connect_to_named_port(c"nvdrv")?);
        let memory = svc::create_transfer_memory(address, size, 0)?;
        nv.call(&Request { command: 3, copy_handles: &[CURRENT_PROCESS, memory], data: &[size as u32], ..Default::default() }, 0)?;
        nv.call(&Request { command: 8, send_pid: true, data: &[0, 0], ..Default::default() }, 0)?;
        Ok(nv)
    }

    /// The reply's data words and first copied handle, once word `error` of the data is zero.
    fn call(&self, request: &Request, error: usize) -> svc::Result<([u32; 2], u32)> {
        let ipc = unsafe { &mut *svc::ipc_buffer() };
        cmif::write_request(ipc, request);
        svc::send_sync_request(self.0)?;
        let reply = cmif::read_reply(ipc).ok_or(u32::MAX)?;
        match (reply.result, reply.data[error]) {
            (0, 0) => Ok(([reply.data[0], reply.data[1]], reply.copy_handles.first().copied().unwrap_or(0))),
            (0, e) | (e, _) => Err(e),
        }
    }

    pub fn open(&self, path: &str) -> svc::Result<u32> {
        let path = Buffer { address: path.as_ptr() as u64, size: path.len() as u64 };
        self.call(&Request { command: 0, send: &[path], ..Default::default() }, 1).map(|([fd, _], _)| fd)
    }

    pub fn query_event(&self, fd: u32, id: u32) -> svc::Result<u32> {
        self.call(&Request { command: 4, data: &[fd, id], ..Default::default() }, 0).map(|(_, event)| event)
    }

    pub fn ioctl<T>(&self, fd: u32, code: u32, arg: &mut T) -> svc::Result<()> {
        self.ioctl_with(1, fd, code, arg, NO_BUFFER)
    }

    pub fn ioctl2<T>(&self, fd: u32, code: u32, arg: &mut T, input: &[u8]) -> svc::Result<()> {
        self.ioctl_with(11, fd, code, arg, Buffer { address: input.as_ptr() as u64, size: input.len() as u64 })
    }

    pub fn ioctl3<T>(&self, fd: u32, code: u32, arg: &mut T, output: &mut [u8]) -> svc::Result<()> {
        self.ioctl_with(12, fd, code, arg, Buffer { address: output.as_mut_ptr() as u64, size: output.len() as u64 })
    }

    /// The argument goes in when the ioctl's in bit is set and comes back when its out bit is.
    fn ioctl_with<T>(&self, command: u32, fd: u32, code: u32, arg: &mut T, extra: Buffer) -> svc::Result<()> {
        let address = core::ptr::from_mut(arg) as u64;
        let arg = |bit: u32| Buffer { address, size: u64::from(code >> bit & 1) * size_of::<T>() as u64 };
        let buffers = [arg(30), extra, arg(31), extra];
        let (send, receive) = match command {
            11 => (&buffers[..2], &buffers[2..3]),
            12 => (&buffers[..1], &buffers[2..]),
            _ => (&buffers[..1], &buffers[2..3]),
        };
        self.call(&Request { command, send, receive, data: &[fd, code], ..Default::default() }, 0).map(drop)
    }
}
