//! Guest shader programs in game files: NintendoWare BNSH containers (layout from maierfelix/bnsh-decoder BNSH.ksy),
//! and a raw scan for program headers elsewhere.
use super::VkState;
use super::sm86::{ResourceKind, Stage, branch_target, emit_module};
use std::panic::{AssertUnwindSafe, catch_unwind};

const SPH_BYTES: usize = 0x80;
const POOL_ENTRY_MAGIC: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
const POOL_ENTRY_SPH: usize = 0x30;

pub struct Program {
    pub stage: Stage,
    pub sph: [u32; 32],
    pub code: Vec<u8>,
}

impl Program {
    pub fn words(&self) -> Vec<u128> {
        self.code.chunks_exact(16).map(|w| u128::from_le_bytes(w.try_into().unwrap())).collect()
    }
}

/// The stage `Gpu::module` translates a program with this header as, before draw state narrows it.
fn stage(sph: &[u32; 32]) -> Option<Stage> {
    let field = |lo: usize, bits: u32| sph[lo / 32] >> (lo % 32) & ((1u64 << bits) - 1) as u32;
    let interpolated = |mode: u32| (0..32).filter(|&i| (0..4).any(|c| field(192 + 8 * i + 2 * c, 2) == mode)).fold(0, |m, i| m | 1 << i);
    match (field(0, 5), field(5, 5), field(10, 4)) {
        (1, 4, 1) => Some(Stage::Vertex { uint: 0, sint: 0 }),
        (2, 4, 5) if field(576, 32) == 0xf => Some(Stage::Fragment { flat: interpolated(1), screen_linear: interpolated(3), depth: field(609, 1) != 0 }),
        _ => None,
    }
}

/// The program whose header starts at `at`, its code running to the self-branch compiled programs end with, or to `end`.
fn program(file: &[u8], at: usize, end: usize) -> Option<Program> {
    let header = file.get(at..at + SPH_BYTES)?;
    let sph = std::array::from_fn(|i| u32::from_le_bytes(header[4 * i..4 * i + 4].try_into().unwrap()));
    let stage = stage(&sph)?;
    let start = at + SPH_BYTES;
    let mut len = 0;
    for (pc, word) in (0..).zip(file.get(start..end)?.chunks_exact(16)) {
        len += 16;
        let inst = u128::from_le_bytes(word.try_into().unwrap());
        if inst >> 12 & 0xf == 7 && branch_target(pc, inst) == Some(pc) {
            break;
        }
    }
    (start.is_multiple_of(16) && len > 0).then(|| Program { stage, sph, code: file[start..start + len].to_vec() })
}

/// Every program in the shader binary pool of a BNSH file. Its offsets are file-relative, so the relocation table is not needed.
pub fn programs(file: &[u8]) -> Vec<Program> {
    let int = |at: usize, len: usize| file.get(at..at + len).map(|b| b.iter().rev().fold(0, |v, &b| v << 8 | b as usize));
    let pool = || {
        let grsc = int(0x16, 2)?;
        let pool = (file.get(grsc..grsc + 4)? == b"grsc").then(|| int(grsc + 0x28, 8))??;
        let memory = int(pool + 8, 8)?;
        Some(memory..memory.checked_add(int(pool + 4, 4)?)?.min(file.len()))
    };
    let Some(pool) = file.starts_with(b"BNSH").then(pool).flatten() else { return Vec::new() };
    let entries = pool.clone().step_by(16).filter(|&at| file[at..].starts_with(&POOL_ENTRY_MAGIC));
    entries.filter_map(|at| program(file, at + POOL_ENTRY_SPH, pool.end)).collect()
}

/// Program headers anywhere in `file` whose program translates: candidates for files no container parser knows.
pub fn scan(file: &[u8]) -> Vec<Program> {
    let candidates = (0..file.len().saturating_sub(SPH_BYTES)).step_by(16).filter_map(|at| program(file, at, file.len()));
    candidates.filter(|program| translate(None, program).is_some()).collect()
}

/// The SPIR-V, resources and whether it stores global memory, or `None` where the translator rejects `program`.
pub fn translate(device: Option<&VkState>, program: &Program) -> Option<(Vec<u32>, Vec<ResourceKind>, bool)> {
    catch_unwind(AssertUnwindSafe(|| {
        let (mut resources, mut stores_global) = (Vec::new(), false);
        let words = emit_module(device, program.stage, |decoder| {
            decoder.translate_program(&program.words());
            resources = decoder.resources.iter().map(|resource| resource.kind).collect();
            stores_global = decoder.stores_global;
        })
        .words();
        (words, resources, stores_global)
    }))
    .ok()
}
