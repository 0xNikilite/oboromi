//! libnx's default window and software framebuffer (nx/source/services/vi.c, display/{default_window,native_window,
//! framebuffer,buffer_producer,binder,parcel}.c): vi:m through sm:, a stray layer on the Default display and two
//! block-linear RGBA8888 framebuffers in one nvmap buffer, queued through the layer's IGraphicBufferProducer.
use crate::cmif::{Buffer, Request};
use crate::hid::call;
use crate::nv::Nv;
use crate::nvdrv::{NVMAP_ALLOC, NVMAP_CREATE, NVMAP_GET_ID, NvmapAlloc, NvmapCreate, NvmapGetId};
use crate::svc::{self, Result};
use core::mem::size_of;

pub const WIDTH: u32 = 128;
pub const HEIGHT: u32 = 64;
const PITCH: u32 = WIDTH * 4;
const BLOCK_HEIGHT_LOG2: u32 = 4;
const FRAMEBUFFER: u32 = PITCH * HEIGHT.next_multiple_of(8 << BLOCK_HEIGHT_LOG2);
const TOKEN: &[u8] = b"android.gui.IGraphicBufferProducer";
const RGBA_8888: u32 = 1;
/// GRALLOC_USAGE_HW_COMPOSER | GRALLOC_USAGE_HW_RENDER | GRALLOC_USAGE_HW_TEXTURE.
const USAGE: u32 = 0xb00;
const A8B8G8R8: u64 = 0x01_0053_2120;

fn buffer<T>(value: &mut T) -> Buffer {
    Buffer { address: core::ptr::from_mut(value) as u64, size: size_of::<T>() as u64 }
}

struct Window {
    relay: u32,
    binder: u32,
}

impl Window {
    fn open() -> Result<Self> {
        let sm = svc::connect_to_named_port(c"sm:")?;
        call(sm, Request { command: 0, send_pid: true, data: &[0, 0], ..Default::default() })?;
        let root = call(sm, Request { command: 1, data: &[u32::from_le_bytes(*b"vi:m"), 0], ..Default::default() })?;
        let display = call(root, Request { command: 2, data: &[1], ..Default::default() })?;
        let mut sessions = [0; 4];
        for (session, command) in sessions.iter_mut().zip(100..) {
            *session = call(display, Request { command, ..Default::default() })?;
        }
        let [relay, _, manager, _] = sessions;
        let mut name = [0u32; 16];
        name[..2].copy_from_slice(&[u32::from_le_bytes(*b"Defa"), u32::from_le_bytes(*b"ult\0")]);
        let id = svc::request(display, &Request { command: 1010, data: &name, ..Default::default() })?.data;
        let mut native_window = [0u32; 64];
        let request = Request { command: 2012, data: &[0, 0, id[0], id[1]], receive: &[buffer(&mut native_window)], ..Default::default() };
        let layer = svc::request(manager, &request)?.data;
        call(display, Request { command: 2101, data: &[2, 0, layer[0], layer[1]], ..Default::default() })?;

        let window = Self { relay, binder: native_window[native_window[1] as usize / 4 + 2] };
        for data in [[window.binder, 1, 0], [window.binder, 1, 1]] {
            call(relay, Request { command: 1, data: &data, ..Default::default() })?;
        }
        call(relay, Request { command: 2, data: &[window.binder, 0x0f], ..Default::default() })?;
        window.transact(10, &[0, 2, 0])?;

        Ok(window)
    }

    fn set_buffer(&self, slot: u32, nvmap_id: u32, [width, height]: [u32; 2], size: u32, offset: u32, layout: u32, block_height: u32) -> Result<()> {
        let header = [0x4742_4652, width, height, width, RGBA_8888, USAGE, 42, 0, 0, 81];
        let mut graphic_buffer = [0; 81];
        graphic_buffer[..14].copy_from_slice(&[u32::MAX, nvmap_id, 0, 0xDAFF_CAFF, 42, 0, USAGE, RGBA_8888, RGBA_8888, width, size, 1, 0, width]);
        graphic_buffer[14..23].copy_from_slice(&[height, A8B8G8R8 as u32, (A8B8G8R8 >> 32) as u32, layout, width * 4, 0, offset, 0xfe, block_height]);
        graphic_buffer[27] = size;
        let mut args = [0; 95];
        args[..4].copy_from_slice(&[slot, 1, 4 * (header.len() + graphic_buffer.len()) as u32, 0]);
        args[4..14].copy_from_slice(&header);
        args[14..].copy_from_slice(&graphic_buffer);
        self.transact(14, &args).map(drop)
    }

