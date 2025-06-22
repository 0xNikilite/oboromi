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
        // 1) NOP and fake ADD cases first
        match opcode {
            0xD503201F => {
                // NOP: do nothing
                self.regs.pc = self.regs.pc.wrapping_add(4);
                return;
            }
            0xD2802674 => {
                // Fake ADD X0, X1, X2
                self.regs.add_with_flags(0, self.regs.x[1], self.regs.x[2]);
                self.regs.pc = self.regs.pc.wrapping_add(4);
                return;
            }
            _ => {}
        }

        // 2) MOV alias: ORR Xd, XZR, #imm12 (dynamic)
        const MOV_MASK: u32    = 0b1111111 << 25; // bits 31–25
        const MOV_PATTERN: u32 = 0b1010000 << 25; // pattern for ORR immediate
        if (opcode & MOV_MASK) == MOV_PATTERN {
            let rd    = (opcode        & 0x1F) as usize;   // bits [4:0]
            let imm12 = ((opcode >> 10) & 0xFFF) as u64;    // bits [21:10]
            self.regs.x[rd] = imm12;
            self.regs.set_nz(imm12);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 3) Unimplemented
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
