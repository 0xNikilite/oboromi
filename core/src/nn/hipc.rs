#[repr(C)]
pub struct HeaderData {
    pub header: [u32; 2],
}

impl HeaderData {
    /// Decodes the fixed eight-byte header, ignoring any following payload.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let bytes = data.get(..8)?;
        let first = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let second = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Some(Self {
            header: [first, second],
        })
    }
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

/// Invokes the method only when the fixed HIPC header is present.
/// Descriptor and payload validation is not yet implemented.
pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: Fn() -> Result,
{
    let Some(header) = HeaderData::from_bytes(data) else {
        return Result::Failure;
    };

    let [hdr0, hdr1] = header.header;
    let _tag          = (hdr0 >>  0) & 0xffff;
    let _ptrs_count   = (hdr0 >> 16) & 0xf;
    let _send_count   = (hdr0 >> 20) & 0xf;
    let _recv_count   = (hdr0 >> 24) & 0xf;
    let _xchg_count   = (hdr0 >> 28) & 0xf;

    let _raw_count       = (hdr1 >>  0) & ((1 << 10) - 1);
    let _recv_list_count = (hdr1 >> 10) & 0xf;
    let _recv_list_offs  = (hdr1 >> 14) & 0x3ff;
    let _special_count   = (hdr1 >> 31) & 0x1;

    f()
}
