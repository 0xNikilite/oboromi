// src/memory/memory.rs

#![allow(clippy::module_inception)]

#[derive(Debug)]
/// System RAM abstraction with bounds-checked reads/writes.
pub struct Memory {
    ram: Vec<u8>,
}

impl Memory {
    /// Create RAM of `size` bytes, zero-initialized.
    pub fn new(size: usize) -> Self {
        Memory { ram: vec![0; size] }
    }

    /// Read one byte; out-of-bounds → 0.
    pub fn read_byte(&self, addr: usize) -> u8 {
        self.ram.get(addr).copied().unwrap_or(0)
    }

    /// Write one byte; out-of-bounds → ignored.
    pub fn write_byte(&mut self, addr: usize, val: u8) {
        if addr < self.ram.len() {
            self.ram[addr] = val;
        }
    }

    /// Read 32-bit little-endian word; any OOB → 0.
    pub fn read_u32(&self, addr: usize) -> u32 {
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            bytes[i] = self.read_byte(addr + i);
        }
        let mut bytes = [0u8; 4];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = self.read_byte(addr + i);
        }
        u32::from_le_bytes(bytes)
    }

    /// Write 32-bit little-endian word; OOB bytes ignored.
    pub fn write_u32(&mut self, addr: usize, value: u32) {
        let bytes = value.to_le_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            self.write_byte(addr + i, b);
        }
    }

    /// Read 64-bit little-endian word; any OOB → 0.
    pub fn read_u64(&self, addr: usize) -> u64 {
        let low = self.read_u32(addr) as u64;
        let high = self.read_u32(addr + 4) as u64;
        low | (high << 32)
    }

    /// Write 64-bit little-endian word; OOB bytes ignored.
    pub fn write_u64(&mut self, addr: usize, value: u64) {
        self.write_u32(addr, value as u32);
        self.write_u32(addr + 4, (value >> 32) as u32);
    }
}