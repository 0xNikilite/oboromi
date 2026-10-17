//! The nvdrv ioctl arguments, laid out as libnx's nx/source/nvidia/ioctl passes them, and their ioctl numbers.
use core::mem::size_of;

const fn ioctl<T>(direction: u32, kind: u32, number: u32) -> u32 {
    direction << 30 | (size_of::<T>() as u32) << 16 | kind << 8 | number
}

const fn write<T>(kind: u32, number: u32) -> u32 {
    ioctl::<T>(1, kind, number)
}

const fn read<T>(kind: u32, number: u32) -> u32 {
    ioctl::<T>(2, kind, number)
}

const fn read_write<T>(kind: u32, number: u32) -> u32 {
    ioctl::<T>(3, kind, number)
}

#[repr(C)]
pub struct Fence {
    pub id: u32,
    pub value: u32,
}

#[repr(C)]
pub struct NvmapCreate {
    pub size: u32,
    pub handle: u32,
}

#[repr(C)]
pub struct NvmapAlloc {
    pub handle: u32,
    pub heapmask: u32,
    pub flags: u32,
    pub align: u32,
    pub kind: u8,
    pub pad: [u8; 7],
    pub address: u64,
}

#[repr(C)]
pub struct NvmapParam {
    pub handle: u32,
    pub param: u32,
    pub result: u32,
}

#[repr(C)]
pub struct NvmapGetId {
    pub id: u32,
    pub handle: u32,
}

#[repr(C)]
pub struct AsBindChannel {
    pub channel_fd: u32,
}

#[repr(C)]
pub struct AsAllocSpace {
    pub pages: u32,
    pub page_size: u32,
    pub flags: u32,
    pub pad: u32,
    pub offset_or_align: u64,
}

#[repr(C)]
pub struct AsMapBufferEx {
    pub flags: u32,
    pub kind: u32,
    pub nvmap_handle: u32,
    pub page_size: u32,
    pub buffer_offset: u64,
    pub mapping_size: u64,
    pub offset: u64,
}

#[repr(C)]
pub struct AsInitializeEx {
    pub flags: u32,
    pub as_fd: i32,
    pub big_page_size: u32,
    pub reserved: u32,
    pub unknown: [u64; 3],
}

#[repr(C)]
pub struct CtrlGpuZcullGetCtxSize {
    pub size: u32,
}

#[repr(C)]
pub struct CtrlSyncptRead {
    pub id: u32,
    pub value: u32,
}

#[repr(C)]
pub struct CtrlSyncptWait {
    pub id: u32,
    pub threshold: u32,
    pub timeout: u32,
}

#[repr(C)]
pub struct ChannelFd {
    pub fd: u32,
}

#[repr(C)]
pub struct ChannelAllocObjCtx {
    pub class: u32,
    pub flags: u32,
    pub object_id: u64,
}

#[repr(C)]
pub struct ChannelZcullBind {
    pub gpu_va: u64,
    pub mode: u32,
    pub pad: u32,
}

#[repr(C)]
pub struct ChannelSetErrorNotifier {
    pub offset: u64,
    pub size: u64,
    pub enable: u32,
    pub pad: u32,
}

#[repr(C)]
pub struct ChannelPriority {
    pub priority: u32,
}

#[repr(C)]
pub struct ChannelAllocGpfifoEx2 {
    pub entries: u32,
    pub flags: u32,
    pub unknown0: u32,
    pub fence: Fence,
    pub unknown: [u32; 3],
}

/// Followed in the same ioctl buffer by `entries` GPFIFO entries for `SUBMIT_GPFIFO`, or passed with them in Ioctl2's
/// second buffer for `KICKOFF_PB`.
#[repr(C)]
pub struct ChannelSubmitGpfifo {
    pub gpfifo: u64,
    pub entries: u32,
    pub flags: u32,
    pub fence: Fence,
}