    /// Sends IGraphicBufferProducer transaction `code` and returns the first word of the reply, once its status is 0.
    fn transact(&self, code: u32, args: &[u32]) -> Result<u32> {
        let (mut parcel, mut reply) = ([0u32; 128], [0u32; 128]);
        let token = (TOKEN.len() + 1).div_ceil(2);
        let size = 4 * (2 + token + args.len()) as u32;
        parcel[..6].copy_from_slice(&[size, 16, 0, 16 + size, 0x100, TOKEN.len() as u32]);
        for (i, &c) in TOKEN.iter().enumerate() {
            parcel[6 + i / 2] |= u32::from(c) << (i % 2 * 16);
        }
        parcel[6 + token..][..args.len()].copy_from_slice(args);
        let request = Request { command: 3, send: &[buffer(&mut parcel)], receive: &[buffer(&mut reply)], data: &[self.binder, code, 0], ..Default::default() };
        svc::request(self.relay, &request)?;
        let payload = &reply[reply[1] as usize / 4..][..reply[0] as usize / 4];
        match payload.last() {
            Some(0) => Ok(payload[0]),
            status => Err(status.copied().unwrap_or(u32::MAX)),
        }
    }
}

/// Where the RGBA8 pixel (x, y) is in a framebuffer, as Ryujinx BlockLinearLayout lays out GOBs.
fn offset(x: u32, y: u32) -> u32 {
    let (x, block) = (x * 4, 512 << BLOCK_HEIGHT_LOG2);
    (y >> 3 >> BLOCK_HEIGHT_LOG2) * (PITCH / 64) * block + x / 64 * block + (y >> 3 & ((1 << BLOCK_HEIGHT_LOG2) - 1)) * 512
        + (x & 63) / 32 * 256 + (y & 7) / 2 * 64 + (x & 31) / 16 * 32 + (y & 1) * 16 + (x & 15)
}

/// Opens the default window and queues three frames of a test pattern from framebuffers at `memory`, returning the
/// queued slots.
pub fn show(nv: &Nv, nvmap: u32, memory: u64) -> Result<[u32; 3]> {
    let window = Window::open()?;

    let mut create = NvmapCreate { size: 2 * FRAMEBUFFER, handle: 0 };
    nv.ioctl(nvmap, NVMAP_CREATE, &mut create)?;
    nv.ioctl(nvmap, NVMAP_ALLOC, &mut NvmapAlloc { handle: create.handle, heapmask: 0, flags: 1, align: 0x20000, kind: 0, pad: [0; 7], address: memory })?;
    let mut nvmap_id = NvmapGetId { id: 0, handle: create.handle };
    nv.ioctl(nvmap, NVMAP_GET_ID, &mut nvmap_id)?;
    for slot in 0..2 {
        window.set_buffer(slot, nvmap_id.id, [WIDTH, HEIGHT], FRAMEBUFFER, slot * FRAMEBUFFER, 3, BLOCK_HEIGHT_LOG2)?;
    }

    let (mut slots, mut requested) = ([0; 3], 0);
    for (frame, queued) in slots.iter_mut().enumerate() {
        // No WaitSynchronization yet, so this skips nwindowDequeueBuffer's wait on the buffer event and asks asynchronously.
        let slot = window.transact(3, &[1, WIDTH, HEIGHT, RGBA_8888, USAGE])?;
        if requested & 1 << slot == 0 {
            window.transact(1, &[slot])?;
            requested |= 1 << slot;
        }
        let framebuffer = memory + u64::from(slot * FRAMEBUFFER);
        for (x, y) in (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))) {
            let rgba = [x as u8 * 2, y as u8 * 4, 0x40 * (frame as u8 + 1), 0xff];
            unsafe { core::ptr::write_volatile((framebuffer + u64::from(offset(x, y))) as *mut [u8; 4], rgba) };
        }
        let mut input = [0; 24];
        input[..3].copy_from_slice(&[slot, 84, 0]);
        input[3 + 11] = 1;
        window.transact(7, &input)?;
        *queued = slot;
    }
    Ok(slots)
}

/// Queues the pitch-linear render target without copying or rewriting its pixels.
pub fn present(nv: &Nv, nvmap: u32, handle: u32, size: [u32; 2]) -> Result<()> {
    let window = Window::open()?;
    let mut id = NvmapGetId { id: 0, handle };
    nv.ioctl(nvmap, NVMAP_GET_ID, &mut id)?;
    window.set_buffer(0, id.id, size, size[0] * size[1] * 4, 0, 1, 0)?;
    let slot = window.transact(3, &[1, size[0], size[1], RGBA_8888, USAGE])?;
    window.transact(1, &[slot])?;
    let mut input = [0; 24];
    input[..3].copy_from_slice(&[slot, 84, 0]);
    input[3 + 11] = 1;
    window.transact(7, &input).map(drop)
}
