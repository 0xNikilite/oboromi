//! Ampere texture header (TIC) and sampler (TSC) pool entries, bit positions as in open-gpu-doc clc797tex.h.
//! Only 2D textures decode, pitch-linear with one level or block-linear with a mip chain, in the formats of
//! `TexturePool::texture`; any other header or sampler state is rejected.
use ash::vk;

pub enum Layout {
    Pitch(u32),
    BlockLinear { gob_height_log2: u32 },
}

pub struct TextureHeader {
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    pub layout: Layout,
    pub address: u64,
    pub normalized: bool,
    block_bytes: u32,
    block_texels: u32,
}

#[derive(Default)]
pub struct TexturePool<'a> {
    pub tic: &'a [u8],
    pub tsc: &'a [u8],
}

impl TexturePool<'_> {
    pub fn texture(&self, handle: u32) -> Result<TextureHeader, String> {
        const PITCH: u32 = 2;
        const BLOCK_LINEAR: u32 = 3;
        const R16_G16_B16_A16: u32 = 0x03;
        const A8B8G8R8: u32 = 0x08;
        const R32: u32 = 0x0f;
        const BC7U: u32 = 0x17;
        const G8R8: u32 = 0x18;
        const R8: u32 = 0x1d;
        const DXT1: u32 = 0x24;
        const DXT45: u32 = 0x26;
        const UNORM: u32 = 0o2222;
        const UINT: u32 = 0o4444;
        const FLOAT: u32 = 0o7777;
        const IDENTITY_SWIZZLE: u32 = 0o5432;
        const TWO_D: u32 = 1;
        let e = entry(self.tic, handle & 0xf_ffff)?;
        let srgb = field(e, 150, 150) == 1;
        let (format, block_bytes, block_texels) = match (field(e, 0, 6), field(e, 7, 18), srgb) {
            (A8B8G8R8, UNORM, false) => (vk::Format::R8G8B8A8_UNORM, 4, 1),
            (A8B8G8R8, UNORM, true) => (vk::Format::R8G8B8A8_SRGB, 4, 1),
            (R8, UNORM, false) => (vk::Format::R8_UNORM, 1, 1),
            (G8R8, UNORM, false) => (vk::Format::R8G8_UNORM, 2, 1),
            (R16_G16_B16_A16, FLOAT, false) => (vk::Format::R16G16B16A16_SFLOAT, 8, 1),
            (R32, UINT, false) => (vk::Format::R32_UINT, 4, 1),
            (DXT1, UNORM, false) => (vk::Format::BC1_RGBA_UNORM_BLOCK, 8, 4),
            (DXT45, UNORM, false) => (vk::Format::BC3_UNORM_BLOCK, 16, 4),
            (BC7U, UNORM, false) => (vk::Format::BC7_UNORM_BLOCK, 16, 4),
            (components, types, srgb) => {
                return Err(format!("unsupported components {components:#x} with component types {types:#o}, srgb {srgb}"));
            }
        };
        check("swizzle", field(e, 19, 30), IDENTITY_SWIZZLE)?;
        check("texture type", field(e, 151, 154), TWO_D)?;
        check("view min mip level", field(e, 224, 227), 0)?;
        let levels = field(e, 124, 127) + 1;
        let layout = match field(e, 85, 87) {
            PITCH => {
                check("pitch-linear max mip level", field(e, 124, 127), 0)?;
                Layout::Pitch(field(e, 145, 145) << 21 | field(e, 96, 111) << 5)
            }
            BLOCK_LINEAR => {
                check("gobs per block width", field(e, 96, 98), 0)?;
                check("gobs per block depth", field(e, 102, 104), 0)?;
                check("tile width in gobs", field(e, 106, 108), 0)?;
                Layout::BlockLinear { gob_height_log2: field(e, 99, 101) }
            }
            version => return Err(format!("unsupported header version {version}")),
        };
        Ok(TextureHeader {
            format,
            width: field(e, 128, 144) + 1,
            height: (field(e, 146, 146) << 16 | field(e, 160, 175)) + 1,
            levels,
            layout,
            address: (field(e, 64, 80) as u64) << 32 | (field(e, 37, 63) as u64) << 5,
            normalized: field(e, 191, 191) == 1,
            block_bytes,
            block_texels,
        })
    }

    pub fn sampler(&self, handle: u32) -> Result<vk::SamplerCreateInfo<'static>, String> {
        let e = entry(self.tsc, handle >> 20)?;
        check("depth compare", field(e, 9, 9), 0)?;
        check("reduction filter", field(e, 42, 43), 0)?;
        check("unnormalized coords", field(e, 57, 57), 0)?;
        const MIP_NONE: u32 = 1;
        const MIP_LINEAR: u32 = 3;
        let mip_filter = field(e, 38, 39);
        let lod = |lo| field(e, lo, lo + 11) as f32 / 256.0;
        // A max lod of 0.25 is the Vulkan spec's no-mipmap sampler: level 0 only, with min and mag filters still apart.
        let (min_lod, max_lod) = if mip_filter <= MIP_NONE { (0.0, 0.25) } else { (lod(64), lod(76)) };
        Ok(vk::SamplerCreateInfo::default()
            .address_mode_u(wrap(field(e, 0, 2))?)
            .address_mode_v(wrap(field(e, 3, 5))?)
            .mag_filter(filter(field(e, 32, 34))?)
            .min_filter(filter(field(e, 36, 37))?)
            .mipmap_mode(if mip_filter == MIP_LINEAR { vk::SamplerMipmapMode::LINEAR } else { vk::SamplerMipmapMode::NEAREST })
            .anisotropy_enable(field(e, 20, 22) > 0)
            .max_anisotropy([1.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 16.0][field(e, 20, 22) as usize])
            .min_lod(min_lod)
            .max_lod(max_lod))
    }
}