pub const NVMAP_CREATE: u32 = read_write::<NvmapCreate>(0x01, 0x01);
pub const NVMAP_ALLOC: u32 = read_write::<NvmapAlloc>(0x01, 0x04);
pub const NVMAP_PARAM: u32 = read_write::<NvmapParam>(0x01, 0x09);
pub const NVMAP_GET_ID: u32 = read_write::<NvmapGetId>(0x01, 0x0e);
pub const AS_BIND_CHANNEL: u32 = write::<AsBindChannel>(0x41, 0x01);
pub const AS_ALLOC_SPACE: u32 = read_write::<AsAllocSpace>(0x41, 0x02);
pub const AS_MAP_BUFFER_EX: u32 = read_write::<AsMapBufferEx>(0x41, 0x06);
pub const AS_INITIALIZE_EX: u32 = write::<AsInitializeEx>(0x41, 0x09);
pub const CTRL_GPU_ZCULL_GET_CTX_SIZE: u32 = read::<CtrlGpuZcullGetCtxSize>(0x47, 0x01);
pub const CTRL_SYNCPT_READ: u32 = read_write::<CtrlSyncptRead>(0x00, 0x14);
pub const CTRL_SYNCPT_WAIT: u32 = read_write::<CtrlSyncptWait>(0x00, 0x16);
pub const CHANNEL_SET_NVMAP_FD: u32 = write::<ChannelFd>(0x48, 0x01);
pub const CHANNEL_ALLOC_OBJ_CTX: u32 = read_write::<ChannelAllocObjCtx>(0x48, 0x09);
pub const CHANNEL_ZCULL_BIND: u32 = read_write::<ChannelZcullBind>(0x48, 0x0b);
pub const CHANNEL_SET_ERROR_NOTIFIER: u32 = read_write::<ChannelSetErrorNotifier>(0x48, 0x0c);
pub const CHANNEL_SET_PRIORITY: u32 = write::<ChannelPriority>(0x48, 0x0d);
pub const CHANNEL_ALLOC_GPFIFO_EX2: u32 = read_write::<ChannelAllocGpfifoEx2>(0x48, 0x1a);
pub const CHANNEL_KICKOFF_PB: u32 = read_write::<ChannelSubmitGpfifo>(0x48, 0x1b);

pub const fn channel_submit_gpfifo(entries: u32) -> u32 {
    read_write::<ChannelSubmitGpfifo>(0x48, 0x08) + ((8 * entries) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::offset_of;

    // Sizes from Eden's static_asserts in core/hle/service/nvdrv/devices, the numbers from libnx's _NV_IOWR.
    #[test]
    fn ioctl_arguments_match_the_driver_layouts() {
        let sizes = [NVMAP_CREATE, NVMAP_ALLOC, NVMAP_PARAM, NVMAP_GET_ID, AS_BIND_CHANNEL, AS_ALLOC_SPACE, AS_MAP_BUFFER_EX, AS_INITIALIZE_EX];
        assert_eq!(sizes.map(|ioctl| ioctl >> 16 & 0x3fff), [8, 32, 12, 8, 4, 24, 40, 40]);
        let sizes = [CTRL_GPU_ZCULL_GET_CTX_SIZE, CTRL_SYNCPT_READ, CTRL_SYNCPT_WAIT, CHANNEL_SET_NVMAP_FD, CHANNEL_ALLOC_OBJ_CTX, CHANNEL_ZCULL_BIND];
        assert_eq!(sizes.map(|ioctl| ioctl >> 16 & 0x3fff), [4, 8, 12, 4, 16, 16]);
        let sizes = [CHANNEL_SET_ERROR_NOTIFIER, CHANNEL_SET_PRIORITY, CHANNEL_ALLOC_GPFIFO_EX2, CHANNEL_KICKOFF_PB, channel_submit_gpfifo(2)];
        assert_eq!(sizes.map(|ioctl| ioctl >> 16 & 0x3fff), [24, 4, 32, 24, 40]);
        assert_eq!([NVMAP_CREATE, AS_INITIALIZE_EX, CTRL_GPU_ZCULL_GET_CTX_SIZE], [0xc008_0101, 0x4028_4109, 0x8004_4701]);
        let offsets = [offset_of!(NvmapAlloc, address), offset_of!(AsMapBufferEx, buffer_offset), offset_of!(ChannelAllocGpfifoEx2, fence), offset_of!(ChannelSubmitGpfifo, fence)];
        assert_eq!(offsets, [24, 16, 12, 16]);
    }
}
