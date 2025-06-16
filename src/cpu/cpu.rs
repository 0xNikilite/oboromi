// src/cpu/cpu.rs

use crate::memory::Memory;

/// CPU registers for ARM64: X0–X30 general purpose, SP (stack pointer), PC (program counter),
/// and PSTATE (processor state flags: Negative, Zero, Carry, Overflow).
pub struct Registers {
    /// General-purpose registers X0–X30
    pub x: [u64; 31],
    /// Stack Pointer
    pub sp: u64,
    /// Program Counter
    pub pc: u64,
    /// Processor state flags (N, Z, C, V, etc.)
    pub pstate: u32,
}

/// The core CPU struct, holding registers and a flat memory buffer.
/// In a full emulator, `memory` would be replaced by a Memory trait object.
pub struct CPU {
    /// CPU registers
    pub regs: Registers,
    /// Linear memory buffer (e.g. DRAM). Size must match actual RAM (12 GiB),
    /// but i can start smaller for tests.
    pub memory: Memory,
}

impl CPU {
    /// Create a new CPU with given memory size in bytes.
    pub fn new(mem_size: usize) -> Self {
        CPU {
            regs: Registers {
                x: [0; 31],
                sp: 0,
                pc: 0,
                pstate: 0,
            },
            memory: Memory::new(mem_size)
        }
    }

    /// Reset CPU state to initial values.
    pub fn reset(&mut self) {
        self.regs.x = [0; 31];
        self.regs.sp = 0;
        self.regs.pc = 0;
        self.regs.pstate = 0;
        // Optionally clear memory or preserve it:
        // self.memory.fill(0);
    }

    /// Fetch a 32-bit instruction (opcode) from memory at the current PC.
    /// ARM64 instructions are 32 bits wide.
    pub fn fetch(&self) -> u32 {
        self.memory.read_u32(self.regs.pc as usize)
    }

    /// Decode and execute a single ARM64 opcode.
    /// For now this just handles a NOP (encoding 0xD503201F) as example.
    pub fn decode_and_execute(&mut self, opcode: u32) {
        match opcode {
            0xD5_03_20_1F => {
                // NOP: no operation, just advance PC.
            }
            _ => {
                // Unimplemented instruction
                // e.g.: println!("Unimplemented opcode: {:08x}", opcode);
            }
        }
        // Advance PC by 4 bytes to next instruction
        self.regs.pc = self.regs.pc.wrapping_add(4);
    }

    /// Perform one full CPU cycle: fetch, decode, execute.
    pub fn step(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
    }
}