impl TextureHeader {
    /// How many guest bytes from the header address `upload` reads.
    pub fn guest_bytes(&self) -> usize {
        (0..self.levels)
            .map(|level| {
                let [columns, rows] = [self.width, self.height].map(|n| (n >> level).max(1).div_ceil(self.block_texels));
                match self.layout {
                    Layout::Pitch(pitch) => (pitch * rows) as usize,
                    Layout::BlockLinear { gob_height_log2 } => {
                        block_linear_size((columns * self.block_bytes) as usize, rows as usize, level_gob_height_log2(rows, gob_height_log2))
                    }
                }
            })
            .sum()
    }

    /// The texel blocks of every level in guest `memory` from the header address, packed row after row and level after
    /// level, with the copy of each level into the image.
    pub fn upload(&self, memory: &[u8]) -> (Vec<u8>, Vec<vk::BufferImageCopy>) {
        let (mut texels, mut copies, mut guest) = (Vec::new(), Vec::new(), 0);
        for level in 0..self.levels {
            let [width, height] = [self.width, self.height].map(|n| (n >> level).max(1));
            let [columns, rows] = [width, height].map(|n| n.div_ceil(self.block_texels));
            let row_bytes = (columns * self.block_bytes) as usize;
            copies.push(
                vk::BufferImageCopy::default()
                    .buffer_offset(texels.len() as u64)
                    .image_subresource(vk::ImageSubresourceLayers::default().aspect_mask(vk::ImageAspectFlags::COLOR).mip_level(level).layer_count(1))
                    .image_extent(vk::Extent3D { width, height, depth: 1 }),
            );
            match self.layout {
                Layout::Pitch(pitch) => texels.extend(memory.chunks(pitch as usize).take(rows as usize).flat_map(|row| &row[..row_bytes])),
                Layout::BlockLinear { gob_height_log2 } => {
                    let gob_height_log2 = level_gob_height_log2(rows, gob_height_log2);
                    let size = block_linear_size(row_bytes, rows as usize, gob_height_log2);
                    texels.extend(block_linear_to_pitch(&memory[guest..guest + size], columns, rows, self.block_bytes, gob_height_log2));
                    guest += size;
                }
            }
        }
        (texels, copies)
    }
}

