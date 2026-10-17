//! Kernels of nvcc-built sm_8x cubins, read from the ELF64 sections `.text.<kernel>`, `.nv.info.<kernel>`,
//! `.nv.constant<bank>[.<kernel>]` and `.nv.shared.<kernel>`, and the constant banks the CUDA driver launches them with.
use super::sm86::SHARED_WORDS;

const SHT_NOBITS: usize = 8;
const EIFMT_SVAL: usize = 4;
const EIATTR_PARAM_CBANK: usize = 0x0a;
const EIATTR_KPARAM_INFO: usize = 0x17;
const LOCAL_BYTES: u32 = 0x1000;

pub struct Kernel {
    pub words: Vec<u128>,
    /// Every constant bank the cubin initialises for the kernel in bank order, bank 0 first and ending in the parameter area.
    pub banks: Vec<(u32, Vec<u8>)>,
    /// Offset in bank 0 and size of each parameter, in parameter order.
    pub params: Vec<(usize, usize)>,
}

pub enum Arg<'a> {
    Value(&'a [u8]),
    Buffer(&'a [u8]),
}

fn le(bytes: &[u8], at: usize, len: usize) -> Result<usize, String> {
    let field = bytes.get(at..at + len).ok_or("truncated cubin")?;
    Ok(field.iter().rev().fold(0, |value, &byte| value << 8 | byte as usize))
}

impl Kernel {
    pub fn load(elf: &[u8], name: &str) -> Result<Self, String> {
        if elf.get(..5) != Some(b"\x7fELF\x02") {
            return Err("not an ELF64 cubin".into());
        }
        // ELF ABI version 8 (CUDA 12.8 on) moved the SM number from e_flags bits 0..8 to 8..16, LLVM ELF.h EF_CUDA_SM_OFFSET.
        let flags = le(elf, 0x30, 4)?;
        let sm = if le(elf, 8, 1)? >= 8 { flags >> 8 & 0xff } else { flags & 0xff };
        if !(80..90).contains(&sm) {
            return Err(format!("the cubin targets sm_{sm}, only sm_8x runs"));
        }
        let (shoff, shnum) = (le(elf, 0x28, 8)?, le(elf, 0x3c, 2)?);
        let names = le(elf, shoff + le(elf, 0x3e, 2)? * 64 + 24, 8)?;
        let sections = (shoff..shoff + shnum * 64).step_by(64).map(|at| {
            let name = elf.get(names + le(elf, at, 4)?..).and_then(|s| s.split(|&b| b == 0).next()).and_then(|s| std::str::from_utf8(s).ok());
            let (offset, size) = (le(elf, at + 24, 8)?, le(elf, at + 32, 8)?);
            let data = if le(elf, at + 4, 4)? == SHT_NOBITS { Some(&[][..]) } else { elf.get(offset..offset + size) };
            Ok((name.ok_or("bad section name")?, data.ok_or("truncated cubin")?, size))
        }).collect::<Result<Vec<_>, String>>()?;
        let section = |kind: &str| sections.iter().find(|s| s.0.strip_prefix(kind) == Some(&format!(".{name}")));
        let Some(&(_, text, _)) = section(".text") else {
            let kernels: Vec<_> = sections.iter().filter_map(|s| s.0.strip_prefix(".text.")).collect();
            return Err(format!("no kernel {name} in the cubin, it has {kernels:?}"));
        };
        let shared = section(".nv.shared").map_or(0, |s| s.2);
        assert!(shared <= 4 * SHARED_WORDS as usize, "{name} declares {shared} bytes of shared memory, the translator has {}", 4 * SHARED_WORDS);
        let info = section(".nv.info").ok_or(format!("{name} has no .nv.info"))?.1;
        let (mut base, mut params, mut at) = (0, Vec::new(), 0);
        while at < info.len() {
            let (attribute, payload) = (le(info, at + 1, 1)?, at + 4);
            // PARAM_CBANK is a symbol u32 then the area's offset u16; KPARAM_INFO an index u32, the ordinal u16, the
            // offset u16, then a u32 with the size in bits 18..32 (field order as cuobjdump -elf prints them).
            match attribute {
                EIATTR_PARAM_CBANK => base = le(info, payload + 4, 2)?,
                EIATTR_KPARAM_INFO => params.push((le(info, payload + 4, 2)?, le(info, payload + 6, 2)?, le(info, payload + 8, 4)? >> 18)),
                _ => {}
            }
            at = payload + if le(info, at, 1)? == EIFMT_SVAL { le(info, at + 2, 2)? } else { 0 };
        }
        params.sort_unstable();
        let mut banks: Vec<(u32, Vec<u8>)> = sections.iter().filter_map(|&(section, data, _)| {
            let rest = section.strip_prefix(".nv.constant")?;
            let (bank, owner) = rest.split_once('.').unwrap_or((rest, name));
            Some((bank.parse().ok()?, data.to_vec())).filter(|_| owner == name)
        }).collect();
        banks.sort_unstable_by_key(|bank| bank.0);
        if banks.first().is_none_or(|bank| bank.0 != 0) {
            return Err(format!("{name} has no .nv.constant0"));
        }
        Ok(Self {
            words: text.chunks_exact(16).map(|word| u128::from_le_bytes(word.try_into().unwrap())).collect(),
            banks,
            params: params.into_iter().map(|(_, offset, size)| (base + offset, size)).collect(),
        })
    }

    /// The constant banks for a `grid` x `block` launch with `args` in parameter order, and the (address, bytes)
    /// regions of global memory it needs: each buffer is placed from `base` up and passed as its address.
    pub fn launch<'a>(&self, grid: [u32; 3], block: [u32; 3], args: &[Arg<'a>], base: u64) -> (Vec<(u32, Vec<u8>)>, Vec<(u64, &'a [u8])>) {
        assert_eq!(args.len(), self.params.len(), "the kernel takes {} parameters", self.params.len());
        let mut banks = self.banks.clone();
        let cbuf0 = &mut banks[0].1;
        // The sm_8x driver's bank 0, from the sass-rig program fixtures (make_program_fixture.py, RUN.md session 5): ntid
        // at 0x0, nctaid at 0xc, the initial stack pointer R1 at 0x28 (0x00fffdc0 on the 3090, here the top of the
        // translator's 4 KiB local array), the memory descriptor at 0x118 (zero) and the parameters from 0x160.
        for (i, dim) in block.iter().chain(&grid).enumerate() {
            cbuf0[4 * i..4 * i + 4].copy_from_slice(&dim.to_le_bytes());
        }
        cbuf0[0x28..0x2c].copy_from_slice(&LOCAL_BYTES.to_le_bytes());
        let (mut regions, mut address) = (Vec::new(), base);
        for (&(offset, size), arg) in self.params.iter().zip(args) {
            let value = match *arg {
                Arg::Value(bytes) => bytes.to_vec(),
                Arg::Buffer(bytes) => {
                    regions.push((address, bytes));
                    let pointer = address.to_le_bytes().to_vec();
                    // cudaMalloc aligns every allocation to 256 bytes.
                    address = (address + bytes.len() as u64).next_multiple_of(256);
                    pointer
                }
            };
            assert_eq!(value.len(), size, "parameter at c[0x0][{offset:#x}] is {size} bytes");
            cbuf0[offset..offset + size].copy_from_slice(&value);
        }
        (banks, regions)
    }
}
