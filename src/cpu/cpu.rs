use crate::memory::Memory;
use bitflags::bitflags;

bitflags! {
    /// Processor State Flags: Negative, Zero, Carry, Overflow
    /// (bitflags! already impls Debug/Clone/Copy)
    pub struct Flags: u32 {
        const NEGATIVE = 1 << 31; // N flag: result is negative
        const ZERO     = 1 << 30; // Z flag: result is zero
        const CARRY    = 1 << 29; // C flag: carry/borrow
        const OVERFLOW = 1 << 28; // V flag: signed overflow
    }
}

/// ARM64 registers X0–X30, SP, PC and processor flags.
pub struct Registers {
    pub x: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub flags: Flags,
}

/// The CPU core, with registers and a flat memory interface.
// Remove any `#[derive(Debug)]` here if you had it.
pub struct CPU {
    pub regs: Registers,
    pub memory: Memory,
}

macro_rules! set_flags {
    ($cpu:expr, $res:expr, $carry:expr, $overflow:expr) => {{
        $cpu.regs.flags.set(Flags::CARRY, $carry);
        $cpu.regs.flags.set(Flags::OVERFLOW, $overflow);
        $cpu.regs.set_nz($res);
    }};
}

impl CPU {
    /// Create a new CPU with `mem_size` bytes of RAM.
    pub fn new(mem_size: usize) -> Self {
        CPU {
            regs: Registers { x: [0; 31], sp: 0, pc: 0, flags: Flags::empty() },
            memory: Memory::new(mem_size),
        }
    }

    /// Reset registers, PC and flags (memory untouched).
    pub fn reset(&mut self) {
        self.regs = Registers { x: [0; 31], sp: 0, pc: 0, flags: Flags::empty() };
    }

    /// Fetch the 32-bit opcode at current PC.
    pub fn fetch(&self) -> u32 {
        self.memory.read_u32(self.regs.pc as usize)
    }

    /// Decode and execute one instruction.
    pub fn decode_and_execute(&mut self, opcode: u32) {
        if opcode == 0xD503201F {
            self.regs.pc = self.regs.pc.wrapping_add(4);
            return;
        }
        if self.exec_addi_subi(opcode) { return; }
        if self.exec_reg_ops(opcode)  { return; }
        if self.exec_cmp_tst(opcode)  { return; }
        if self.exec_ldr_str(opcode)  { return; }
        if self.exec_branch_ret(opcode){ return; }

        // Fallback
        println!("⚠️ Unimplemented opcode: {:08X}", opcode);
        self.regs.pc = self.regs.pc.wrapping_add(4);
    }

    /// One fetch–decode–execute cycle.
    pub fn step(&mut self) {
        let op = self.fetch();
        self.decode_and_execute(op);
    }

    fn exec_addi_subi(&mut self, opcode: u32) -> bool {
        const IMM_MASK: u32 = 0xFF000000;
        const ADDI: u32     = 0x91000000;
        const SUBI: u32     = 0xD1000000;
        if (opcode & IMM_MASK) == ADDI || (opcode & IMM_MASK) == SUBI {
            let rd  = (opcode & 0x1F) as usize;
            let rn  = ((opcode >> 5) & 0x1F) as usize;
            let imm = ((opcode >> 10) & 0xFFF) as u64;
            let a   = self.regs.x.get(rn).copied().unwrap_or(0);
            let (res, c, v) = if (opcode & IMM_MASK) == ADDI {
                let (r, c) = a.overflowing_add(imm);
                let v = ((a ^ r) & (imm ^ r)) >> 63 != 0;
                (r, c, v)
            } else {
                let (r, bt) = a.overflowing_sub(imm);
                let v = ((a ^ imm) & (a ^ r)) >> 63 != 0;
                (r, !bt, v)
            };
            if rd < 31 { self.regs.x[rd] = res; }
            set_flags!(self, res, c, v);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            true
        } else {
            false
        }
    }

