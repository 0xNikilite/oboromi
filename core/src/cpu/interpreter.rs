#![allow(dead_code)]

use super::instructions::{BranchRegOp, Condition, FpOp2, Instruction, LogicalOp, MoveWideOp, ShiftType};
use super::jit_cache::JitBlockCache;
use super::registers::CpuRegisters;
use crate::mmu::{Memory, MemoryError};

#[derive(Debug, thiserror::Error)]
pub enum CpuError {
    #[error("Memory access violation: {0}")]
    Memory(#[from] MemoryError),

    #[error("Unimplemented instruction 0x{raw:08X} at PC: 0x{pc:016X}")]
    UnimplementedInstruction { raw: u32, pc: u64 },

    #[error("CPU halted: {0}")]
    Halted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepResult {
    /// Normal instruction completed
    Continue,
    /// Horizon OS Supervisor Call triggered (SVC #imm)
    Svc(u16),
    /// Process exited or CPU halted
    Halt(String),
}

/// AArch64 Execution Engine with Basic-Block JIT Instruction Cache.
pub struct Cpu {
    pub regs: CpuRegisters,
    pub jit: JitBlockCache,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: CpuRegisters::new(),
            jit: JitBlockCache::new(),
        }
    }

    /// Fetches, basic-block JIT-compiles/looks up, and executes a single AArch64 instruction.
    pub fn step(&mut self, mmu: &mut Memory) -> Result<StepResult, CpuError> {
        let pc = self.regs.pc;
        let raw_inst = mmu.read_u32(pc)?;
        let inst = self.jit.fetch_or_compile(pc, raw_inst, mmu);

        match inst {
            Instruction::Nop => {
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::Svc { imm } => {
                // Return control to Horizon kernel dispatcher
                self.regs.advance_pc();
                Ok(StepResult::Svc(imm))
            }

            Instruction::Adr { rd, imm, is_page } => {
                let base = if is_page { pc & !0xFFF } else { pc };
                let target = (base as i64).wrapping_add(imm) as u64;
                self.regs.write_x(rd, target);
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::MoveWide {
                rd,
                imm,
                shift,
                op,
                is_64,
            } => {
                let shifted = (imm as u64) << shift;
                let val = match op {
                    MoveWideOp::Movz => shifted,
                    MoveWideOp::Movn => {
                        let mask = if is_64 { u64::MAX } else { u32::MAX as u64 };
                        (!shifted) & mask
                    }
                    MoveWideOp::Movk => {
                        let cur = if is_64 {
                            self.regs.read_x(rd)
                        } else {
                            self.regs.read_w(rd) as u64
                        };
                        let mask = !((0xFFFF as u64) << shift);
                        (cur & mask) | shifted
                    }
                };

                if is_64 {
                    self.regs.write_x(rd, val);
                } else {
                    self.regs.write_w(rd, val as u32);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::AddSubImm {
                rd,
                rn,
                imm,
                is_sub,
                set_flags,
                is_64,
            } => {
                // If setting flags or rd != 31, read Rn (with SP support if not setting flags)
                let op1 = if !set_flags && rn == 31 {
                    self.regs.sp
                } else {
                    self.regs.read_x(rn)
                };

                let op2 = imm;

                if is_64 {
                    let (res, c, v) = if is_sub {
                        let (res, borrow) = op1.overflowing_sub(op2);
                        let v = ((op1 ^ op2) & (op1 ^ res) & (1 << 63)) != 0;
                        (res, !borrow, v)
                    } else {
                        let (res, carry) = op1.overflowing_add(op2);
                        let v = (!((op1 ^ op2)) & (op1 ^ res) & (1 << 63)) != 0;
                        (res, carry, v)
                    };

                    if set_flags {
                        self.regs.pstate.n = (res as i64) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                        self.regs.write_x(rd, res);
                    } else {
                        self.regs.write_x_or_sp(rd, res);
                    }
                } else {
                    let op1_32 = op1 as u32;
                    let op2_32 = op2 as u32;

                    let (res, c, v) = if is_sub {
                        let (res, borrow) = op1_32.overflowing_sub(op2_32);
                        let v = ((op1_32 ^ op2_32) & (op1_32 ^ res) & (1 << 31)) != 0;
                        (res, !borrow, v)
                    } else {
                        let (res, carry) = op1_32.overflowing_add(op2_32);
                        let v = (!((op1_32 ^ op2_32)) & (op1_32 ^ res) & (1 << 31)) != 0;
                        (res, carry, v)
                    };

                    if set_flags {
                        self.regs.pstate.n = (res as i32) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                        self.regs.write_w(rd, res);
                    } else {
                        self.regs.write_x_or_sp(rd, res as u64);
                    }
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::AddSubReg {
                rd,
                rn,
                rm,
                shift_type,
                shift_amount,
                is_extended,
                extend_option,
                is_sub,
                set_flags,
                is_64,
            } => {
                let op1 = if is_extended && rn == 31 {
                    self.regs.sp
                } else {
                    self.regs.read_x(rn)
                };
                let raw_rm = self.regs.read_x(rm);
                let op2 = if is_extended {
                    let ext: u64 = match extend_option {
                        0b000 => (raw_rm as u8) as u64,                  // UXTB
                        0b001 => (raw_rm as u16) as u64,                 // UXTH
                        0b010 => (raw_rm as u32) as u64,                 // UXTW
                        0b011 => raw_rm,                                 // UXTX
                        0b100 => ((raw_rm as i8) as i64) as u64,         // SXTB
                        0b101 => ((raw_rm as i16) as i64) as u64,        // SXTH
                        0b110 => ((raw_rm as i32) as i64) as u64,        // SXTW
                        _ => raw_rm,                                     // SXTX
                    };
                    ext.wrapping_shl((shift_amount & 7) as u32)
                } else if shift_amount == 0 {
                    raw_rm
                } else if is_64 {
                    let sh = (shift_amount & 63) as u32;
                    match shift_type {
                        ShiftType::Lsl => raw_rm.wrapping_shl(sh),
                        ShiftType::Lsr => raw_rm.wrapping_shr(sh),
                        ShiftType::Asr => ((raw_rm as i64).wrapping_shr(sh)) as u64,
                        ShiftType::Ror => raw_rm.rotate_right(sh),
                    }
                } else {
                    let sh = (shift_amount & 31) as u32;
                    let rm32 = raw_rm as u32;
                    let shifted32 = match shift_type {
                        ShiftType::Lsl => rm32.wrapping_shl(sh),
                        ShiftType::Lsr => rm32.wrapping_shr(sh),
                        ShiftType::Asr => ((rm32 as i32).wrapping_shr(sh)) as u32,
                        ShiftType::Ror => rm32.rotate_right(sh),
                    };
                    shifted32 as u64
                };

                if is_64 {
                    let (res, c, v) = if is_sub {
                        let (res, borrow) = op1.overflowing_sub(op2);
                        let v = ((op1 ^ op2) & (op1 ^ res) & (1 << 63)) != 0;
                        (res, !borrow, v)
                    } else {
                        let (res, carry) = op1.overflowing_add(op2);
                        let v = (!((op1 ^ op2)) & (op1 ^ res) & (1 << 63)) != 0;
                        (res, carry, v)
                    };

                    if set_flags {
                        self.regs.pstate.n = (res as i64) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                    }
                    self.regs.write_x(rd, res);
                } else {
                    let op1_32 = op1 as u32;
                    let op2_32 = op2 as u32;

                    let (res, c, v) = if is_sub {
                        let (res, borrow) = op1_32.overflowing_sub(op2_32);
                        let v = ((op1_32 ^ op2_32) & (op1_32 ^ res) & (1 << 31)) != 0;
                        (res, !borrow, v)
                    } else {
                        let (res, carry) = op1_32.overflowing_add(op2_32);
                        let v = (!((op1_32 ^ op2_32)) & (op1_32 ^ res) & (1 << 31)) != 0;
                        (res, carry, v)
                    };

                    if set_flags {
                        self.regs.pstate.n = (res as i32) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                    }
                    self.regs.write_w(rd, res);
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::LogicalReg {
                rd,
                rn,
                rm,
                shift_type,
                shift_amount,
                invert_rm,
                op,
                set_flags,
                is_64,
            } => {
                let op1 = self.regs.read_x(rn);
                let raw_rm = self.regs.read_x(rm);
                let shifted = if shift_amount == 0 {
                    raw_rm
                } else if is_64 {
                    let sh = (shift_amount & 63) as u32;
                    match shift_type {
                        ShiftType::Lsl => raw_rm.wrapping_shl(sh),
                        ShiftType::Lsr => raw_rm.wrapping_shr(sh),
                        ShiftType::Asr => ((raw_rm as i64).wrapping_shr(sh)) as u64,
                        ShiftType::Ror => raw_rm.rotate_right(sh),
                    }
                } else {
                    let sh = (shift_amount & 31) as u32;
                    let rm32 = raw_rm as u32;
                    let shifted32 = match shift_type {
                        ShiftType::Lsl => rm32.wrapping_shl(sh),
                        ShiftType::Lsr => rm32.wrapping_shr(sh),
                        ShiftType::Asr => ((rm32 as i32).wrapping_shr(sh)) as u32,
                        ShiftType::Ror => rm32.rotate_right(sh),
                    };
                    shifted32 as u64
                };
                let op2 = if invert_rm { !shifted } else { shifted };

                let res = match op {
                    LogicalOp::And | LogicalOp::Ands => op1 & op2,
                    LogicalOp::Orr => op1 | op2,
                    LogicalOp::Eor => op1 ^ op2,
                };

                if is_64 {
                    self.regs.write_x(rd, res);
                    if set_flags {
                        self.regs.pstate.n = (res as i64) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = false;
                        self.regs.pstate.v = false;
                    }
                } else {
                    let res32 = res as u32;
                    self.regs.write_w(rd, res32);
                    if set_flags {
                        self.regs.pstate.n = (res32 as i32) < 0;
                        self.regs.pstate.z = res32 == 0;
                        self.regs.pstate.c = false;
                        self.regs.pstate.v = false;
                    }
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::LogicalImm {
                rd,
                rn,
                imm,
                op,
                set_flags,
                is_64,
            } => {
                let op1 = self.regs.read_x(rn);
                let res = match op {
                    LogicalOp::And | LogicalOp::Ands => op1 & imm,
                    LogicalOp::Orr => op1 | imm,
                    LogicalOp::Eor => op1 ^ imm,
                };
                if is_64 {
                    if set_flags {
                        self.regs.pstate.n = (res as i64) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = false;
                        self.regs.pstate.v = false;
                        self.regs.write_x(rd, res);
                    } else {
                        self.regs.write_x_or_sp(rd, res);
                    }
                } else {
                    let res32 = res as u32;
                    if set_flags {
                        self.regs.pstate.n = (res32 as i32) < 0;
                        self.regs.pstate.z = res32 == 0;
                        self.regs.pstate.c = false;
                        self.regs.pstate.v = false;
                        self.regs.write_w(rd, res32);
                    } else {
                        self.regs.write_x_or_sp(rd, res32 as u64);
                    }
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::Bitfield {
                rd,
                rn,
                immr,
                imms,
                opc,
                is_64,
            } => {
                let src = if is_64 {
                    self.regs.read_x(rn)
                } else {
                    self.regs.read_w(rn) as u64
                };
                let dst = if is_64 {
                    self.regs.read_x(rd)
                } else {
                    self.regs.read_w(rd) as u64
                };
                let bits: u8 = if is_64 { 64 } else { 32 };
                let r = immr % bits;
                let s = imms % bits;

                let res = if s >= r {
                    // Extract bits [s:r] -> [s-r:0]
                    let width = (s - r + 1) as u32;
                    let mask = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
                    let extracted = (src >> r) & mask;
                    match opc {
                        0 => {
                            // SBFM: sign-extend from bit (width - 1)
                            let shift = 64 - width;
                            (((extracted << shift) as i64) >> shift) as u64
                        }
                        1 => {
                            // BFM: bitfield extract and insert into low bits of Rd
                            (dst & !mask) | extracted
                        }
                        _ => extracted, // UBFM
                    }
                } else {
                    // Left shift / insert: shift left by (bits - r), width = s + 1
                    let lshift = (bits - r) as u32;
                    let width = (s + 1) as u32;
                    let mask = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
                    let extracted = src & mask;
                    let positioned = extracted.wrapping_shl(lshift);
                    match opc {
                        0 => {
                            // SBFIZ
                            let sign_bit = (extracted >> s) & 1;
                            if sign_bit != 0 {
                               let top_shift = lshift + width;
                               let top_mask = if top_shift >= 64 { 0 } else { u64::MAX << top_shift };
                               positioned | top_mask
                            } else {
                               positioned
                            }
                        }
                        1 => {
                            // BFI: insert into dst at [lshift + width - 1 : lshift]
                            let dst_mask = mask.wrapping_shl(lshift);
                            (dst & !dst_mask) | positioned
                        }
                        _ => positioned, // UBFIZ / LSL #imm
                    }
                };

                if is_64 {
                    self.regs.write_x(rd, res);
                } else {
                    self.regs.write_w(rd, res as u32);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::Extract {
                rd,
                rn,
                rm,
                lsb,
                is_64,
            } => {
                if is_64 {
                    let hi = self.regs.read_x(rn) as u128;
                    let lo = self.regs.read_x(rm) as u128;
                    let combined = (hi << 64) | lo;
                    let res = ((combined >> (lsb & 63)) & 0xFFFF_FFFF_FFFF_FFFF) as u64;
                    self.regs.write_x(rd, res);
                } else {
                    let hi = (self.regs.read_w(rn) as u64) << 32;
                    let lo = self.regs.read_w(rm) as u64;
                    let res = ((hi | lo) >> (lsb & 31)) as u32;
                    self.regs.write_w(rd, res);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::CondSelect {
                rd,
                rn,
                rm,
                cond,
                op,
                is_64,
            } => {
                let res = if self.check_condition(cond) {
                    self.regs.read_x(rn)
                } else {
                    let m = self.regs.read_x(rm);
                    match op {
                        0 => m,                   // CSEL
                        1 => m.wrapping_add(1),   // CSINC
                        2 => !m,                  // CSINV
                        _ => m.wrapping_neg(),    // CSNEG
                    }
                };
                if is_64 {
                    self.regs.write_x(rd, res);
                } else {
                    self.regs.write_w(rd, res as u32);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::CondCompare {
                rn,
                rm_or_imm,
                nzcv,
                cond,
                is_imm,
                is_sub,
                is_64,
            } => {
                if self.check_condition(cond) {
                    let op1 = self.regs.read_x(rn);
                    let op2 = if is_imm {
                        rm_or_imm as u64
                    } else {
                        self.regs.read_x(rm_or_imm)
                    };
                    if is_64 {
                        let (res, c, v) = if is_sub {
                            let (r, borrow) = op1.overflowing_sub(op2);
                            let v = ((op1 ^ op2) & (op1 ^ r) & (1 << 63)) != 0;
                            (r, !borrow, v)
                        } else {
                            let (r, carry) = op1.overflowing_add(op2);
                            let v = (!((op1 ^ op2)) & (op1 ^ r) & (1 << 63)) != 0;
                            (r, carry, v)
                        };
                        self.regs.pstate.n = (res as i64) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                    } else {
                        let op1_32 = op1 as u32;
                        let op2_32 = op2 as u32;
                        let (res, c, v) = if is_sub {
                            let (r, borrow) = op1_32.overflowing_sub(op2_32);
                            let v = ((op1_32 ^ op2_32) & (op1_32 ^ r) & (1 << 31)) != 0;
                            (r, !borrow, v)
                        } else {
                            let (r, carry) = op1_32.overflowing_add(op2_32);
                            let v = (!((op1_32 ^ op2_32)) & (op1_32 ^ r) & (1 << 31)) != 0;
                            (r, carry, v)
                        };
                        self.regs.pstate.n = (res as i32) < 0;
                        self.regs.pstate.z = res == 0;
                        self.regs.pstate.c = c;
                        self.regs.pstate.v = v;
                    }
                } else {
                    self.regs.pstate.n = (nzcv & 0b1000) != 0;
                    self.regs.pstate.z = (nzcv & 0b0100) != 0;
                    self.regs.pstate.c = (nzcv & 0b0010) != 0;
                    self.regs.pstate.v = (nzcv & 0b0001) != 0;
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::TestBranch {
                rt,
                bit,
                offset,
                is_tbnz,
            } => {
                let val = self.regs.read_x(rt);
                let bit_set = ((val >> (bit & 63)) & 1) != 0;
                if bit_set == is_tbnz {
                    self.regs.pc = (pc as i64).wrapping_add(offset) as u64;
                } else {
                    self.regs.advance_pc();
                }
                Ok(StepResult::Continue)
            }

            Instruction::LoadStoreReg {
                rt,
                rn,
                rm,
                option,
                shift,
                size_bytes,
                is_load,
                is_signed,
                is_64_target,
            } => {
                let base = self.regs.read_x_or_sp(rn);
                let raw_m = self.regs.read_x(rm);
                let index: u64 = match option {
                    0b010 => (raw_m as u32) as u64,               // UXTW
                    0b110 => ((raw_m as i32) as i64) as u64,      // SXTW
                    0b111 => raw_m,                               // SXTX
                    _ => raw_m,                                   // LSL / UXTX
                };
                let eff_addr = base.wrapping_add(index.wrapping_shl(shift as u32));

                if is_load {
                    let val = match (size_bytes, is_signed) {
                        (1, false) => mmu.read_u8(eff_addr)? as u64,
                        (1, true) => ((mmu.read_u8(eff_addr)? as i8) as i64) as u64,
                        (2, false) => mmu.read_u16(eff_addr)? as u64,
                        (2, true) => ((mmu.read_u16(eff_addr)? as i16) as i64) as u64,
                        (4, false) => mmu.read_u32(eff_addr)? as u64,
                        (4, true) => ((mmu.read_u32(eff_addr)? as i32) as i64) as u64,
                        (8, _) => mmu.read_u64(eff_addr)?,
                        _ => unreachable!(),
                    };
                    if is_64_target || size_bytes == 8 {
                        self.regs.write_x(rt, val);
                    } else {
                        self.regs.write_w(rt, val as u32);
                    }
                } else {
                    match size_bytes {
                        1 => mmu.write_u8(eff_addr, self.regs.read_w(rt) as u8)?,
                        2 => mmu.write_u16(eff_addr, self.regs.read_w(rt) as u16)?,
                        4 => mmu.write_u32(eff_addr, self.regs.read_w(rt))?,
                        8 => mmu.write_u64(eff_addr, self.regs.read_x(rt))?,
                        _ => unreachable!(),
                    }
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::BranchImm { offset, link } => {
                if link {
                    self.regs.write_x(30, pc.wrapping_add(4));
                }
                self.regs.pc = (pc as i64).wrapping_add(offset) as u64;
                Ok(StepResult::Continue)
            }

            Instruction::BranchCond { offset, cond } => {
                if self.check_condition(cond) {
                    self.regs.pc = (pc as i64).wrapping_add(offset) as u64;
                } else {
                    self.regs.advance_pc();
                }
                Ok(StepResult::Continue)
            }

            Instruction::BranchReg { rn, op } => {
                let target = self.regs.read_x(rn);
                match op {
                    BranchRegOp::Br => {
                        self.regs.pc = target;
                    }
                    BranchRegOp::Blr => {
                        self.regs.write_x(30, pc.wrapping_add(4));
                        self.regs.pc = target;
                    }
                    BranchRegOp::Ret => {
                        self.regs.pc = target;
                    }
                }
                Ok(StepResult::Continue)
            }

            Instruction::CompareBranch {
                rt,
                offset,
                is_cbnz,
                is_64,
            } => {
                let val = if is_64 {
                    self.regs.read_x(rt)
                } else {
                    self.regs.read_w(rt) as u64
                };

                let condition_met = if is_cbnz { val != 0 } else { val == 0 };

                if condition_met {
                    self.regs.pc = (pc as i64).wrapping_add(offset) as u64;
                } else {
                    self.regs.advance_pc();
                }
                Ok(StepResult::Continue)
            }

            Instruction::LoadStoreImm {
                rt,
                rn,
                offset,
                size_bytes,
                is_load,
                is_signed,
                writeback,
                post_index,
            } => {
                let base = self.regs.read_x_or_sp(rn);
                let eff_addr = if post_index {
                    base
                } else {
                    (base as i64).wrapping_add(offset) as u64
                };

                if is_load {
                    match (size_bytes, is_signed) {
                        (1, false) => self.regs.write_x(rt, mmu.read_u8(eff_addr)? as u64),
                        (1, true) => self.regs.write_x(rt, (mmu.read_u8(eff_addr)? as i8 as i64) as u64),
                        (2, false) => self.regs.write_x(rt, mmu.read_u16(eff_addr)? as u64),
                        (2, true) => self.regs.write_x(rt, (mmu.read_u16(eff_addr)? as i16 as i64) as u64),
                        (4, false) => self.regs.write_w(rt, mmu.read_u32(eff_addr)?),
                        (4, true) => self.regs.write_x(rt, (mmu.read_u32(eff_addr)? as i32 as i64) as u64),
                        (8, _) => self.regs.write_x(rt, mmu.read_u64(eff_addr)?),
                        _ => unreachable!(),
                    }
                } else {
                    match size_bytes {
                        1 => mmu.write_u8(eff_addr, self.regs.read_w(rt) as u8)?,
                        2 => mmu.write_u16(eff_addr, self.regs.read_w(rt) as u16)?,
                        4 => mmu.write_u32(eff_addr, self.regs.read_w(rt))?,
                        8 => mmu.write_u64(eff_addr, self.regs.read_x(rt))?,
                        _ => unreachable!(),
                    }
                }

                if writeback || post_index {
                    let updated = (base as i64).wrapping_add(offset) as u64;
                    self.regs.write_x_or_sp(rn, updated);
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::LoadStorePair {
                rt1,
                rt2,
                rn,
                offset,
                is_load,
                is_64,
                writeback,
                post_index,
            } => {
                let base = self.regs.read_x_or_sp(rn);
                let stride = if is_64 { 8 } else { 4 };
                let eff_addr = if post_index {
                    base
                } else {
                    (base as i64).wrapping_add(offset) as u64
                };

                if is_load {
                    if is_64 {
                        let v1 = mmu.read_u64(eff_addr)?;
                        let v2 = mmu.read_u64(eff_addr.wrapping_add(stride))?;
                        self.regs.write_x(rt1, v1);
                        self.regs.write_x(rt2, v2);
                    } else {
                        let v1 = mmu.read_u32(eff_addr)?;
                        let v2 = mmu.read_u32(eff_addr.wrapping_add(stride))?;
                        self.regs.write_w(rt1, v1);
                        self.regs.write_w(rt2, v2);
                    }
                } else if is_64 {
                    mmu.write_u64(eff_addr, self.regs.read_x(rt1))?;
                    mmu.write_u64(eff_addr.wrapping_add(stride), self.regs.read_x(rt2))?;
                } else {
                    mmu.write_u32(eff_addr, self.regs.read_w(rt1))?;
                    mmu.write_u32(eff_addr.wrapping_add(stride), self.regs.read_w(rt2))?;
                }

                if writeback || post_index {
                    let updated = (base as i64).wrapping_add(offset) as u64;
                    self.regs.write_x_or_sp(rn, updated);
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::SystemReg {
                rt,
                sysreg,
                is_read,
            } => {
                // System registers:
                // 0x5E82 = TPIDR_EL0   (S3_3_C13_C0_2)
                // 0x5E83 = TPIDRRO_EL0 (S3_3_C13_C0_3) - Horizon TLS Pointer
                // 0x5A10 = NZCV        (S3_3_C4_C2_0)
                // 0x5F00 = CNTFRQ_EL0  (19,200,000 Hz Switch 2 hardware timer frequency)
                // 0x5F01 = CNTPCT_EL0  (Switch 2 physical timer counter)
                if is_read {
                    let val = match sysreg {
                        0x5E82 => self.regs.tpidr_el0,
                        0x5E83 => self.regs.tpidrro_el0,
                        0x5A10 => self.regs.pstate.to_bits() as u64,
                        0x5F00 => 19_200_000,
                        0x5F01 => self.jit.cache_hits.wrapping_add(self.jit.cache_misses).wrapping_mul(16).wrapping_add(0x0123_4560),
                        _ => 0,
                    };
                    self.regs.write_x(rt, val);
                } else {
                    let val = self.regs.read_x(rt);
                    match sysreg {
                        0x5E82 => self.regs.tpidr_el0 = val,
                        0x5E83 => self.regs.tpidrro_el0 = val,
                        0x5A10 => self.regs.pstate.from_bits(val as u32),
                        _ => {}
                    }
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::FpData2 {
                rd,
                rn,
                rm,
                op,
                is_double,
            } => {
                if is_double {
                    let n = self.regs.read_d(rn);
                    let m = self.regs.read_d(rm);
                    let res = match op {
                        FpOp2::Fadd => n + m,
                        FpOp2::Fsub => n - m,
                        FpOp2::Fmul => n * m,
                        FpOp2::Fdiv => n / m,
                        FpOp2::Fmax => n.max(m),
                        FpOp2::Fmin => n.min(m),
                    };
                    self.regs.write_d(rd, res);
                } else {
                    let n = self.regs.read_s(rn);
                    let m = self.regs.read_s(rm);
                    let res = match op {
                        FpOp2::Fadd => n + m,
                        FpOp2::Fsub => n - m,
                        FpOp2::Fmul => n * m,
                        FpOp2::Fdiv => n / m,
                        FpOp2::Fmax => n.max(m),
                        FpOp2::Fmin => n.min(m),
                    };
                    self.regs.write_s(rd, res);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::FpCompare {
                rn,
                rm,
                is_zero: _,
                is_double,
            } => {
                let (n, m) = if is_double {
                    (self.regs.read_d(rn), self.regs.read_d(rm))
                } else {
                    (self.regs.read_s(rn) as f64, self.regs.read_s(rm) as f64)
                };

                if n.is_nan() || m.is_nan() {
                    self.regs.pstate.n = false;
                    self.regs.pstate.z = false;
                    self.regs.pstate.c = true;
                    self.regs.pstate.v = true;
                } else if n == m {
                    self.regs.pstate.n = false;
                    self.regs.pstate.z = true;
                    self.regs.pstate.c = true;
                    self.regs.pstate.v = false;
                } else if n < m {
                    self.regs.pstate.n = true;
                    self.regs.pstate.z = false;
                    self.regs.pstate.c = false;
                    self.regs.pstate.v = false;
                } else {
                    // n > m
                    self.regs.pstate.n = false;
                    self.regs.pstate.z = false;
                    self.regs.pstate.c = true;
                    self.regs.pstate.v = false;
                }

                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::FpMovGen {
                rd,
                rn,
                to_fp,
                is_64,
            } => {
                if to_fp {
                    if is_64 {
                        self.regs.write_d(rd, f64::from_bits(self.regs.read_x(rn)));
                    } else {
                        self.regs.write_s(rd, f32::from_bits(self.regs.read_w(rn)));
                    }
                } else if is_64 {
                    self.regs.write_x(rd, self.regs.read_d(rn).to_bits());
                } else {
                    self.regs.write_w(rd, self.regs.read_s(rn).to_bits());
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::FpLoadStoreImm {
                rt,
                rn,
                offset,
                is_load,
                is_double,
            } => {
                let base = self.regs.read_x_or_sp(rn);
                let eff_addr = (base as i64).wrapping_add(offset) as u64;
                if is_load {
                    if is_double {
                        let bits = mmu.read_u64(eff_addr)?;
                        self.regs.write_d(rt, f64::from_bits(bits));
                    } else {
                        let bits = mmu.read_u32(eff_addr)?;
                        self.regs.write_s(rt, f32::from_bits(bits));
                    }
                } else if is_double {
                    mmu.write_u64(eff_addr, self.regs.read_d(rt).to_bits())?;
                } else {
                    mmu.write_u32(eff_addr, self.regs.read_s(rt).to_bits())?;
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::MultiplyAddSub {
                rd,
                rn,
                rm,
                ra,
                is_sub,
                is_64,
            } => {
                if is_64 {
                    let n = self.regs.read_x(rn);
                    let m = self.regs.read_x(rm);
                    let a = self.regs.read_x(ra);
                    let prod = n.wrapping_mul(m);
                    let res = if is_sub {
                        a.wrapping_sub(prod)
                    } else {
                        a.wrapping_add(prod)
                    };
                    self.regs.write_x(rd, res);
                } else {
                    let n = self.regs.read_w(rn);
                    let m = self.regs.read_w(rm);
                    let a = self.regs.read_w(ra);
                    let prod = n.wrapping_mul(m);
                    let res = if is_sub {
                        a.wrapping_sub(prod)
                    } else {
                        a.wrapping_add(prod)
                    };
                    self.regs.write_w(rd, res);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::Divide {
                rd,
                rn,
                rm,
                is_signed,
                is_64,
            } => {
                if is_64 {
                    let n = self.regs.read_x(rn);
                    let m = self.regs.read_x(rm);
                    let res = if m == 0 {
                        0
                    } else if is_signed {
                        (n as i64).wrapping_div(m as i64) as u64
                    } else {
                        n / m
                    };
                    self.regs.write_x(rd, res);
                } else {
                    let n = self.regs.read_w(rn);
                    let m = self.regs.read_w(rm);
                    let res = if m == 0 {
                        0
                    } else if is_signed {
                        (n as i32).wrapping_div(m as i32) as u32
                    } else {
                        n / m
                    };
                    self.regs.write_w(rd, res);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::ShiftReg {
                rd,
                rn,
                rm,
                shift_type,
                is_64,
            } => {
                if is_64 {
                    let n = self.regs.read_x(rn);
                    let shift = (self.regs.read_x(rm) & 63) as u32;
                    let res = match shift_type {
                        ShiftType::Lsl => n.wrapping_shl(shift),
                        ShiftType::Lsr => n.wrapping_shr(shift),
                        ShiftType::Asr => ((n as i64).wrapping_shr(shift)) as u64,
                        ShiftType::Ror => n.rotate_right(shift),
                    };
                    self.regs.write_x(rd, res);
                } else {
                    let n = self.regs.read_w(rn);
                    let shift = (self.regs.read_w(rm) & 31) as u32;
                    let res = match shift_type {
                        ShiftType::Lsl => n.wrapping_shl(shift),
                        ShiftType::Lsr => n.wrapping_shr(shift),
                        ShiftType::Asr => ((n as i32).wrapping_shr(shift)) as u32,
                        ShiftType::Ror => n.rotate_right(shift),
                    };
                    self.regs.write_w(rd, res);
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::SimdLoadStoreQ {
                rt,
                rn,
                offset,
                is_load,
            } => {
                let base = self.regs.read_x_or_sp(rn);
                let addr = (base as i64).wrapping_add(offset) as u64;
                if is_load {
                    let lo = mmu.read_u64(addr)? as u128;
                    let hi = mmu.read_u64(addr + 8)? as u128;
                    self.regs.write_q(rt, lo | (hi << 64));
                } else {
                    let q = self.regs.read_q(rt);
                    mmu.write_u64(addr, (q & 0xFFFF_FFFF_FFFF_FFFF) as u64)?;
                    mmu.write_u64(addr + 8, (q >> 64) as u64)?;
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::SimdVec4S { rd, rn, rm, op } => {
                let a = self.regs.read_v4s(rn);
                let b = self.regs.read_v4s(rm);
                let mut out = [0.0f32; 4];
                for i in 0..4 {
                    out[i] = match op {
                        FpOp2::Fadd => a[i] + b[i],
                        FpOp2::Fsub => a[i] - b[i],
                        FpOp2::Fmul => a[i] * b[i],
                        FpOp2::Fdiv => a[i] / b[i],
                        FpOp2::Fmax => a[i].max(b[i]),
                        FpOp2::Fmin => a[i].min(b[i]),
                    };
                }
                self.regs.write_v4s(rd, out);
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::LoadStoreExclusive {
                rt,
                rn,
                rs,
                size_bytes,
                is_load,
                is_exclusive,
            } => {
                let addr = self.regs.read_x_or_sp(rn);
                if is_load {
                    let val = match size_bytes {
                        1 => mmu.read_u8(addr)? as u64,
                        2 => mmu.read_u16(addr)? as u64,
                        4 => mmu.read_u32(addr)? as u64,
                        8 => mmu.read_u64(addr)?,
                        _ => unreachable!(),
                    };
                    if size_bytes == 8 {
                        self.regs.write_x(rt, val);
                    } else {
                        self.regs.write_w(rt, val as u32);
                    }
                } else {
                    match size_bytes {
                        1 => mmu.write_u8(addr, self.regs.read_w(rt) as u8)?,
                        2 => mmu.write_u16(addr, self.regs.read_w(rt) as u16)?,
                        4 => mmu.write_u32(addr, self.regs.read_w(rt))?,
                        8 => mmu.write_u64(addr, self.regs.read_x(rt))?,
                        _ => unreachable!(),
                    }
                    if is_exclusive {
                        // Store-exclusive succeeded -> status register Ws = 0
                        self.regs.write_w(rs, 0);
                    }
                }
                self.regs.advance_pc();
                Ok(StepResult::Continue)
            }

            Instruction::Unimplemented { raw } => {
                Err(CpuError::UnimplementedInstruction { raw, pc })
            }
        }
    }

    /// Evaluates AArch64 condition codes against PSTATE.
    pub fn check_condition(&self, cond: Condition) -> bool {
        let pstate = &self.regs.pstate;
        match cond {
            Condition::Eq => pstate.z,
            Condition::Ne => !pstate.z,
            Condition::Cs => pstate.c,
            Condition::Cc => !pstate.c,
            Condition::Mi => pstate.n,
            Condition::Pl => !pstate.n,
            Condition::Vs => pstate.v,
            Condition::Vc => !pstate.v,
            Condition::Hi => pstate.c && !pstate.z,
            Condition::Ls => !pstate.c || pstate.z,
            Condition::Ge => pstate.n == pstate.v,
            Condition::Lt => pstate.n != pstate.v,
            Condition::Gt => !pstate.z && (pstate.n == pstate.v),
            Condition::Le => pstate.z || (pstate.n != pstate.v),
            Condition::Al | Condition::Nv => true,
        }
    }
}
