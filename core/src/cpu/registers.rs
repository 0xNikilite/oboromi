#![allow(dead_code)]

/// ARMv8.2-A AArch64 Processor Registers.
///
/// Models the 31 general-purpose registers (X0..X30), the Stack Pointer (SP),
/// the Program Counter (PC), the PSTATE flags (NZCV), and the 32 SIMD/FP registers.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pstate {
    /// Negative condition flag (bit 31)
    pub n: bool,
    /// Zero condition flag (bit 30)
    pub z: bool,
    /// Carry condition flag (bit 29)
    pub c: bool,
    /// Overflow condition flag (bit 28)
    pub v: bool,
}

impl Pstate {
    pub const fn new() -> Self {
        Self {
            n: false,
            z: false,
            c: false,
            v: false,
        }
    }

    /// Read raw NZCV bit representation
    pub fn to_bits(&self) -> u32 {
        ((self.n as u32) << 31)
            | ((self.z as u32) << 30)
            | ((self.c as u32) << 29)
            | ((self.v as u32) << 28)
    }

    /// Set from raw NZCV bit representation
    pub fn from_bits(&mut self, val: u32) {
        self.n = (val & (1 << 31)) != 0;
        self.z = (val & (1 << 30)) != 0;
        self.c = (val & (1 << 29)) != 0;
        self.v = (val & (1 << 28)) != 0;
    }
}

#[derive(Debug, Clone)]
pub struct CpuRegisters {
    /// 31 General Purpose 64-bit registers (X0 to X30).
    /// Note: Register 31 represents XZR (Zero Register) in most contexts,
    /// or SP (Stack Pointer) in certain instructions.
    pub x: [u64; 31],
    /// Dedicated Stack Pointer (SP)
    pub sp: u64,
    /// Program Counter (PC)
    pub pc: u64,
    /// Processor state flags
    pub pstate: Pstate,
    /// Thread Pointer / ID (EL0) - for Thread-Local Storage (TLS)
    pub tpidr_el0: u64,
    /// Read-Only Thread Pointer (EL0) - Horizon OS TLS / IPC command buffer pointer
    pub tpidrro_el0: u64,
    /// 32 128-bit SIMD / Floating-Point registers (V0 to V31)
    pub v: [u128; 32],
}

impl Default for CpuRegisters {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuRegisters {
    pub fn new() -> Self {
        Self {
            x: [0; 31],
            sp: 0,
            pc: 0,
            pstate: Pstate::new(),
            tpidr_el0: 0,
            tpidrro_el0: 0,
            v: [0; 32],
        }
    }

    /// Read a 64-bit general-purpose register.
    /// Index 31 is the zero register (XZR), which returns 0.
    #[inline(always)]
    pub fn read_x(&self, reg: u8) -> u64 {
        if reg < 31 {
            self.x[reg as usize]
        } else {
            0
        }
    }

    /// Write a 64-bit general-purpose register.
    /// Writes to register 31 (XZR) are discarded.
    #[inline(always)]
    pub fn write_x(&mut self, reg: u8, val: u64) {
        if reg < 31 {
            self.x[reg as usize] = val;
        }
    }

    /// Read a 32-bit register (W0..W30, or WZR).
    #[inline(always)]
    pub fn read_w(&self, reg: u8) -> u32 {
        self.read_x(reg) as u32
    }

    /// Write a 32-bit register (zero-extends the upper 32 bits into the 64-bit register).
    #[inline(always)]
    pub fn write_w(&mut self, reg: u8, val: u32) {
        self.write_x(reg, val as u64);
    }

    /// Read register with SP support (if reg == 31, returns SP; otherwise returns X[reg]).
    #[inline(always)]
    pub fn read_x_or_sp(&self, reg: u8) -> u64 {
        if reg == 31 {
            self.sp
        } else {
            self.x[reg as usize]
        }
    }

    /// Write register with SP support (if reg == 31, updates SP; otherwise updates X[reg]).
    #[inline(always)]
    pub fn write_x_or_sp(&mut self, reg: u8, val: u64) {
        if reg == 31 {
            self.sp = val;
        } else {
            self.x[reg as usize] = val;
        }
    }

    /// Read 64-bit double precision float from V register (D0..D31).
    #[inline(always)]
    pub fn read_d(&self, reg: u8) -> f64 {
        let raw = (self.v[(reg & 0x1F) as usize] & 0xFFFFFFFFFFFFFFFF) as u64;
        f64::from_bits(raw)
    }

    /// Write 64-bit double precision float into V register (clears upper 64 bits).
    #[inline(always)]
    pub fn write_d(&mut self, reg: u8, val: f64) {
        self.v[(reg & 0x1F) as usize] = val.to_bits() as u128;
    }

    /// Read 32-bit single precision float from V register (S0..S31).
    #[inline(always)]
    pub fn read_s(&self, reg: u8) -> f32 {
        let raw = (self.v[(reg & 0x1F) as usize] & 0xFFFFFFFF) as u32;
        f32::from_bits(raw)
    }

    /// Write 32-bit single precision float into V register (clears upper 96 bits).
    #[inline(always)]
    pub fn write_s(&mut self, reg: u8, val: f32) {
        self.v[(reg & 0x1F) as usize] = val.to_bits() as u128;
    }

    /// Read full 128-bit NEON Q register (`Q0..Q31`).
    #[inline(always)]
    pub fn read_q(&self, reg: u8) -> u128 {
        self.v[(reg & 0x1F) as usize]
    }

    /// Write full 128-bit NEON Q register (`Q0..Q31`).
    #[inline(always)]
    pub fn write_q(&mut self, reg: u8, val: u128) {
        self.v[(reg & 0x1F) as usize] = val;
    }

    /// Read 128-bit NEON register as 4 lanes of 32-bit floats (`Vd.4S`).
    #[inline(always)]
    pub fn read_v4s(&self, reg: u8) -> [f32; 4] {
        let raw = self.v[(reg & 0x1F) as usize];
        [
            f32::from_bits((raw & 0xFFFF_FFFF) as u32),
            f32::from_bits(((raw >> 32) & 0xFFFF_FFFF) as u32),
            f32::from_bits(((raw >> 64) & 0xFFFF_FFFF) as u32),
            f32::from_bits(((raw >> 96) & 0xFFFF_FFFF) as u32),
        ]
    }

    /// Write 4 lanes of 32-bit floats into a 128-bit NEON register (`Vd.4S`).
    #[inline(always)]
    pub fn write_v4s(&mut self, reg: u8, lanes: [f32; 4]) {
        let l0 = lanes[0].to_bits() as u128;
        let l1 = (lanes[1].to_bits() as u128) << 32;
        let l2 = (lanes[2].to_bits() as u128) << 64;
        let l3 = (lanes[3].to_bits() as u128) << 96;
        self.v[(reg & 0x1F) as usize] = l0 | l1 | l2 | l3;
    }

    /// Advance program counter by 4 bytes (standard 32-bit ARM instruction size).
    #[inline(always)]
    pub fn advance_pc(&mut self) {
        self.pc = self.pc.wrapping_add(4);
    }
}
