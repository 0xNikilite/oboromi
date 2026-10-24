#![allow(dead_code)]

use super::instructions::{
    BranchRegOp, Condition, FpOp2, Instruction, LogicalOp, MoveWideOp, ShiftType,
};

/// Decodes a 32-bit little-endian ARMv8.2-A AArch64 opcode.
pub fn decode(raw: u32) -> Instruction {
    // 1. NOP / HINT / Barrier / Cache Maintenance check:
    //    0xD503201F (NOP), 0xD503245F (BTI c), 0xD503237F (PACIBSP), DSB/DMB/ISB (0xD5033xxx), DC/IC (0xD5087xxx / 0xD50B7xxx)
    if (raw & 0xFFFF_F01F) == 0xD503_201F
        || (raw & 0xFFFF_F000) == 0xD503_3000
        || (raw & 0xFFF0_0000) == 0xD500_0000
    {
        return Instruction::Nop;
    }

    // 1b. Pointer-Authenticated Return: RETAA (0xD65F0BFF) / RETAB (0xD65F0FFF)
    if raw == 0xD65F_0BFF || raw == 0xD65F_0FFF {
        return Instruction::BranchReg {
            rn: 30,
            op: BranchRegOp::Ret,
        };
    }

    // 2. Supervisor Call: SVC #imm16
    // Pattern: 1101 0100 000 imm16 00001
    if (raw & 0xFFE0001F) == 0xD4000001 {
        let imm = ((raw >> 5) & 0xFFFF) as u16;
        return Instruction::Svc { imm };
    }

    // 3. PC-Relative: ADR and ADRP
    // Pattern: [31] 00 10000 [23:5] [4:0]
    if (raw & 0x1F000000) == 0x10000000 {
        let is_page = (raw & (1 << 31)) != 0;
        let immlo = ((raw >> 29) & 0x3) as u64;
        let immhi = ((raw >> 5) & 0x7FFFF) as u64;
        let raw_imm = (immhi << 2) | immlo;
        let signed_imm = sign_extend(raw_imm, 21);
        let rd = (raw & 0x1F) as u8;

        let imm = if is_page {
            signed_imm << 12
        } else {
            signed_imm
        };

        return Instruction::Adr { rd, imm, is_page };
    }

    // 4. Move Wide (Immediate): MOVZ, MOVN, MOVK
    // Pattern: sf 10 100101 hw imm16 Rd
    if (raw & 0x1F800000) == 0x12800000 || (raw & 0x1F800000) == 0x52800000 || (raw & 0x1F800000) == 0x72800000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let opc = (raw >> 29) & 0x3;
        let hw = ((raw >> 21) & 0x3) as u8;
        let imm16 = ((raw >> 5) & 0xFFFF) as u16;
        let rd = (raw & 0x1F) as u8;
        let shift = hw * 16;

        let op = match opc {
            0b00 => MoveWideOp::Movn,
            0b10 => MoveWideOp::Movz,
            0b11 => MoveWideOp::Movk,
            _ => return Instruction::Unimplemented { raw },
        };

        return Instruction::MoveWide {
            rd,
            imm: imm16,
            shift,
            op,
            is_64,
        };
    }

    // 5. Add / Subtract (Immediate): ADD, ADDS, SUB, SUBS
    // Pattern: sf op S 100010 shift imm12 Rn Rd
    if (raw & 0x1F000000) == 0x11000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let is_sub = (raw & (1 << 30)) != 0;
        let set_flags = (raw & (1 << 29)) != 0;
        let shift = (raw >> 22) & 0x3;
        let imm12 = ((raw >> 10) & 0xFFF) as u64;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        let imm = match shift {
            0 => imm12,
            1 => imm12 << 12,
            _ => return Instruction::Unimplemented { raw },
        };

        return Instruction::AddSubImm {
            rd,
            rn,
            imm,
            is_sub,
            set_flags,
            is_64,
        };
    }

    // 6. Add / Subtract (Shifted or Extended Register)
    // Pattern: sf op S 01011 shift/opt is_ext Rm imm6/opt+imm3 Rn Rd
    if (raw & 0x1F000000) == 0x0B000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let is_sub = (raw & (1 << 30)) != 0;
        let set_flags = (raw & (1 << 29)) != 0;
        let is_extended = (raw & (1 << 21)) != 0;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        let (shift_type, shift_amount, extend_option) = if is_extended {
            let opt = ((raw >> 13) & 0x7) as u8;
            let imm3 = ((raw >> 10) & 0x7) as u8;
            (ShiftType::Lsl, imm3, opt)
        } else {
            let sh_type = match (raw >> 22) & 0x3 {
                0 => ShiftType::Lsl,
                1 => ShiftType::Lsr,
                2 => ShiftType::Asr,
                _ => ShiftType::Ror,
            };
            let imm6 = ((raw >> 10) & 0x3F) as u8;
            (sh_type, imm6, 0)
        };

        return Instruction::AddSubReg {
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
        };
    }

    // 7. Logical (Shifted Register): AND, BIC, ORR, ORN, EOR, EON, ANDS, BICS
    // Pattern: sf opc 01010 shift N Rm imm6 Rn Rd
    if (raw & 0x1F000000) == 0x0A000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let opc = (raw >> 29) & 0x3;
        let shift_bits = (raw >> 22) & 0x3;
        let invert_rm = (raw & (1 << 21)) != 0;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let shift_amount = ((raw >> 10) & 0x3F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        let shift_type = match shift_bits {
            0b00 => ShiftType::Lsl,
            0b01 => ShiftType::Lsr,
            0b10 => ShiftType::Asr,
            _ => ShiftType::Ror,
        };

        let (op, set_flags) = match opc {
            0b00 => (LogicalOp::And, false),
            0b01 => (LogicalOp::Orr, false),
            0b10 => (LogicalOp::Eor, false),
            0b11 => (LogicalOp::Ands, true),
            _ => unreachable!(),
        };

        return Instruction::LogicalReg {
            rd,
            rn,
            rm,
            shift_type,
            shift_amount,
            invert_rm,
            op,
            set_flags,
            is_64,
        };
    }

    // 7b. Logical (Immediate): AND, ORR, EOR, ANDS Xd, Xn, #bitmask_imm
    // Pattern: sf opc 100100 N immr imms Rn Rd
    if (raw & 0x1F800000) == 0x12000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let opc = (raw >> 29) & 0x3;
        let n = ((raw >> 22) & 1) as u8;
        let immr = ((raw >> 16) & 0x3F) as u8;
        let imms = ((raw >> 10) & 0x3F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        if let Some(imm) = decode_bit_mask(is_64, n, imms, immr) {
            let (op, set_flags) = match opc {
                0b00 => (LogicalOp::And, false),
                0b01 => (LogicalOp::Orr, false),
                0b10 => (LogicalOp::Eor, false),
                0b11 => (LogicalOp::Ands, true),
                _ => unreachable!(),
            };
            return Instruction::LogicalImm {
                rd,
                rn,
                imm,
                op,
                set_flags,
                is_64,
            };
        }
    }

    // 7c. Bitfield: SBFM (00), BFM (01), UBFM (10)
    // Pattern: sf opc 100110 N immr imms Rn Rd
    if (raw & 0x1F800000) == 0x13000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let opc = ((raw >> 29) & 0x3) as u8;
        let immr = ((raw >> 16) & 0x3F) as u8;
        let imms = ((raw >> 10) & 0x3F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        if opc <= 2 {
            return Instruction::Bitfield {
                rd,
                rn,
                immr,
                imms,
                opc,
                is_64,
            };
        }
    }

    // 7d. Extract: EXTR Xd, Xn, Xm, #lsb
    // Pattern: sf 00 100111 N 0 Rm imms Rn Rd
    if (raw & 0x1FA00000) == 0x13800000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let lsb = ((raw >> 10) & 0x3F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return Instruction::Extract {
            rd,
            rn,
            rm,
            lsb,
            is_64,
        };
    }

    // 7e. Conditional Select: CSEL, CSINC, CSINV, CSNEG
    // Pattern: sf op 0 11010100 Rm cond 0 o2 Rn Rd
    if (raw & 0x1FE00800) == 0x1A800000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let op_hi = ((raw >> 30) & 1) as u8;
        let op_lo = ((raw >> 10) & 1) as u8;
        let op = (op_hi << 1) | op_lo;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let cond = Condition::from_u8(((raw >> 12) & 0xF) as u8);
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return Instruction::CondSelect {
            rd,
            rn,
            rm,
            cond,
            op,
            is_64,
        };
    }

    // 7f. Conditional Compare: CCMP / CCMN (Register or Immediate)
    // Pattern: sf op 1 11010010 Rm_or_imm5 cond is_imm 0 Rn 0 nzcv
    if (raw & 0x1FE00410) == 0x1A400000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let is_sub = (raw & (1 << 30)) != 0;
        let rm_or_imm = ((raw >> 16) & 0x1F) as u8;
        let cond = Condition::from_u8(((raw >> 12) & 0xF) as u8);
        let is_imm = (raw & (1 << 11)) != 0;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let nzcv = (raw & 0xF) as u8;
        return Instruction::CondCompare {
            rn,
            rm_or_imm,
            nzcv,
            cond,
            is_imm,
            is_sub,
            is_64,
        };
    }

    // 10b. Test and Branch (TBZ / TBNZ)
    // Pattern: b5 011011 op b40 imm14 Rt
    if (raw & 0x7E000000) == 0x36000000 {
        let b5 = ((raw >> 31) & 1) as u8;
        let b40 = ((raw >> 19) & 0x1F) as u8;
        let bit = (b5 << 5) | b40;
        let is_tbnz = (raw & (1 << 24)) != 0;
        let imm14 = ((raw >> 5) & 0x3FFF) as u64;
        let offset = sign_extend(imm14, 14) * 4;
        let rt = (raw & 0x1F) as u8;
        return Instruction::TestBranch {
            rt,
            bit,
            offset,
            is_tbnz,
        };
    }

    // 8. Unconditional Branch (Immediate): B / BL
    // Pattern: op 00101 imm26
    if (raw & 0x7C000000) == 0x14000000 {
        let link = (raw & (1 << 31)) != 0;
        let imm26 = (raw & 0x03FFFFFF) as u64;
        let signed_offset = sign_extend(imm26, 26) * 4;
        return Instruction::BranchImm {
            offset: signed_offset,
            link,
        };
    }

    // 9. Conditional Branch (B.cond)
    // Pattern: 0101010 0 imm19 0 cond
    if (raw & 0xFF000010) == 0x54000000 {
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let signed_offset = sign_extend(imm19, 19) * 4;
        let cond_val = (raw & 0xF) as u8;
        return Instruction::BranchCond {
            offset: signed_offset,
            cond: Condition::from_u8(cond_val),
        };
    }

    // 10. Compare and Branch (CBZ / CBNZ)
    // Pattern: sf 011010 op imm19 Rt
    if (raw & 0x7E000000) == 0x34000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let is_cbnz = (raw & (1 << 24)) != 0;
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) * 4;
        let rt = (raw & 0x1F) as u8;
        return Instruction::CompareBranch {
            rt,
            offset,
            is_cbnz,
            is_64,
        };
    }

    // 11. Branch to Register: BR, BLR, RET
    // Pattern: 1101011 0000 11111 000000 Rn 00000
    if (raw & 0xFE1FFC1F) == 0xD61F0000 {
        let opc = (raw >> 21) & 0x3;
        let rn = ((raw >> 5) & 0x1F) as u8;

        let op = match opc {
            0 => BranchRegOp::Br,
            1 => BranchRegOp::Blr,
            2 => BranchRegOp::Ret,
            _ => return Instruction::Unimplemented { raw },
        };

        return Instruction::BranchReg { rn, op };
    }

    // 12. Load / Store Pair (LDP / STP)
    // Pattern: opc 101 V 0 mode L imm7 Rt2 Rn Rt1
    if (raw & 0x3A000000) == 0x28000000 {
        let opc = (raw >> 30) & 0x3;
        let is_load = (raw & (1 << 22)) != 0;
        let is_64 = opc == 2;
        let scale = if is_64 { 8 } else { 4 };
        let imm7 = ((raw >> 15) & 0x7F) as u64;
        let offset = sign_extend(imm7, 7) * scale;
        let rt2 = ((raw >> 10) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rt1 = (raw & 0x1F) as u8;

        let mode = (raw >> 23) & 0x3;
        let (writeback, post_index) = match mode {
            0b01 => (true, true),  // Post-index
            0b10 => (false, false), // Signed offset
            0b11 => (true, false), // Pre-index
            _ => return Instruction::Unimplemented { raw },
        };

        return Instruction::LoadStorePair {
            rt1,
            rt2,
            rn,
            offset,
            is_load,
            is_64,
            writeback,
            post_index,
        };
    }

    // 13. Load / Store Single Register (Immediate unsigned offset, V == 0)
    if (raw & 0x3F000000) == 0x39000000 {
        let size = ((raw >> 30) & 0x3) as u8;
        let opc = ((raw >> 22) & 0x3) as u8;
        let is_load = opc != 0;
        let is_signed = opc == 2 || opc == 3;
        let imm12 = ((raw >> 10) & 0xFFF) as u64;
        let scale = 1 << size;
        let offset = (imm12 * scale) as i64;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rt = (raw & 0x1F) as u8;

        return Instruction::LoadStoreImm {
            rt,
            rn,
            offset,
            size_bytes: scale as u8,
            is_load,
            is_signed,
            writeback: false,
            post_index: false,
        };
    }

    // 13b. Load / Store Single Register (Unscaled Immediate LDUR/STUR, Post-Index, Pre-Index, & Register Offset, V == 0)
    if (raw & 0x3F000000) == 0x38000000 {
        let size = ((raw >> 30) & 0x3) as u8;
        let opc = ((raw >> 22) & 0x3) as u8;
        let is_load = opc != 0;
        let is_signed = opc == 2 || opc == 3;
        let scale = 1u8 << size;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rt = (raw & 0x1F) as u8;

        if (raw & (1 << 21)) != 0 && ((raw >> 10) & 0x3) == 0b10 {
            // Register offset: LDR/STR Rt, [Xn, Xm, option #shift]
            let rm = ((raw >> 16) & 0x1F) as u8;
            let option = ((raw >> 13) & 0x7) as u8;
            let s = ((raw >> 12) & 1) as u8;
            let shift = if s != 0 { size } else { 0 };
            let is_64_target = size == 3 || opc == 2;
            return Instruction::LoadStoreReg {
                rt,
                rn,
                rm,
                option,
                shift,
                size_bytes: scale,
                is_load,
                is_signed,
                is_64_target,
            };
        } else {
            let mode = (raw >> 10) & 0x3;
            let imm9 = ((raw >> 12) & 0x1FF) as u64;
            let offset = sign_extend(imm9, 9);
            let (writeback, post_index) = match mode {
                0b00 => (false, false), // Unscaled LDUR / STUR
                0b01 => (true, true),   // Post-index [Xn], #imm9
                0b11 => (true, false),  // Pre-index [Xn, #imm9]!
                _ => return Instruction::Unimplemented { raw },
            };
            return Instruction::LoadStoreImm {
                rt,
                rn,
                offset,
                size_bytes: scale,
                is_load,
                is_signed,
                writeback,
                post_index,
            };
        }
    }

    // 13c. Load/Store Exclusive & Load-Acquire/Store-Release (LDXR, LDAXR, LDAR, STXR, STLXR, STLR)
    // Pattern: size 001000 o2 L o1 Rs o0 Rt2 Rn Rt
    if (raw & 0x3F000000) == 0x08000000 {
        let size = ((raw >> 30) & 0x3) as u8;
        let is_exclusive = (raw & (1 << 23)) == 0;
        let is_load = (raw & (1 << 22)) != 0;
        let rs = ((raw >> 16) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rt = (raw & 0x1F) as u8;
        return Instruction::LoadStoreExclusive {
            rt,
            rn,
            rs,
            size_bytes: 1u8 << size,
            is_load,
            is_exclusive,
        };
    }

    // 14. Floating-Point 2-Source Arithmetic: FADD, FSUB, FMUL, FDIV, FMAX, FMIN
    // Pattern: 0 0 0 11110 ftype 1 Rm opcode 00 Rn Rd
    if (raw & 0x5F200C00) == 0x1E200800 {
        let ftype = (raw >> 22) & 0x3;
        if ftype == 0 || ftype == 1 {
            let is_double = ftype == 1;
            let rm = ((raw >> 16) & 0x1F) as u8;
            let opcode = (raw >> 12) & 0xF;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let rd = (raw & 0x1F) as u8;

            let op = match opcode {
                0b0000 => FpOp2::Fmul,
                0b0001 => FpOp2::Fdiv,
                0b0010 => FpOp2::Fadd,
                0b0011 => FpOp2::Fsub,
                0b0100 => FpOp2::Fmax,
                0b0101 => FpOp2::Fmin,
                _ => return Instruction::Unimplemented { raw },
            };

            return Instruction::FpData2 {
                rd,
                rn,
                rm,
                op,
                is_double,
            };
        }
    }

    // 15. Floating-Point Compare: FCMP
    if (raw & 0x5F203C1F) == 0x1E202000 {
        let ftype = (raw >> 22) & 0x3;
        let is_double = ftype == 1;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        return Instruction::FpCompare {
            rn,
            rm,
            is_zero: false,
            is_double,
        };
    }

    // 16. System Register Read/Write: MRS Xt, <sysreg> / MSR <sysreg>, Xt
    // MRS: 1101 0101 0011 op1 CRn CRm op2 Rt (0xD5300000)
    // MSR: 1101 0101 0001 op1 CRn CRm op2 Rt (0xD5100000)
    if (raw & 0xFFF00000) == 0xD5300000 || (raw & 0xFFF00000) == 0xD5100000 {
        let is_read = (raw & (1 << 21)) != 0;
        let sysreg = ((raw >> 5) & 0x7FFF) as u16;
        let rt = (raw & 0x1F) as u8;
        return Instruction::SystemReg {
            rt,
            sysreg,
            is_read,
        };
    }

    // 17. Integer Multiply-Add / Multiply-Subtract: MADD, MSUB, MUL
    // Pattern: sf 00 11011 000 Rm o0 Ra Rn Rd
    if (raw & 0x7FE00000) == 0x1B000000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let is_sub = (raw & (1 << 15)) != 0;
        let ra = ((raw >> 10) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return Instruction::MultiplyAddSub {
            rd,
            rn,
            rm,
            ra,
            is_sub,
            is_64,
        };
    }

    // 18. Data-Processing (2-Source): UDIV, SDIV, LSLV, LSRV, ASRV, RORV
    // Pattern: sf 00 11010110 Rm opcode Rn Rd
    if (raw & 0x7FE00000) == 0x1AC00000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let opcode = (raw >> 10) & 0x3F;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        match opcode {
            0b000010 | 0b000011 => {
                return Instruction::Divide {
                    rd,
                    rn,
                    rm,
                    is_signed: opcode == 0b000011,
                    is_64,
                };
            }
            0b001000..=0b001011 => {
                let shift_type = match opcode & 0x3 {
                    0 => ShiftType::Lsl,
                    1 => ShiftType::Lsr,
                    2 => ShiftType::Asr,
                    _ => ShiftType::Ror,
                };
                return Instruction::ShiftReg {
                    rd,
                    rn,
                    rm,
                    shift_type,
                    is_64,
                };
            }
            _ => {}
        }
    }

    // 19. Floating-Point / General Register Transfer: FMOV Dd, Xn / FMOV Xd, Dn / FMOV Sd, Wn / FMOV Wd, Sn
    if (raw & 0x7FBEFC00) == 0x1E260000 {
        let is_64 = (raw & (1 << 31)) != 0;
        let to_fp = ((raw >> 16) & 1) != 0;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return Instruction::FpMovGen {
            rd,
            rn,
            to_fp,
            is_64,
        };
    }

    // 20. Floating-Point Scalar & 128-bit NEON Q Load / Store (Immediate unsigned offset):
    //     LDR/STR St/Dt/Qt, [Xn, #offset]
    if (raw & 0x3F000000) == 0x3D000000 {
        let size = ((raw >> 30) & 0x3) as u8;
        let opc = ((raw >> 22) & 0x3) as u8;
        let imm12 = ((raw >> 10) & 0xFFF) as u64;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rt = (raw & 0x1F) as u8;

        // 128-bit Q register load/store: size == 0b00 && (opc == 0b10 [STR Q] || opc == 0b11 [LDR Q])
        if size == 0 && (opc == 0b10 || opc == 0b11) {
            let is_load = opc == 0b11;
            let offset = (imm12 * 16) as i64;
            return Instruction::SimdLoadStoreQ {
                rt,
                rn,
                offset,
                is_load,
            };
        }

        if (size == 2 || size == 3) && (opc == 0b00 || opc == 0b01) {
            let is_load = opc == 0b01;
            let is_double = size == 3;
            let scale = 1u64 << size;
            let offset = (imm12 * scale) as i64;
            return Instruction::FpLoadStoreImm {
                rt,
                rn,
                offset,
                is_load,
                is_double,
            };
        }
    }

    // 21. 128-bit NEON Vector 4xFloat32 SIMD:
    //     FADD Vd.4S, Vn.4S, Vm.4S (0x4E20D400)
    //     FSUB Vd.4S, Vn.4S, Vm.4S (0x4EA0D400)
    //     FMUL Vd.4S, Vn.4S, Vm.4S (0x6E20DC00)
    if (raw & 0xFF20FC00) == 0x4E20D400 || (raw & 0xFFE0FC00) == 0x6E20DC00 {
        let rm = ((raw >> 16) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        let op = if (raw & 0xFFE0FC00) == 0x6E20DC00 {
            FpOp2::Fmul
        } else if (raw & (1 << 23)) != 0 {
            FpOp2::Fsub
        } else {
            FpOp2::Fadd
        };
        return Instruction::SimdVec4S { rd, rn, rm, op };
    }

    Instruction::Unimplemented { raw }
}

/// Sign-extends an `n`-bit unsigned integer to a 64-bit signed integer.
#[inline(always)]
fn sign_extend(val: u64, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((val << shift) as i64) >> shift
}

/// Decodes an ARMv8 bitmask immediate (`N:imms:immr`) for `AND`, `ORR`, `EOR`, and `ANDS`.
fn decode_bit_mask(is_64: bool, n: u8, imms: u8, immr: u8) -> Option<u64> {
    if !is_64 && n != 0 {
        return None;
    }
    let combined = ((n as u32) << 6) | ((!imms as u32) & 0x3F);
    if combined == 0 {
        return None;
    }
    let len = 31 - combined.leading_zeros();
    if len < 1 {
        return None;
    }
    let esize = 1u32 << len;
    let levels = esize - 1;
    let s = (imms as u32) & levels;
    let r = (immr as u32) & levels;
    if s == levels {
        return None;
    }
    let welem: u64 = if s + 1 == 64 {
        u64::MAX
    } else {
        (1u64 << (s + 1)) - 1
    };
    let rotated = if r == 0 {
        welem
    } else if esize == 64 {
        welem.rotate_right(r)
    } else {
        let mask = (1u64 << esize) - 1;
        let elem = welem & mask;
        ((elem >> r) | (elem << (esize - r))) & mask
    };
    let mut result = 0u64;
    let mut shift = 0u32;
    while shift < 64 {
        result |= rotated << shift;
        shift += esize;
    }
    if !is_64 {
        result &= 0xFFFF_FFFF;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nop() {
        assert_eq!(decode(0xD503201F), Instruction::Nop);
    }

    #[test]
    fn test_decode_svc() {
        assert_eq!(decode(0xD40004E1), Instruction::Svc { imm: 0x27 });
    }

    #[test]
    fn test_decode_ret() {
        assert_eq!(
            decode(0xD65F03C0),
            Instruction::BranchReg {
                rn: 30,
                op: BranchRegOp::Ret
            }
        );
    }

    #[test]
    fn test_decode_movz() {
        match decode(0xD2800540) {
            Instruction::MoveWide { rd, imm, shift, op, is_64 } => {
                assert_eq!(rd, 0);
                assert_eq!(imm, 42);
                assert_eq!(shift, 0);
                assert_eq!(op, MoveWideOp::Movz);
                assert!(is_64);
            }
            other => panic!("Unexpected decode: {:?}", other),
        }
    }

    #[test]
    fn test_decode_fadd() {
        assert_eq!(
            decode(0x1E622820),
            Instruction::FpData2 {
                rd: 0,
                rn: 1,
                rm: 2,
                op: FpOp2::Fadd,
                is_double: true,
            }
        );
    }
}
