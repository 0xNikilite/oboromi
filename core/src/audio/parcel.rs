//! RequestUpdate parcels: a 0x40-byte header of section sizes, then the sections in order.
//! Record layouts follow libnx audren.h; fields added after REV6 follow the reference emulators.

use super::Error;

pub const HEADER: usize = 0x40;
pub const BEHAVIOUR: usize = 0x10;
pub const POOL_IN: usize = 0x20;
pub const POOL_OUT: usize = 0x10;
pub const CHANNEL_IN: usize = 0x70;
pub const VOICE_IN: usize = 0x170;
pub const VOICE_IN_V2: usize = 0x188;
pub const VOICE_OUT: usize = 0x10;
pub const EFFECT_IN: usize = 0xC0;
pub const EFFECT_OUT_V1: usize = 0x10;
pub const EFFECT_OUT_V2: usize = 0x90;
pub const MIX_DIRTY_HEADER: usize = 0x20;
pub const MIX_IN: usize = 0x930;
pub const SINK_IN: usize = 0x140;
pub const SINK_OUT: usize = 0x20;
pub const PERFORMANCE: usize = 0x10;
pub const ERROR_INFO_OUT: usize = 0xB0;
pub const RENDER_INFO_OUT: usize = 0x10;
pub const SPLITTER_HEADER: usize = 0x20;
pub const SPLITTER_IN: usize = 0x10;
pub const DESTINATION_V1: usize = 0x70;
pub const DESTINATION_V2: usize = 0x90;
pub const DESTINATION_V2B: usize = 0xA8;

pub fn bytes<const N: usize>(b: &[u8], at: usize) -> [u8; N] {
    b[at..at + N].try_into().unwrap()
}
pub fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes(b, at))
}
pub fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes(b, at))
}
pub fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes(b, at))
}
pub fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes(b, at))
}
pub fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes(b, at))
}
pub fn f32s<const N: usize>(b: &[u8], at: usize) -> [f32; N] {
    std::array::from_fn(|i| f32_at(b, at + i * 4))
}
pub fn put(b: &mut [u8], at: usize, value: &[u8]) {
    b[at..at + value.len()].copy_from_slice(value);
}

pub struct Reader<'a> {
    input: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let section = self
            .input
            .get(self.pos..self.pos + len)
            .ok_or(Error::InvalidUpdateInfo)?;
        self.pos += len;
        Ok(section)
    }

    /// a section whose header size must equal `count` records of `len` bytes.
    pub fn records(
        &mut self,
        size: u32,
        count: usize,
        len: usize,
    ) -> Result<std::slice::ChunksExact<'a, u8>, Error> {
        if size as usize != count * len {
            return Err(Error::InvalidUpdateInfo);
        }
        Ok(self.take(size as usize)?.chunks_exact(len))
    }
}

pub struct Writer<'a> {
    output: &'a mut [u8],
    pos: usize,
}

impl<'a> Writer<'a> {
    pub fn new(output: &'a mut [u8], revision: u32) -> Result<Self, Error> {
        output
            .get_mut(..HEADER)
            .ok_or(Error::InsufficientBuffer)?
            .fill(0);
        put(output, 0, &revision.to_le_bytes());
        Ok(Self {
            output,
            pos: HEADER,
        })
    }

    /// appends a zeroed record and accounts it in the header slot at `size_at`.
    pub fn record(&mut self, size_at: usize, len: usize) -> Result<&mut [u8], Error> {
        let start = self.pos;
        let record = self
            .output
            .get_mut(start..start + len)
            .ok_or(Error::InsufficientBuffer)?;
        record.fill(0);
        self.pos += len;
        let size = u32_at(self.output, size_at) + len as u32;
        put(self.output, size_at, &size.to_le_bytes());
        put(self.output, 0x3C, &(self.pos as u32).to_le_bytes());
        Ok(&mut self.output[start..start + len])
    }
}
