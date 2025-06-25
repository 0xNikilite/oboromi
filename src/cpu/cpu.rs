// src/cpu/cpu.rs

use crate::memory::Memory;
use bitflags::bitflags;

bitflags! {
    /// Processor State Flags: Negative, Zero, Carry, Overflow
    pub struct Flags: u32 {
        const NEGATIVE = 1 << 31; // N flag: result is negative
        const ZERO     = 1 << 30; // Z flag: result is zero
        const CARRY    = 1 << 29; // C flag: carry/borrow
        const OVERFLOW = 1 << 28; // V flag: signed overflow
    }
}

/// ARM64 registers X0–X30, SP, PC and processor flags.
pub struct Registers {
    pub x: [u64; 31], // General-purpose registers
    pub sp: u64,      // Stack pointer
    pub pc: u64,      // Program counter
    pub flags: Flags, // NZCV flags
}

/// The CPU core, with registers and a flat memory interface.
pub struct CPU {
    pub regs: Registers,
    pub memory: Memory,
}

impl CPU {
    /// Create a new CPU with the given RAM size (bytes).
    pub fn new(mem_size: usize) -> Self {
        CPU {
            regs: Registers { x: [0; 31], sp: 0, pc: 0, flags: Flags::empty() },
            memory: Memory::new(mem_size),
        }
    }

    /// Reset registers, PC and flags; memory remains intact.
    pub fn reset(&mut self) {
        self.regs = Registers { x: [0; 31], sp: 0, pc: 0, flags: Flags::empty() };
    }

    /// Fetch a 32‑bit instruction from memory at PC.
    pub fn fetch(&self) -> u32 {
        self.memory.read_u32(self.regs.pc as usize)
    }

