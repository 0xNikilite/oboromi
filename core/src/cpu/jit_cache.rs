#![allow(dead_code)]

use super::decoder::decode;
use super::instructions::Instruction;
use crate::mmu::Memory;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct CachedInsn {
    pub raw: u32,
    pub decoded: Instruction,
}

/// AArch64 Basic-Block JIT / Pre-Decoded Instruction Cache.
///
/// Translates guest AArch64 basic blocks (contiguous sequences of instructions ending
/// at a control-flow branch or `SVC`) into pre-decoded `CachedInsn` entries on first
/// execution, eliminating redundant bitwise decoding inside hot loops.
#[derive(Debug, Default)]
pub struct JitBlockCache {
    entries: HashMap<u64, CachedInsn>,
    pub blocks_compiled: u64,
    pub instructions_compiled: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl JitBlockCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(4096),
            blocks_compiled: 0,
            instructions_compiled: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Looks up the pre-decoded instruction at `pc` (verifying `raw_word` for SMC safety).
    /// On a cache miss, compiles the entire basic block starting at `pc`.
    #[inline]
    pub fn fetch_or_compile(&mut self, pc: u64, raw_word: u32, mmu: &Memory) -> Instruction {
        if let Some(entry) = self.entries.get(&pc) {
            if entry.raw == raw_word {
                self.cache_hits += 1;
                return entry.decoded;
            }
        }

        self.cache_misses += 1;
        self.compile_basic_block(pc, mmu);

        self.entries
            .get(&pc)
            .map(|e| e.decoded)
            .unwrap_or_else(|| decode(raw_word))
    }

    /// Compiles a contiguous AArch64 basic block starting at `start_pc` up to the first
    /// branch (`B`, `B.cond`, `CBZ`, `RET`, `BLR`) or supervisor call (`SVC`), up to 64 instructions.
    pub fn compile_basic_block(&mut self, start_pc: u64, mmu: &Memory) {
        self.blocks_compiled += 1;
        let mut cur_pc = start_pc;

        for _ in 0..64 {
            let Ok(raw) = mmu.read_u32(cur_pc) else {
                break;
            };
            let decoded = decode(raw);
            self.entries.insert(cur_pc, CachedInsn { raw, decoded });
            self.instructions_compiled += 1;

            if Self::is_block_terminator(&decoded) {
                break;
            }
            cur_pc = cur_pc.wrapping_add(4);
        }
    }

    #[inline]
    fn is_block_terminator(inst: &Instruction) -> bool {
        matches!(
            inst,
            Instruction::BranchImm { .. }
                | Instruction::BranchCond { .. }
                | Instruction::CompareBranch { .. }
                | Instruction::BranchReg { .. }
                | Instruction::Svc { .. }
                | Instruction::Unimplemented { .. }
        )
    }

    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_jit_basic_block_compilation_and_hits() {
        let mut mmu = Memory::new();
        let mut jit = JitBlockCache::new();
        let base = 0x80000000u64;
        mmu.map_range(base, 4096, Permission::READ_WRITE_EXECUTE);

        // Write a 4-instruction basic block ending with B.NE
        let block: [u32; 4] = [
            0x91000421, // ADD  X1, X1, #1
            0xF1000463, // SUBS X3, X3, #1
            0x54FFFFC1, // B.NE -2
            0xD503201F, // NOP (outside basic block)
        ];
        for (i, &w) in block.iter().enumerate() {
            mmu.write_u32(base + (i * 4) as u64, w).unwrap();
        }

        // First fetch at `base` triggers a basic-block compile of all 3 instructions up to B.NE
        let _ = jit.fetch_or_compile(base, block[0], &mmu);
        assert_eq!(jit.blocks_compiled, 1);
        assert_eq!(jit.instructions_compiled, 3);
        assert_eq!(jit.cache_misses, 1);

        // Subsequent fetches of instructions 1 and 2 in the block are immediate JIT cache hits!
        let _ = jit.fetch_or_compile(base + 4, block[1], &mmu);
        let _ = jit.fetch_or_compile(base + 8, block[2], &mmu);
        assert_eq!(jit.cache_hits, 2);
        assert_eq!(jit.cache_misses, 1);
    }
}
