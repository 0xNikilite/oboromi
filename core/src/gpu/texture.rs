//! Ampere texture header (TIC) and sampler (TSC) pool entries, bit positions as in open-gpu-doc clc697tex.h.
//! Only pitch-linear one-mip RGBA8 2D textures decode; any other header or sampler state is rejected.
use ash::vk;

pub struct TextureHeader {
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub address: u64,
}

#[derive(Default)]
pub struct TexturePool<'a> {
    pub tic: &'a [u8],
    pub tsc: &'a [u8],
}

impl TexturePool<'_> {
    pub fn texture(&self, handle: u32) -> Result<TextureHeader, String> {
        const PITCH: u32 = 2;
        const A8B8G8R8: u32 = 8;
        const UNORM_RGBA: u32 = 0b010_010_010_010;
        const IDENTITY_SWIZZLE: u32 = 0o5432;
        const TWO_D: u32 = 1;
        let e = entry(self.tic, handle & 0xf_ffff)?;
        check("header version", field(e, 85, 87), PITCH)?;
        check("components", field(e, 0, 6), A8B8G8R8)?;
        check("component types", field(e, 7, 18), UNORM_RGBA)?;
        check("swizzle", field(e, 19, 30), IDENTITY_SWIZZLE)?;
        check("texture type", field(e, 151, 154), TWO_D)?;
        check("max mip level", field(e, 124, 127), 0)?;
        check("normalized coords", field(e, 191, 191), 1)?;
        Ok(TextureHeader {
            format: vk::Format::R8G8B8A8_UNORM,
            width: field(e, 128, 144) + 1,
            height: (field(e, 146, 146) << 16 | field(e, 160, 175)) + 1,
            pitch: field(e, 145, 145) << 21 | field(e, 96, 111) << 5,
            address: (field(e, 64, 80) as u64) << 32 | (field(e, 37, 63) as u64) << 5,
        })
    }

    pub fn sampler(&self, handle: u32) -> Result<vk::SamplerCreateInfo<'static>, String> {
        let e = entry(self.tsc, handle >> 20)?;
        check("depth compare", field(e, 9, 9), 0)?;
        check("max anisotropy", field(e, 20, 22), 0)?;
        check("reduction filter", field(e, 42, 43), 0)?;
        check("unnormalized coords", field(e, 57, 57), 0)?;
        Ok(vk::SamplerCreateInfo::default()
            .address_mode_u(wrap(field(e, 0, 2))?)
            .address_mode_v(wrap(field(e, 3, 5))?)
            .mag_filter(filter(field(e, 32, 34))?)
            .min_filter(filter(field(e, 36, 37))?))
    }
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
