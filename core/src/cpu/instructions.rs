#![allow(dead_code)]

/// AArch64 Condition Codes for conditional branching (`B.cond`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Eq = 0x0, // Equal (Z == 1)
    Ne = 0x1, // Not Equal (Z == 0)
    Cs = 0x2, // Carry Set / Unsigned Higher or Same (C == 1)
    Cc = 0x3, // Carry Clear / Unsigned Lower (C == 0)
    Mi = 0x4, // Minus / Negative (N == 1)
    Pl = 0x5, // Plus / Positive or Zero (N == 0)
    Vs = 0x6, // Overflow (V == 1)
    Vc = 0x7, // No Overflow (V == 0)
    Hi = 0x8, // Unsigned Higher (C == 1 && Z == 0)
    Ls = 0x9, // Unsigned Lower or Same (C == 0 || Z == 1)
    Ge = 0xA, // Signed Greater Than or Equal (N == V)
    Lt = 0xB, // Signed Less Than (N != V)
    Gt = 0xC, // Signed Greater Than (Z == 0 && N == V)
    Le = 0xD, // Signed Less Than or Equal (Z == 1 || N != V)
    Al = 0xE, // Always
    Nv = 0xF, // Always (never tested in practice)
}

impl Condition {
    pub fn from_u8(val: u8) -> Self {
        match val & 0xF {
            0x0 => Condition::Eq,
            0x1 => Condition::Ne,
            0x2 => Condition::Cs,
            0x3 => Condition::Cc,
            0x4 => Condition::Mi,
            0x5 => Condition::Pl,
            0x6 => Condition::Vs,
            0x7 => Condition::Vc,
            0x8 => Condition::Hi,
            0x9 => Condition::Ls,
            0xA => Condition::Ge,
            0xB => Condition::Lt,
            0xC => Condition::Gt,
            0xD => Condition::Le,
            0xE => Condition::Al,
            _ => Condition::Nv,
        }
    }
}

