#[repr(C)]
pub struct HeaderData {
    pub header: [u32; 2],
}

#[repr(C)]
pub struct MapData {
    pub data: [u32; 3],
}

#[repr(C)]
pub struct PointerData {
    pub data: [u32; 2],
}

#[repr(C)]
pub struct ReceiveListData {
    pub data: [u32; 2],
}

#[repr(u32)]
pub enum Result {
    Success = 0,
    Failure = 1,
}

pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: Fn() -> Result,
{
    let hdr0 = unsafe { *(data.as_ptr() as *const u32) };
    let _tag          = (hdr0 >>  0) & 0xffff;
    let _ptrs_count   = (hdr0 >> 16) & 0xf;
    let _send_count   = (hdr0 >> 20) & 0xf;
    let _recv_count   = (hdr0 >> 24) & 0xf;
    let _xchg_count   = (hdr0 >> 28) & 0xf;

    let hdr1 = unsafe { *(data.as_ptr().byte_add(4) as *const u32) };
    let _raw_count       = (hdr1 >>  0) & ((1 << 10) - 1);
    let _recv_list_count = (hdr1 >> 10) & 0xf;
    let _recv_list_offs  = (hdr1 >> 14) & 0x3ff;
    let _special_count   = (hdr1 >> 31) & 0x1;

    f()
}