    fn exec_reg_ops(&mut self, opcode: u32) -> bool {
        const REG_MASK: u32 = 0xFFE00000;
        const ADD: u32      = 0x8B000000;
        const SUB: u32      = 0xCB000000;
        const AND: u32      = 0x8A000000;
        const ORR: u32      = 0xAA000000;
        const EOR: u32      = 0xCA000000;
        if matches!(opcode & REG_MASK, ADD|SUB|AND|ORR|EOR) {
            let rd = (opcode & 0x1F) as usize;
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;
            let a  = self.regs.x.get(rn).copied().unwrap_or(0);
            let b  = self.regs.x.get(rm).copied().unwrap_or(0);
            let (res, c, v) = match opcode & REG_MASK {
                ADD => { let (r,c)=a.overflowing_add(b); let v=((a^r)&(b^r))>>63!=0; (r,c,v) }
                SUB => { let (r,bt)=a.overflowing_sub(b); let v=((a^b)&(a^r))>>63!=0; (r,!bt,v) }
                AND => (a & b, false, false),
                ORR => (a | b, false, false),
                EOR => (a ^ b, false, false),
                _   => unreachable!(),
            };
            if rd < 31 { self.regs.x[rd] = res; }
            set_flags!(self, res, c, v);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            true
        } else {
            false
        }
    }

    fn exec_cmp_tst(&mut self, opcode: u32) -> bool {
        // CMP: 0xEB000000.. & rd==31, TST: 0xEA000000.. & rd==31
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
            true
        } else if (opcode & 0xFF000000) == 0xEA000000 && (opcode & 0x1F) == 31 {
            let rn = ((opcode >> 5) & 0x1F) as usize;
            let rm = ((opcode >> 16) & 0x1F) as usize;
            let res = self.regs.x.get(rn).copied().unwrap_or(0)
                    & self.regs.x.get(rm).copied().unwrap_or(0);
            self.regs.set_nz(res);
            self.regs.pc = self.regs.pc.wrapping_add(4);
            true
        } else {
            false
        }
    }

    fn exec_ldr_str(&mut self, opcode: u32) -> bool {
        const LS_MASK: u32 = 0xFFC00000;
        const LDR: u32     = 0xF9400000;
        const STR: u32     = 0xF9000000;
        if (opcode & LS_MASK) == LDR || (opcode & LS_MASK) == STR {
            let rt  = (opcode & 0x1F) as usize;
            let rn  = ((opcode >> 5) & 0x1F) as usize;
            let imm = ((opcode & 0x3FFC00) >> 10) as usize * 8;
            let base= self.regs.x.get(rn).copied().unwrap_or(0) as usize;
            let addr= base.wrapping_add(imm);
            if (opcode & LS_MASK) == LDR {
                let v = self.memory.read_u64(addr);
                if rt < 31 { 
                    self.regs.x[rt] = v; 
                }
            } else if rt < 31 {
                self.memory.write_u64(addr, self.regs.x[rt]);
            }
            self.regs.pc = self.regs.pc.wrapping_add(4);
            true
        } else {
            false
        }
    }

    fn exec_branch_ret(&mut self, opcode: u32) -> bool {
        const B_MASK: u32 = 0x7C000000;
        const B: u32      = 0x14000000;
        if (opcode & B_MASK) == B {
            let imm = (opcode & 0x03FFFFFF) as i32;
            let off = (imm as i64 * 4) as u64;
            self.regs.pc = self.regs.pc.wrapping_add(off);
            true
        } 
        else if opcode == 0xD65F03C0 {
            self.regs.pc = self.regs.x[30];
            true
        } else {
            false
        }
    }
}

impl Registers {
    /// Update Negative and Zero flags.
    pub fn set_nz(&mut self, res: u64) {
        self.flags.set(Flags::NEGATIVE, (res >> 63) != 0);
        self.flags.set(Flags::ZERO,     res == 0);
    }
}