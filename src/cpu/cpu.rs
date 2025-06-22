// src/cpu/cpu.rs

use crate::memory::Memory;
use bitflags::bitflags;

bitflags! {
    /// Processor State Flags: Negative, Zero, Carry, Overflow
    pub struct Flags: u32 {
        const NEGATIVE = 1 << 31; // N
        const ZERO     = 1 << 30; // Z
        const CARRY    = 1 << 29; // C
        const OVERFLOW = 1 << 28; // V
    }
}

/// CPU registers for ARM64: X0–X30, SP, PC, and Flags.
pub struct Registers {
    pub x: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub flags: Flags,
}

pub struct CPU {
    pub regs: Registers,
    pub memory: Memory,
}

impl CPU {
    /// Create a new CPU with given memory size.
    pub fn new(mem_size: usize) -> Self {
        CPU {
            regs: Registers {
                x: [0; 31],
                sp: 0,
                pc: 0,
                flags: Flags::empty(),
            },
            memory: Memory::new(mem_size),
        }
    }

    /// Reset state (registers, PC and flags), preserve memory.
    pub fn reset(&mut self) {
        self.regs.x = [0; 31];
        self.regs.sp = 0;
        self.regs.pc = 0;
        self.regs.flags = Flags::empty();
    }

    /// Fetch a 32-bit opcode from memory.
    pub fn fetch(&self) -> u32 {
        self.memory.read_u32(self.regs.pc as usize)
    }

    /// Decode and execute a single ARM64 opcode.
    pub fn decode_and_execute(&mut self, opcode: u32) {
        // 1) NOP
        if opcode == 0xD503201F {
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 2) MOV (ORR Xd, XZR, #imm12)
        const MOV_MASK: u32    = 0b1111111 << 25;
        const MOV_PATTERN: u32 = 0b1010000 << 25;
        if (opcode & MOV_MASK) == MOV_PATTERN {
            let rd = (opcode & 0x1F) as usize;
            let imm12 = ((opcode >> 10) & 0xFFF) as u64;
            self.regs.x[rd] = imm12;
            self.regs.set_nz(imm12);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 3) ADD/SUB (register-register)
        // Pattern bits [31:21] = 10001011000 (ADD), 11001011000 (SUB)
        const ARITH_MASK: u32  = 0xFFE00000; // mask bits 31..21
        const ADD_OPCODE: u32  = 0x8B000000; // ADD Xd, Xn, Xm
        const SUB_OPCODE: u32  = 0xCB000000; // SUB Xd, Xn, Xm
        if (opcode & ARITH_MASK) == ADD_OPCODE || (opcode & ARITH_MASK) == SUB_OPCODE {
            let rd = (opcode & 0x1F) as usize;
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;

            let src1 = self.regs.x[rn];
            let src2 = self.regs.x[rm];

            let result = if (opcode & ARITH_MASK) == ADD_OPCODE {
                self.regs.set_cv_add(src1, src2);
                src1.wrapping_add(src2)
            } else {
                self.regs.set_cv_add(src1, !src2 + 1);
                src1.wrapping_sub(src2)
            };

            self.regs.x[rd] = result;
            self.regs.set_nz(result);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 4) Fallback
        println!("⚠️ Unimplemented opcode: {:08X}", opcode);
        self.regs.pc = self.regs.pc.wrapping_add(4);
    }

    /// Perform one CPU cycle.
    pub fn step(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
    }
}

impl Registers {
    pub fn set_nz(&mut self, result: u64) {
        self.flags.set(Flags::NEGATIVE, (result as i64) < 0);
        self.flags.set(Flags::ZERO, result == 0);
    }

    pub fn set_cv_add(&mut self, a: u64, b: u64) {
        let (res, carry) = a.overflowing_add(b);
        let overflow = ((a ^ !b) & (a ^ res)) >> 63 != 0;
        self.flags.set(Flags::CARRY, carry);
        self.flags.set(Flags::OVERFLOW, overflow);
    }

    pub fn add_with_flags(&mut self, dst: usize, src1: u64, src2: u64) {
        self.set_cv_add(src1, src2);
        let result = src1.wrapping_add(src2);
        self.set_nz(result);
        self.x[dst] = result;
    }
}
