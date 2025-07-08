#![allow(clippy::module_inception)]

use crate::mmu::MMU;

#[derive(Debug)]
/// System RAM abstraction with MMU support
pub struct Memory {
    pub ram: Vec<u8>,
    pub mmu: MMU,
}

impl Memory {
    /// Create RAM of `size` bytes with MMU and identity mapping
    pub fn new(size: usize) -> Self {
        let mut mmu = MMU::new();
        
        // Setup identity mapping for the entire RAM
        let pages = (size + 4095) / 4096; // Arrotonda per eccesso
        mmu.set_identity_mapping(0, pages);

        Memory {
            ram: vec![0; size],
            mmu,
        }
    }

    /// Read one byte through MMU
    pub fn read_byte(&mut self, vaddr: usize) -> u8 {
        match self.mmu.translate(vaddr as u64) {
            Some(paddr) => {
                let paddr_usize = paddr as usize;
                if paddr_usize < self.ram.len() {
                    self.ram[paddr_usize]
                } else {
                    0
                }
            }
            None => {
                println!("⚠️ Page fault reading at {:#x}", vaddr);
                0
            }
        }
    }

    /// Write one byte through MMU
    pub fn write_byte(&mut self, vaddr: usize, val: u8) {
        match self.mmu.translate(vaddr as u64) {
            Some(paddr) => {
                let paddr_usize = paddr as usize;
                if paddr_usize < self.ram.len() {
                    self.ram[paddr_usize] = val;
                }
            }
            None => println!("⚠️ Page fault writing at {:#x}", vaddr),
        }
    }

    /// Read 32-bit little-endian word through MMU
    pub fn read_u32(&mut self, addr: usize) -> u32 {
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            bytes[i] = self.read_byte(addr + i);
        }
        u32::from_le_bytes(bytes)
    }

    /// Write 32-bit little-endian word through MMU
    pub fn write_u32(&mut self, addr: usize, value: u32) {
        let bytes = value.to_le_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            self.write_byte(addr + i, b);
        }
    }

    /// Read 64-bit little-endian word through MMU
    pub fn read_u64(&mut self, addr: usize) -> u64 {
        let low = self.read_u32(addr) as u64;
        let high = self.read_u32(addr + 4) as u64;
        low | (high << 32)
    }

    /// Write 64-bit little-endian word through MMU
    pub fn write_u64(&mut self, addr: usize, value: u64) {
        self.write_u32(addr, value as u32);
        self.write_u32(addr + 4, (value >> 32) as u32);
    }
}