/// Decoded AArch64 Instructions supported by the Phase 1 core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// PC-relative address calculation: `ADR Xd, imm` or `ADRP Xd, imm`
    Adr {
        rd: u8,
        imm: i64,
        is_page: bool, // true if ADRP
    },

    /// Add / Subtract (Immediate): `ADD/SUB{S} Xd|Wd, Xn|Wn|SP, #imm{, LSL #12}`
    AddSubImm {
        rd: u8,
        rn: u8,
        imm: u64,
        is_sub: bool,
        set_flags: bool,
        is_64: bool,
    },

    /// Add / Subtract (Shifted or Extended Register): `ADD/SUB{S} Xd, Xn, Xm{, shift/extend}`
    AddSubReg {
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: ShiftType,
        shift_amount: u8,
        is_extended: bool,
        extend_option: u8,
        is_sub: bool,
        set_flags: bool,
        is_64: bool,
    },

    /// Logical (Shifted Register): `AND/BIC/ORR/ORN/EOR/EON/ANDS/BICS Xd, Xn, Xm{, shift #imm}`
    LogicalReg {
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: ShiftType,
        shift_amount: u8,
        invert_rm: bool,
        op: LogicalOp,
        set_flags: bool,
        is_64: bool,
    },

    /// Logical (Immediate): `AND/ORR/EOR/ANDS Xd, Xn, #bitmask_imm`
    LogicalImm {
        rd: u8,
        rn: u8,
        imm: u64,
        op: LogicalOp,
        set_flags: bool,
        is_64: bool,
    },

    /// Bitfield Move: `SBFM/BFM/UBFM Xd, Xn, #immr, #imms` (includes `LSL/LSR/ASR/UXTB/UXTH/SXTW/UBFX/BFI`)
    Bitfield {
        rd: u8,
        rn: u8,
        immr: u8,
        imms: u8,
        opc: u8,
        is_64: bool,
    },

    /// Extract Register: `EXTR/ROR Xd, Xn, Xm, #lsb`
    Extract {
        rd: u8,
        rn: u8,
        rm: u8,
        lsb: u8,
        is_64: bool,
    },

    /// Conditional Select: `CSEL/CSINC/CSINV/CSNEG Xd, Xn, Xm, cond`
    CondSelect {
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
        op: u8, // 0=CSEL, 1=CSINC, 2=CSINV, 3=CSNEG
        is_64: bool,
    },

    /// Conditional Compare: `CCMP/CCMN Xn, Xm|#imm, #nzcv, cond`
    CondCompare {
        rn: u8,
        rm_or_imm: u8,
        nzcv: u8,
        cond: Condition,
        is_imm: bool,
        is_sub: bool,
        is_64: bool,
    },

    /// Test Bit and Branch: `TBZ/TBNZ Rt, #bit, label`
    TestBranch {
        rt: u8,
        bit: u8,
        offset: i64,
        is_tbnz: bool,
    },

    /// Single Load / Store (Register Offset): `LDR/STR Rt, [Xn, Xm, LSL/SXTW #shift]`
    LoadStoreReg {
        rt: u8,
        rn: u8,
        rm: u8,
        option: u8,
        shift: u8,
        size_bytes: u8,
        is_load: bool,
        is_signed: bool,
        is_64_target: bool,
    },

    /// Move Wide: `MOVZ/MOVN/MOVK Xd, #imm{, LSL #shift}`
    MoveWide {
        rd: u8,
        imm: u16,
        shift: u8,
        op: MoveWideOp,
        is_64: bool,
    },

    /// Unconditional Branch (Immediate): `B label` or `BL label`
    BranchImm {
        offset: i64,
        link: bool, // true if BL (saves return address in X30)
    },

    /// Conditional Branch: `B.cond label`
    BranchCond {
        offset: i64,
        cond: Condition,
    },

    /// Branch to Register: `BR/BLR/RET Xn`
    BranchReg {
        rn: u8,
        op: BranchRegOp,
    },

    /// Compare and Branch: `CBZ/CBNZ Xn|Wn, label`
    CompareBranch {
        rt: u8,
        offset: i64,
        is_cbnz: bool,
        is_64: bool,
    },

    /// Single Load / Store (Immediate / Unscaled / Pre/Post Index): `LDR/STR Rt, [Xn, #offset]`
    LoadStoreImm {
        rt: u8,
        rn: u8,
        offset: i64,
        size_bytes: u8, // 1 (byte), 2 (half), 4 (word), 8 (double)
        is_load: bool,
        is_signed: bool,
        writeback: bool,
        post_index: bool,
    },

    /// Load / Store Pair: `LDP/STP Rt1, Rt2, [Xn, #offset]`
    LoadStorePair {
        rt1: u8,
        rt2: u8,
        rn: u8,
        offset: i64,
        is_load: bool,
        is_64: bool,
        writeback: bool,
        post_index: bool,
    },

    /// System Call / Supervisor Call: `SVC #imm16`
    Svc {
        imm: u16,
    },

    /// System Register Read / Write: `MRS/MSR`
    SystemReg {
        rt: u8,
        sysreg: u16,
        is_read: bool,
    },

    /// No Operation / Hint
    Nop,

    /// Floating-Point 2-Source Arithmetic: `FADD/FSUB/FMUL/FDIV Dd/Sd, Dn/Sn, Dm/Sm`
    FpData2 {
        rd: u8,
        rn: u8,
        rm: u8,
        op: FpOp2,
        is_double: bool,
    },

    /// Floating-Point Compare: `FCMP Dn/Sn, Dm/Sm` or `FCMP Dn/Sn, #0.0`
    FpCompare {
        rn: u8,
        rm: u8,
        is_zero: bool,
        is_double: bool,
    },

    /// Floating-Point / GP Register Transfer: `FMOV Dd, Xn`, `FMOV Xd, Dn`, etc.
    FpMovGen {
        rd: u8,
        rn: u8,
        to_fp: bool,
        is_64: bool,
    },

    /// Floating-Point Single Load / Store (Immediate unsigned offset): `LDR/STR St/Dt, [Xn, #offset]`
    FpLoadStoreImm {
        rt: u8,
        rn: u8,
        offset: i64,
        is_load: bool,
        is_double: bool,
    },

    /// Integer Multiply-Add / Multiply-Subtract (`MADD`, `MSUB`, `MUL`): `Rd = Ra +/- Rn * Rm`
    MultiplyAddSub {
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
        is_sub: bool,
        is_64: bool,
    },

    /// Integer Divide (`UDIV` / `SDIV`)
    Divide {
        rd: u8,
        rn: u8,
        rm: u8,
        is_signed: bool,
        is_64: bool,
    },

    /// Variable Shift (`LSLV`, `LSRV`, `ASRV`, `RORV`)
    ShiftReg {
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: ShiftType,
        is_64: bool,
    },

    /// 128-bit NEON Vector Load / Store (`LDR Qt, [Xn, #offset]` / `STR Qt, [Xn, #offset]`)
    SimdLoadStoreQ {
        rt: u8,
        rn: u8,
        offset: i64,
        is_load: bool,
    },

    /// 128-bit NEON Vector 4xFloat32 SIMD (`FADD/FSUB/FMUL Vd.4S, Vn.4S, Vm.4S`)
    SimdVec4S {
        rd: u8,
        rn: u8,
        rm: u8,
        op: FpOp2,
    },

    /// Exclusive & Acquire/Release Load / Store (`LDXR`, `LDAXR`, `LDAR`, `STXR`, `STLXR`, `STLR`)
    LoadStoreExclusive {
        rt: u8,
        rn: u8,
        rs: u8,
        size_bytes: u8,
        is_load: bool,
        is_exclusive: bool,
    },

    /// Unimplemented or Unknown instruction (preserves raw 32-bit opcode for debugging)
    Unimplemented {
        raw: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftType {
    Lsl = 0,
    Lsr = 1,
    Asr = 2,
    Ror = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Orr,
    Eor,
    Ands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveWideOp {
    Movn = 0b00,
    Movz = 0b10,
    Movk = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchRegOp {
    Br,
    Blr,
    Ret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpOp2 {
    Fmul,
    Fdiv,
    Fadd,
    Fsub,
    Fmax,
    Fmin,
}
