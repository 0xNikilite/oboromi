pub const CLASS_ID: u16 = 0xc7c0;

const REGISTER_BYTES: usize = 0x1600;
pub const SET_QMD_VERSION: usize = 0x0288;
pub const CHECK_QMD_VERSION: usize = 0x0290;
pub const SET_SHADER_SHARED_MEMORY_WINDOW_A: usize = 0x02a0;
pub const SET_SHADER_SHARED_MEMORY_WINDOW_B: usize = 0x02a4;
pub const SEND_PCAS_A: usize = 0x02b4;
pub const SEND_PCAS_B: usize = 0x02b8;
pub const SEND_SIGNALING_PCAS_B: usize = 0x02bc;
pub const SEND_SIGNALING_PCAS2_B: usize = 0x02c0;
pub const SET_SHADER_LOCAL_MEMORY_WINDOW_A: usize = 0x07b0;
pub const SET_SHADER_LOCAL_MEMORY_WINDOW_B: usize = 0x07b4;
pub const SET_TEX_SAMPLER_POOL_A: usize = 0x155c;
pub const SET_TEX_SAMPLER_POOL_B: usize = 0x1560;
pub const SET_TEX_SAMPLER_POOL_C: usize = 0x1564;
pub const SET_TEX_HEADER_POOL_A: usize = 0x1574;
pub const SET_TEX_HEADER_POOL_B: usize = 0x1578;
pub const SET_TEX_HEADER_POOL_C: usize = 0x157c;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstantBuffer {
    pub address: u64,
    pub size: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DescriptorPool {
    pub address: u64,
    pub maximum_index: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReleaseSemaphore {
    pub address: u64,
    pub payload: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchSnapshot {
    pub program_address: u64,
    pub grid: [u32; 3],
    pub block: [u32; 3],
    pub shared_memory_size: u32,
    pub constant_buffers: [Option<ConstantBuffer>; 8],
    pub release_semaphores: [Option<ReleaseSemaphore>; 2],
    pub barrier_count: u8,
    pub register_count: u8,
    pub sass_version: u8,
    pub shared_memory_window: u64,
    pub local_memory_window: u64,
    pub sampler_pool: DescriptorPool,
    pub texture_pool: DescriptorPool,
}

pub struct ComputeClass {
    words: Box<[u32; REGISTER_BYTES / 4]>,
}

impl Default for ComputeClass {
    fn default() -> Self {
        Self {
            words: Box::new([0; REGISTER_BYTES / 4]),
        }
    }
}

impl ComputeClass {
    pub fn raw(&self, method: u16) -> u32 {
        self.words[method as usize]
    }

    pub(crate) fn write<'a>(
        &mut self,
        method: u16,
        value: u32,
        fetch: &mut impl FnMut(u64, usize) -> Result<&'a [u8], String>,
    ) -> Result<Option<DispatchSnapshot>, String> {
        let byte = method as usize * 4;
        if byte >= REGISTER_BYTES {
            return Err(format!("C7C0 method {byte:#06x} outside register file"));
        }
        self.words[method as usize] = value;
        match byte {
            SEND_SIGNALING_PCAS_B => {
                let address = (self.word(SEND_PCAS_A) as u64) << 8;
                let bytes = fetch(address, 256)?;
                if bytes.len() != 256 {
                    return Err(format!(
                        "QMD fetch returned {} bytes, expected 256",
                        bytes.len()
                    ));
                }
                Ok(Some(self.decode_qmd(bytes)?))
            }
            _ => Ok(None),
        }
    }

    fn decode_qmd(&self, bytes: &[u8]) -> Result<DispatchSnapshot, String> {
        let mut words = [0; 64];
        for (word, bytes) in words.iter_mut().zip(bytes.chunks_exact(4)) {
            *word = u32::from_le_bytes(bytes.try_into().unwrap());
        }
        // QMDV02_03 layout from clc7c0qmd.h:145, QMD_VERSION (MW(579:576)).
        let version = field(&words, 576, 4);
        let major = field(&words, 580, 4);
        if (major, version) != (2, 3) {
            return Err(format!(
                "unsupported C7C0 QMD version QMDV{major:02}_{version:02}"
            ));
        }
        let valid = field(&words, 640, 8);
        let mut constant_buffers = [None; 8];
        for (i, buffer) in constant_buffers.iter_mut().enumerate() {
            if valid & (1 << i) != 0 {
                let base = 1024 + i * 64;
                *buffer = Some(ConstantBuffer {
                    address: field(&words, base, 32) as u64
                        | (field(&words, base + 32, 17) as u64) << 32,
                    size: field(&words, base + 51, 13) << 4,
                });
            }
        }
        let releases = [138, 139].map(|enable| {
            (field(&words, enable, 1) != 0).then(|| {
                let slot = enable - 138;
                let base = 736 + slot * 96;
                ReleaseSemaphore {
                    address: field(&words, base, 32) as u64
                        | (field(&words, base + 32, 8) as u64) << 32,
                    payload: field(&words, base + 64, 32),
                }
            })
        });
        Ok(DispatchSnapshot {
            program_address: field(&words, 1536, 32) as u64
                | (field(&words, 1568, 17) as u64) << 32,
            grid: [
                field(&words, 384, 32),
                field(&words, 416, 16),
                field(&words, 448, 16),
            ],
            block: [
                field(&words, 592, 16),
                field(&words, 608, 16),
                field(&words, 624, 16),
            ],
            shared_memory_size: field(&words, 544, 18),
            constant_buffers,
            release_semaphores: releases,
            barrier_count: field(&words, 955, 5) as u8,
            register_count: field(&words, 984, 8) as u8,
            sass_version: field(&words, 1016, 8) as u8,
            shared_memory_window: self.address_pair(
                SET_SHADER_SHARED_MEMORY_WINDOW_A,
                SET_SHADER_SHARED_MEMORY_WINDOW_B,
            ),
            local_memory_window: self.address_pair(
                SET_SHADER_LOCAL_MEMORY_WINDOW_A,
                SET_SHADER_LOCAL_MEMORY_WINDOW_B,
            ),
            sampler_pool: self.pool(
                SET_TEX_SAMPLER_POOL_A,
                SET_TEX_SAMPLER_POOL_B,
                SET_TEX_SAMPLER_POOL_C,
                0x000f_ffff,
            ),
            texture_pool: self.pool(
                SET_TEX_HEADER_POOL_A,
                SET_TEX_HEADER_POOL_B,
                SET_TEX_HEADER_POOL_C,
                0x003f_ffff,
            ),
        })
    }

    fn word(&self, byte: usize) -> u32 {
        self.words[byte / 4]
    }

    fn address_pair(&self, upper: usize, lower: usize) -> u64 {
        ((self.word(upper) & 0x1ffff) as u64) << 32 | self.word(lower) as u64
    }

    fn pool(&self, upper: usize, lower: usize, maximum: usize, mask: u32) -> DescriptorPool {
        DescriptorPool {
            address: self.address_pair(upper, lower),
            maximum_index: self.word(maximum) & mask,
        }
    }
}

fn field(words: &[u32; 64], bit: usize, width: usize) -> u32 {
    (words[bit / 32] >> (bit % 32))
        & if width == 32 {
            u32::MAX
        } else {
            (1 << width) - 1
        }
}