    /// Decode and execute one instruction.
    pub fn decode_and_execute(&mut self, opcode: u32) {
        // 1) NOP: encoding 0xD503201F
        if opcode == 0xD503201F {
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 2) ADDI / SUBI (immediate arithmetic)
        const IMM_MASK: u32 = 0xFF000000;
        const ADDI: u32     = 0x91000000;
        const SUBI: u32     = 0xD1000000;
        if (opcode & IMM_MASK) == ADDI || (opcode & IMM_MASK) == SUBI {
            let rd  = (opcode & 0x1F) as usize;
            let rn  = ((opcode >> 5) & 0x1F) as usize;
            let imm = ((opcode >> 10) & 0xFFF) as u64;
            let a   = self.regs.x.get(rn).copied().unwrap_or(0);
            let (res, carry, overflow) = if (opcode & IMM_MASK) == ADDI {
                let (r, c) = a.overflowing_add(imm);
                let v = ((a ^ r) & (imm ^ r)) >> 63 != 0;
                (r, c, v)
            } else {
                let (r, bt) = a.overflowing_sub(imm);
                let v = ((a ^ imm) & (a ^ r)) >> 63 != 0;
                (r, !bt, v)
            };
            if rd < 31 { self.regs.x[rd] = res; }
            self.regs.flags.set(Flags::CARRY, carry);
            self.regs.flags.set(Flags::OVERFLOW, overflow);
            self.regs.set_nz(res);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 3) Register‑form arithmetic/logical: ADD/SUB/AND/ORR/EOR
        const REG_MASK: u32  = 0xFFE00000;
        const ADDR: u32      = 0x8B000000;
        const SUBR: u32      = 0xCB000000;
        const ANDR: u32      = 0x8A000000;
        const ORR: u32       = 0xAA000000;
        const EOR: u32       = 0xCA000000;
        if matches!(opcode & REG_MASK, ADDR|SUBR|ANDR|ORR|EOR) {
            let rd = (opcode & 0x1F) as usize;
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;
            let a  = self.regs.x.get(rn).copied().unwrap_or(0);
            let b  = self.regs.x.get(rm).copied().unwrap_or(0);
            let (res, carry, overflow) = match opcode & REG_MASK {
                ADDR => {
                    let (r,c) = a.overflowing_add(b);
                    let v = ((a ^ r) & (b ^ r)) >> 63 != 0;
                    (r, c, v)
                }
                SUBR => {
                    let (r,bt) = a.overflowing_sub(b);
                    let v = ((a ^ b) & (a ^ r)) >> 63 != 0;
                    (r, !bt, v)
                }
                ANDR => (a & b, false, false),
                ORR  => (a | b, false, false),
                EOR  => (a ^ b, false, false),
                _    => unreachable!(),
            };
            if rd < 31 { self.regs.x[rd] = res; }
            self.regs.flags.set(Flags::CARRY, carry);
            self.regs.flags.set(Flags::OVERFLOW, overflow);
            self.regs.set_nz(res);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 4) CMP = SUBS XZR, Xn, Xm  (sets flags only)
        if (opcode & 0xFF000000) == 0xEB000000 && (opcode & 0x1F) == 31 {
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;
            let a  = self.regs.x.get(rn).copied().unwrap_or(0);
            let b  = self.regs.x.get(rm).copied().unwrap_or(0);
            let (res, bt) = a.overflowing_sub(b);
            let ov = ((a ^ b) & (a ^ res)) >> 63 != 0;
            self.regs.flags.set(Flags::CARRY, !bt);
            self.regs.flags.set(Flags::OVERFLOW, ov);
            self.regs.set_nz(res);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 5) TST = ANDS XZR, Xn, Xm  (sets NZ flags)
        if (opcode & 0xFF000000) == 0xEA000000 && (opcode & 0x1F) == 31 {
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;
            let res= self.regs.x.get(rn).copied().unwrap_or(0)
                     & self.regs.x.get(rm).copied().unwrap_or(0);
            self.regs.set_nz(res);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 6) LDR/STR immediate (64‑bit) load/store
        const LS_MASK: u32 = 0xFFC00000; // Changed to cover full opcode pattern
        const LDR: u32     = 0xF9400000;
        const STR: u32     = 0xF9000000;
        if (opcode & LS_MASK) == LDR || (opcode & LS_MASK) == STR {
            let rt   = (opcode & 0x1F) as usize;
            let rn   = ((opcode >> 5) & 0x1F) as usize;
            // Corrected immediate extraction - use mask 0x3FFC00
            let off  = ((opcode & 0x3FFC00) >> 10) as usize;
            let addr = (self.regs.x.get(rn).copied().unwrap_or(0) as usize)
                       .wrapping_add(off * 8);

            if (opcode & LS_MASK) == LDR {
                let v = self.memory.read_u64(addr);
                if rt < 31 { self.regs.x[rt] = v; }
            } else {
                if rt < 31 {
                    self.memory.write_u64(addr, self.regs.x[rt]);
                }
            }
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }

        // 7) Branch (B) and Return (RET)
        const B: u32      = 0x14000000;
        const B_MASK: u32 = 0x7C000000;
        if (opcode & B_MASK) == B {
            let imm = (opcode & 0x03FFFFFF) as i32;
            let off = ((imm << 6) >> 4) as u64;
            self.regs.pc = self.regs.pc.wrapping_add(off);
            return;
        }
        if opcode == 0xD65F03C0 {
            self.regs.pc = self.regs.x[30];
            return;
        }

        // Fallback for unimplemented instructions
        println!("⚠️ Unimplemented opcode: {:08X}", opcode);
        self.regs.pc = self.regs.pc.wrapping_add(4);
    }

    /// Perform one fetch‑decode‑execute cycle.
    pub fn step(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
    }
}

impl Registers {
    /// Set Negative and Zero flags based on the result.
    pub fn set_nz(&mut self, res: u64) {
        self.flags.set(Flags::NEGATIVE, (res >> 63) != 0);
        self.flags.set(Flags::ZERO,     res == 0);
    }
}