/// The gob height a mip level of `rows` texel block rows uses: the texture's, halved while half of it still covers the
/// level, as Ryujinx SizeCalculator.GetBlockLinearTextureSize shrinks it.
pub(super) fn level_gob_height_log2(rows: u32, gob_height_log2: u32) -> u32 {
    (0..=gob_height_log2).rev().find(|&g| g == 0 || rows > 4 << g).unwrap()
}

fn block_linear_size(row_bytes: usize, rows: usize, gob_height_log2: u32) -> usize {
    rows.div_ceil(8 << gob_height_log2) * row_bytes.div_ceil(64) * (512 << gob_height_log2)
}

// A GOB is 64 bytes by 8 rows, its 16-byte sectors interleaved with row pairs, and blocks of 2^gob_height_log2 GOBs
// stacked along Y run left to right, then down (Ryujinx BlockLinearLayout.GetOffsetWithLineOffset).
fn block_linear_offset(x: usize, y: usize, row_bytes: usize, gob_height_log2: u32) -> usize {
    let block = 512 << gob_height_log2;
    (y >> 3 >> gob_height_log2) * row_bytes.div_ceil(64) * block
        + x / 64 * block
        + (y >> 3 & (1 << gob_height_log2) - 1) * 512
        + (x & 63) / 32 * 256
        + (y & 7) / 2 * 64
        + (x & 31) / 16 * 32
        + (y & 1) * 16
        + (x & 15)
}

pub fn block_linear_to_pitch(src: &[u8], width: u32, height: u32, bytes_per_pixel: u32, gob_height_log2: u32) -> Vec<u8> {
    let row_bytes = (width * bytes_per_pixel) as usize;
    (0..height as usize)
        .flat_map(|y| (0..row_bytes).map(move |x| src[block_linear_offset(x, y, row_bytes, gob_height_log2)]))
        .collect()
}

pub fn pitch_to_block_linear(src: &[u8], width: u32, height: u32, bytes_per_pixel: u32, gob_height_log2: u32) -> Vec<u8> {
    let row_bytes = (width * bytes_per_pixel) as usize;
    let mut out = vec![0; block_linear_size(row_bytes, height as usize, gob_height_log2)];
    for (i, &byte) in src.iter().enumerate() {
        out[block_linear_offset(i % row_bytes, i / row_bytes, row_bytes, gob_height_log2)] = byte;
    }
    out
}

fn entry(pool: &[u8], index: u32) -> Result<&[u8], String> {
    pool.chunks_exact(32).nth(index as usize).ok_or_else(|| format!("entry {index} is outside the pool"))
}

fn field(entry: &[u8], lo: usize, hi: usize) -> u32 {
    let word = u32::from_le_bytes(entry[lo / 32 * 4..][..4].try_into().unwrap());
    word >> (lo % 32) & u32::MAX >> (31 - (hi - lo))
}

fn check(name: &str, value: u32, supported: u32) -> Result<(), String> {
    if value == supported { Ok(()) } else { Err(format!("unsupported {name} {value:#x}")) }
}

fn wrap(mode: u32) -> Result<vk::SamplerAddressMode, String> {
    match mode {
        0 => Ok(vk::SamplerAddressMode::REPEAT),
        2 => Ok(vk::SamplerAddressMode::CLAMP_TO_EDGE),
        mode => Err(format!("unsupported wrap mode {mode}")),
    }
}

fn filter(mode: u32) -> Result<vk::Filter, String> {
    match mode {
        1 => Ok(vk::Filter::NEAREST),
        2 => Ok(vk::Filter::LINEAR),
        mode => Err(format!("unsupported filter {mode}")),
    }
}
