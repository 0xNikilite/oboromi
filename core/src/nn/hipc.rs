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
    pub data: [u32; 3],
}

#[repr(C)]
pub struct ReceiveListData {
    pub data: [u32; 3],
}

type Header0Tag = u32;
type Header1Tag = u32;
type SpecialTag = u32;
type Map0Tag = u32;
type Map1Tag = u32;
type Map2Tag = u32;

#[repr(u32)]
pub enum Result {
    Sucess = 0
}

pub fn invoke_method<F>(data: &[u8], f: F)
where
    F: Fn() -> Result
{
    let hdr0 = unsafe { *(data.as_ptr().byte_add(0) as *const u32) };
    let tag = (hdr0 >> 0) & 0xffff;
    let ptrs_count = (hdr0 >> 16) & 0xf;
    let send_count = (hdr0 >> 20) & 0xf;
    let recv_count = (hdr0 >> 24) & 0xf;
    let xchg_count = (hdr0 >> 24) & 0xf;
    // hdr1
    let hdr1 = unsafe { *(data.as_ptr().byte_add(4) as *const u32) };
    let raw_count = (hdr1 >> 0) & ((1 << 9) - 1);
    let recv_list_count = (hdr1 >> 10) & 0xf;
    let resv = (hdr1 >> 24) & 0xf;
    let recv_list_offs = (hdr1 >> 20) & 0xf;
    let special_count = (hdr1 >> 31) & 0x1;
}
