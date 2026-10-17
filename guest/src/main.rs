#![no_std]
#![no_main]

use core::fmt::Write;
use guest::nv::Nv;
use guest::nvdrv::*;
use guest::{am_time, display, hid, svc};
use scene::{Addresses, Scene, Vertex};

core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "adrp x9, __stack_top",
    "mov sp, x9",
    "bl main",
    "b .",
);

const SIZE: u32 = 64;
const BUFFER: u64 = 0x1_0000;
const ORANGE: [u8; 4] = [255, 128, 0, 255];
// Pixel corners (4, 4), (60, 4) and (4, 60), as the host triangle test draws them.
const TRIANGLE: [Vertex; 3] = [([-0.875, -0.875], ORANGE), ([0.875, -0.875], ORANGE), ([-0.875, 0.875], ORANGE)];

/// An nvmap buffer of `BUFFER` bytes at `address`, mapped at a GPU address it returns.
fn buffer(nv: &Nv, [nvmap, space]: [u32; 2], address: u64) -> svc::Result<u64> {
    let mut create = NvmapCreate { size: BUFFER as u32, handle: 0 };
    nv.ioctl(nvmap, NVMAP_CREATE, &mut create)?;
    nv.ioctl(nvmap, NVMAP_ALLOC, &mut NvmapAlloc { handle: create.handle, heapmask: 0, flags: 1, align: 0x1000, kind: 0, pad: [0; 7], address })?;
    let mut reserved = AsAllocSpace { pages: 1, page_size: BUFFER as u32, flags: 0, pad: 0, offset_or_align: BUFFER };
    nv.ioctl(space, AS_ALLOC_SPACE, &mut reserved)?;
    let mut map = AsMapBufferEx { flags: 1, kind: 0, nvmap_handle: create.handle, page_size: BUFFER as u32, buffer_offset: 0, mapping_size: BUFFER, offset: reserved.offset_or_align };
    nv.ioctl(space, AS_MAP_BUFFER_EX, &mut map)?;
    Ok(map.offset)
}

fn guest_slice<T>(address: u64, len: usize) -> &'static mut [T] {
    unsafe { core::slice::from_raw_parts_mut(address as *mut T, len) }
}

/// Draws the scene crate's triangle through nvdrv, then queues three software frames.
fn draw() -> svc::Result<([u8; 4], [u32; 3])> {
    let heap = svc::set_heap_size(0x40_0000)?;
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
    let [scene_va, target_va] = [buffer(&nv, [nvmap, space], scene)?, buffer(&nv, [nvmap, space], target)?];
    let at = |offset: u64| scene_va + offset;
    let addresses = Addresses { vertex_program: at(0x1000), pixel_program: at(0x2000), vertices: at(0x3000), cbuf: at(0x4000), report: at(0x5000), target: target_va };
    let mut out = Scene {
        pushbuffer: guest_slice(scene, 0x400),
        vertex_program: guest_slice(scene + 0x1000, scene::VERTEX_PROGRAM_BYTES),
        pixel_program: guest_slice(scene + 0x2000, scene::PIXEL_PROGRAM_BYTES),
        vertices: guest_slice(scene + 0x3000, 36),
    };
    let words = scene::triangle_scene(&mut out, &addresses, [SIZE; 2], &TRIANGLE, [1.0, 0.0, 0.0, 1.0], 1);
    let mut submit = ChannelSubmitGpfifo { gpfifo: 0, entries: 1, flags: 2, fence: Fence { id: 0, value: 0 } };
    nv.ioctl2(channel, CHANNEL_KICKOFF_PB, &mut submit, &scene::entry(scene_va, words))?;
    let (id, threshold) = (submit.fence.id, submit.fence.value);
    nv.ioctl(ctrl, CTRL_SYNCPT_WAIT_EVENT, &mut CtrlSyncptWaitEvent { id, threshold, timeout: u32::MAX, value: 0 })?;
    let pixel = guest_slice::<[u8; 4]>(target + u64::from(20 * SIZE + 20) * 4, 1)[0];
    Ok((pixel, display::show(&nv, nvmap, heap + 0x30_0000)?))
}

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

fn print(line: core::fmt::Arguments) {
    let mut text = Text { bytes: [0; 64], len: 0 };
    let _ = text.write_fmt(line);
    let _ = svc::output_debug_string(core::str::from_utf8(&text.bytes[..text.len]).unwrap_or_default());
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    print(format_args!("hello from guest"));
    match am_time::initialize() {
        Ok((focus, time)) => print(format_args!("focus {focus} time {time}")),
        Err(code) => print(format_args!("am/time error {code:#x}")),
    }
    match hid::read_full_key() {
        Ok(pad) => print(format_args!("buttons {:#x} left {} {}", pad.buttons, pad.left[0], pad.left[1])),
        Err(code) => print(format_args!("hid error {code:#x}")),
    }
    match draw() {
        Ok((rgba, slots)) => {
            print(format_args!("pixel {:08x}", u32::from_be_bytes(rgba)));
            print(format_args!("slots {} {} {}", slots[0], slots[1], slots[2]));
        }
        Err(code) => print(format_args!("nvdrv error {code:#x}")),
    }
    svc::exit_process()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    svc::exit_process()
}
