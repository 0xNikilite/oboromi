// src/memory/memory.rs

//⚠️, this emoji will be added only when there is a "serious" problem

/// Represents the system's memory interface
pub struct Memory {
    ram: Vec<u8>, // RAM memory storage
}

impl Memory {
    /// Creates a new instance of the memory with given size (in bytes)
    pub fn new(size: usize) -> Self {
        Memory {
            ram: vec![0; size], // initialize RAM to requested size
        }
    }

    /// Reads a byte from memory at the given address
    /// Returns 0 if the address is out of bounds
    pub fn read_byte(&self, addr: usize) -> u8 {
        self.ram.get(addr).copied().unwrap_or(0)
    }

    /// Writes a byte to memory at the given address
    /// Silently ignores if the address is out of bounds
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if let Some(byte) = self.ram.get_mut(addr) {
            *byte = value;
        }
    }

    /// Reads a little-endian 32-bit word from memory at the given address
    /// Returns 0 if any byte in the range is out of bounds
    pub fn read_u32(&self, addr: usize) -> u32 {
        if addr + 4 > self.ram.len() {
            return 0;
        }
        let b0 = self.read_byte(addr);
        let b1 = self.read_byte(addr + 1);
        let b2 = self.read_byte(addr + 2);
        let b3 = self.read_byte(addr + 3);
        u32::from_le_bytes([b0, b1, b2, b3])
    }
}
