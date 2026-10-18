//! The nvdrv client libnx's nvInitialize, nvOpen, nvIoctl and nvQueryEvent are: one session to the nvdrv service,
//! whose replies carry an NvError word that turns into the `Err` here.
use crate::cmif::{self, Buffer, Request};
use crate::svc;
use crate::nvdrv::*;
use scene::{Addresses, Scene};
use core::mem::size_of;

const CURRENT_PROCESS: u32 = 0xFFFF_8001;
const NO_BUFFER: Buffer = Buffer { address: 0, size: 0 };

pub struct Nv(u32);

impl Nv {
    /// Connects and initializes the driver with `size` bytes of transfer memory at `address`.
    pub fn initialize(address: u64, size: u64) -> svc::Result<Self> {
        let nv = Self(crate::hid::service(*b"nvdrv\0\0\0")?);
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

const BUFFER: u64 = 0x1_0000;

/// Allocates and maps guest memory, returning its GPU address and nvmap handle.
fn buffer(nv: &Nv, [nvmap, space]: [u32; 2], address: u64, size: u32) -> svc::Result<(u64, u32)> {
    let mut create = NvmapCreate { size, handle: 0 };
    nv.ioctl(nvmap, NVMAP_CREATE, &mut create)?;
    nv.ioctl(nvmap, NVMAP_ALLOC, &mut NvmapAlloc { handle: create.handle, heapmask: 0, flags: 1, align: 0x1000, kind: 0, pad: [0; 7], address })?;
    let mut reserved = AsAllocSpace { pages: size / BUFFER as u32, page_size: BUFFER as u32, flags: 0, pad: 0, offset_or_align: BUFFER };
    nv.ioctl(space, AS_ALLOC_SPACE, &mut reserved)?;
    let mut map = AsMapBufferEx { flags: 1, kind: 0, nvmap_handle: create.handle, page_size: BUFFER as u32, buffer_offset: 0, mapping_size: u64::from(size), offset: reserved.offset_or_align };
    nv.ioctl(space, AS_MAP_BUFFER_EX, &mut map)?;
    Ok((map.offset, create.handle))
}

fn guest_slice<T>(address: u64, len: usize) -> &'static mut [T] {
    unsafe { core::slice::from_raw_parts_mut(address as *mut T, len) }
}

pub fn triangle(heap: u64, size: [u32; 2], vertices: &[scene::Vertex; 3], matrix: [f32; 4]) -> svc::Result<(Nv, u32, u32, u64)> {
    let nv = Nv::initialize(heap, 0x10_0000)?;
    let mut fds = [0; 5];
    for (fd, path) in fds.iter_mut().zip(["/dev/nvmap", "/dev/nvhost-as-gpu", "/dev/nvhost-ctrl", "/dev/nvhost-ctrl-gpu", "/dev/nvhost-gpu"]) {
        *fd = nv.open(path)?;
    }
    let [nvmap, space, ctrl, ctrl_gpu, channel] = fds;
    let (mut characteristics, mut gpu) = ([0; 28], [0; 0xd0]);
    characteristics[0] = gpu.len() as u64;
    nv.ioctl3(ctrl_gpu, CTRL_GPU_GET_CHARACTERISTICS, &mut characteristics, &mut gpu)?;
    nv.ioctl(space, AS_INITIALIZE_EX, &mut AsInitializeEx { flags: 0, as_fd: 0, big_page_size: 0, reserved: 0, unknown: [0; 3] })?;
    nv.ioctl(space, AS_BIND_CHANNEL, &mut AsBindChannel { channel_fd: channel })?;
    nv.ioctl(channel, CHANNEL_SET_NVMAP_FD, &mut ChannelFd { fd: nvmap })?;
    let fence = Fence { id: 0, value: 0 };
    nv.ioctl(channel, CHANNEL_ALLOC_GPFIFO_EX2, &mut ChannelAllocGpfifoEx2 { entries: 0x800, flags: 0, unknown0: 0, fence, unknown: [0; 3] })?;
    let threed = u32::from_le_bytes(gpu[92..96].try_into().unwrap());
    nv.ioctl(channel, CHANNEL_ALLOC_OBJ_CTX, &mut ChannelAllocObjCtx { class: threed, flags: 0, object_id: 0 })?;
    nv.query_event(channel, 3)?;
    nv.ioctl(channel, CHANNEL_SET_PRIORITY, &mut ChannelPriority { priority: 0x64 })?;

    let (scene, target) = (heap + 0x10_0000, heap + 0x20_0000);
    let (scene_va, _) = buffer(&nv, [nvmap, space], scene, BUFFER as u32)?;
    let (target_va, target_handle) = buffer(&nv, [nvmap, space], target, (size[0] * size[1] * 4).next_multiple_of(BUFFER as u32))?;
    let at = |offset: u64| scene_va + offset;
    let addresses = Addresses { vertex_program: at(0x1000), pixel_program: at(0x2000), vertices: at(0x3000), cbuf: at(0x4000), report: at(0x5000), target: target_va };
    let mut out = Scene {
        pushbuffer: guest_slice(scene, 0x400),
        vertex_program: guest_slice(scene + 0x1000, scene::VERTEX_PROGRAM_BYTES),
        pixel_program: guest_slice(scene + 0x2000, scene::PIXEL_PROGRAM_BYTES),
        vertices: guest_slice(scene + 0x3000, 36),
    };
    let words = scene::triangle_scene(&mut out, &addresses, size, vertices, matrix, 1);
    let mut submit = ChannelSubmitGpfifo { gpfifo: 0, entries: 1, flags: 2, fence: Fence { id: 0, value: 0 } };
    nv.ioctl2(channel, CHANNEL_KICKOFF_PB, &mut submit, &scene::entry(scene_va, words))?;
    let (id, threshold) = (submit.fence.id, submit.fence.value);
    nv.ioctl(ctrl, CTRL_SYNCPT_WAIT_EVENT, &mut CtrlSyncptWaitEvent { id, threshold, timeout: u32::MAX, value: 0 })?;
    Ok((nv, nvmap, target_handle, target))
}
