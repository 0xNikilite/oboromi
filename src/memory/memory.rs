// src/memory/memory.rs

//⚠️, this emoji will be added only when there is a "serious" problem

/// Represents the system's memory interface
pub struct Memory {
    ram: Vec<u8>, // ram memory storage
}

impl Memory {
    /// Creates a new instance of the memory
    pub fn new() -> Self {
        Memory {
            ram: vec![0; 64 * 1024 * 1024] // 64 MiB of ram
        }
    }
    
    /// Reads a byte from the memory at the given address
    pub fn read_byte(&self, addr: usize) -> u8 {
        self.ram[addr] // ⚠️ no bounds check [will panic if addr is out of range]
    }

    /// Writes a byte to the memory at the given address
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.ram[addr] = value; // ⚠️ no bounds check [will panic if addr is out of range]
    }
}