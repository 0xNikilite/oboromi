//! One synthetic C7B5 submission per step of the Qt texture demo.
use super::State;

pub(crate) fn transfer(input: &[u8], width: u32, height: u32, upload: bool) -> Option<Vec<u8>> {
    // Whole GOBs keep the raw storage preview the same size as the source.
    if width == 0
        || width > 512
        || width % 16 != 0
        || height == 0
        || height > 512
        || height % 8 != 0
    {
        return None;
    }
    let pitch = width * 4;
    if input.len() != (pitch * height) as usize {
        return None;
    }
    let mut gpu = State::new();
    let surface = if upload { 0x70c } else { 0x728 };
    let flags = if upload { 0x282 } else { 0x302 };
    let words: [u32; 19] = [
        0x2001_0000,
        0xc7b5,
        0x2006_0000 | surface / 4,
        0x1000,
        pitch,
        height,
        1,
        0,
        0,
        0x2008_0100,
        0,
        0x100000,
        0,
        0x200000,
        pitch,
        pitch,
        pitch,
        height,
        0x8000_00c0 | (flags << 16),
    ];
    gpu.address_space
        .map(0x1000, words.iter().flat_map(|w| w.to_le_bytes()).collect())
        .ok()?;
    gpu.address_space.map(0x100000, input.to_vec()).ok()?;
    gpu.address_space.map(0x200000, vec![0; input.len()]).ok()?;
    gpu.submit_gpfifo(&[0x1000 | (words.len() as u64) << 42])
        .ok()?;
    gpu.address_space.unmap(0x200000)
}
