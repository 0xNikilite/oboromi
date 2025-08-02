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
        let pages = (size + 4095) / 4096;
        mmu.set_identity_mapping(0, pages);

        Memory {
            ram: vec![0; size],
            mmu,
        }
    }

    /// Map a memory range with permissions
    pub fn map_range(&mut self, vaddr: usize, size: usize, readable: bool, writable: bool) {
        let start_page = vaddr / 4096;
        let end_page = (vaddr + size + 4095) / 4096;
        
        for page in start_page..end_page {
            let vaddr = page * 4096;
            self.mmu.map_page(
                vaddr as u64, 
                vaddr as u64, 
                readable, 
                writable
            );
        }
    }

    /// Read one byte through MMU
    pub fn read_byte(&mut self, vaddr: usize) -> u8 {
        match self.mmu.translate(vaddr as u64) {
            Some((paddr, _, _)) => {
                let paddr = paddr as usize;
                if paddr < self.ram.len() {
                    self.ram[paddr]
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
            Some((paddr, _, _)) => {
                let paddr = paddr as usize;
                if paddr < self.ram.len() {
                    self.ram[paddr] = val;
                }
            }
            None => println!("⚠️ Page fault writing at {:#x}", vaddr),
        }
    }

    /// Read 32-bit little-endian word through MMU (atomic if cross-page)
    pub fn read_u32(&mut self, addr: usize) -> u32 {
        let start_page = addr / 4096;
        let end_page = (addr + 3) / 4096;
        
        // Atomic access if cross-page
        if start_page != end_page {
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[i] = self.read_byte(addr + i);
            }
            return u32::from_le_bytes(bytes);
        }
        
        // Direct access for single page
        if let Some((paddr, _, _)) = self.mmu.translate(addr as u64) {
            let paddr = paddr as usize;
            if paddr + 3 < self.ram.len() {
                return u32::from_le_bytes([
                    self.ram[paddr],
                    self.ram[paddr + 1],
                    self.ram[paddr + 2],
                    self.ram[paddr + 3]
                ]);
            }
        }
        0
    }

    /// Write 32-bit little-endian word through MMU (atomic if cross-page)
    pub fn write_u32(&mut self, addr: usize, value: u32) {
        let bytes = value.to_le_bytes();
        let start_page = addr / 4096;
        let end_page = (addr + 3) / 4096;
        
        // Atomic write if cross-page
        if start_page != end_page {
            for (i, &b) in bytes.iter().enumerate() {
                self.write_byte(addr + i, b);
            }
            return;
        }
        
        // Direct write for single page
        if let Some((paddr, _, _)) = self.mmu.translate(addr as u64) {
            let paddr = paddr as usize;
            if paddr + 3 < self.ram.len() {
                self.ram[paddr..paddr + 4].copy_from_slice(&bytes);
            }
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