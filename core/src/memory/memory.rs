use crate::mmu::MMU;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

#[derive(Debug)]
pub struct Memory {
    pub ram: Vec<u8>,
    pub mmu: MMU,
    exclusive_monitor: Arc<ExclusiveMonitor>,
}

#[derive(Debug)]
pub struct ExclusiveMonitor {
    monitored_address: AtomicU64,
    monitored_size: AtomicU64,
    owner_thread_id: AtomicU64,
}

impl ExclusiveMonitor {
    pub fn new() -> Self {
        Self {
            monitored_address: AtomicU64::new(u64::MAX),
            monitored_size: AtomicU64::new(0),
            owner_thread_id: AtomicU64::new(0),
        }
    }

    pub fn mark_exclusive(&self, addr: u64, size: u64) -> bool {
        let current_addr = self.monitored_address.load(Ordering::Acquire);
        if current_addr == u64::MAX {
            self.monitored_address.store(addr, Ordering::Release);
            self.monitored_size.store(size, Ordering::Release);
            self.owner_thread_id
                .store(Self::get_thread_id(), Ordering::Release);
            true
        } else {
            false
        }
    }

    pub fn clear_exclusive(&self) {
        self.monitored_address.store(u64::MAX, Ordering::Release);
        self.monitored_size.store(0, Ordering::Release);
        self.owner_thread_id.store(0, Ordering::Release);
    }

    pub fn check_exclusive(&self, addr: u64, size: u64) -> bool {
        let monitored_addr = self.monitored_address.load(Ordering::Acquire);
        let monitored_size = self.monitored_size.load(Ordering::Acquire);
        let owner_thread_id = self.owner_thread_id.load(Ordering::Acquire);

        monitored_addr == addr && monitored_size == size && owner_thread_id == Self::get_thread_id()
    }

    fn get_thread_id() -> u64 {
        let thread_id = thread::current().id();
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        thread_id.hash(&mut hasher);
        hasher.finish()
    }
}

impl Memory {
    pub fn new(size: usize) -> Self {
        let mut mmu = MMU::new();
        let pages = (size + 4095) / 4096;
        mmu.set_identity_mapping(0, pages);

        Memory {
            ram: vec![0; size],
            mmu,
            exclusive_monitor: Arc::new(ExclusiveMonitor::new()),
        }
    }

    pub fn map_range(&mut self, vaddr: usize, size: usize, readable: bool, writable: bool) {
        let start_page = vaddr / 4096;
        let end_page = (vaddr + size + 4095) / 4096;

        for page in start_page..end_page {
            let vaddr = page * 4096;
            self.mmu
                .map_page(vaddr as u64, vaddr as u64, readable, writable);
        }
    }

    pub fn read_byte(&mut self, vaddr: usize) -> u8 {
        match self.mmu.translate(vaddr as u64) {
            Some((paddr, _, _)) => {
                let paddr = paddr as usize;
                if paddr < self.ram.len() {
                    self.ram[paddr]
                } else {
                    println!("⚠️ Page fault reading at {:#x}", vaddr);
                    0
                }
            }
            None => {
                println!("⚠️ Page fault reading at {:#x}", vaddr);
                0
            }
        }
    }

    pub fn write_byte(&mut self, vaddr: usize, val: u8) {
        match self.mmu.translate(vaddr as u64) {
            Some((paddr, _, _)) => {
                let paddr = paddr as usize;
                if paddr < self.ram.len() {
                    self.ram[paddr] = val;
                } else {
                    println!("⚠️ Page fault writing at {:#x}", vaddr);
                }
            }
            None => {
                println!("⚠️ Page fault writing at {:#x}", vaddr);
            }
        }
    }

    pub fn read_u32(&mut self, addr: usize) -> u32 {
        let start_page = addr / 4096;
        let end_page = (addr + 3) / 4096;

        if start_page != end_page {
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[i] = self.read_byte(addr + i);
            }
            return u32::from_le_bytes(bytes);
        }

        if let Some((paddr, _, _)) = self.mmu.translate(addr as u64) {
            let paddr = paddr as usize;
            if paddr + 3 < self.ram.len() {
                u32::from_le_bytes([
                    self.ram[paddr],
                    self.ram[paddr + 1],
                    self.ram[paddr + 2],
                    self.ram[paddr + 3],
                ])
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn write_u32(&mut self, addr: usize, value: u32) {
        let start_page = addr / 4096;
        let end_page = (addr + 3) / 4096;

        if start_page != end_page {
            let bytes = value.to_le_bytes();
            for i in 0..4 {
                self.write_byte(addr + i, bytes[i]);
            }
            return;
        }

        if let Some((paddr, _, _)) = self.mmu.translate(addr as u64) {
            let paddr = paddr as usize;
            if paddr + 3 < self.ram.len() {
                let bytes = value.to_le_bytes();
                self.ram[paddr] = bytes[0];
                self.ram[paddr + 1] = bytes[1];
                self.ram[paddr + 2] = bytes[2];
                self.ram[paddr + 3] = bytes[3];
            }
        }
    }

    pub fn read_u64(&mut self, addr: usize) -> u64 {
        let low = self.read_u32(addr) as u64;
        let high = self.read_u32(addr + 4) as u64;
        low | (high << 32)
    }

    pub fn write_u64(&mut self, addr: usize, value: u64) {
        self.write_u32(addr, value as u32);
        self.write_u32(addr + 4, (value >> 32) as u32);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_read_u8(mem_ptr: *mut Memory, addr: u64) -> u8 {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.read_byte(addr as usize)
            } else {
                0
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_write_u8(mem_ptr: *mut Memory, addr: u64, value: u8) {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.write_byte(addr as usize, value);
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_read_u16(mem_ptr: *mut Memory, addr: u64) -> u16 {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                let low = mem.read_byte(addr as usize) as u16;
                let high = mem.read_byte((addr + 1) as usize) as u16;
                low | (high << 8)
            } else {
                0
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_write_u16(mem_ptr: *mut Memory, addr: u64, value: u16) {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.write_byte(addr as usize, (value & 0xFF) as u8);
                mem.write_byte((addr + 1) as usize, ((value >> 8) & 0xFF) as u8);
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_read_u32(mem_ptr: *mut Memory, addr: u64) -> u32 {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.read_u32(addr as usize)
            } else {
                0
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_write_u32(mem_ptr: *mut Memory, addr: u64, value: u32) {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.write_u32(addr as usize, value);
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_read_u64(mem_ptr: *mut Memory, addr: u64) -> u64 {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.read_u64(addr as usize)
            } else {
                0
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_write_u64(mem_ptr: *mut Memory, addr: u64, value: u64) {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.write_u64(addr as usize, value);
            }
        }
    }

    pub fn atomic_compare_exchange_u8(&mut self, addr: usize, expected: u8, new: u8) -> u8 {
        let current = self.read_byte(addr);
        if current == expected {
            self.write_byte(addr, new);
        }
        current
    }

    pub fn atomic_compare_exchange_u32(&mut self, addr: usize, expected: u32, new: u32) -> u32 {
        let current = self.read_u32(addr);
        if current == expected {
            self.write_u32(addr, new);
        }
        current
    }

    pub fn atomic_add_u32(&mut self, addr: usize, value: u32) -> u32 {
        let current = self.read_u32(addr);
        self.write_u32(addr, current.wrapping_add(value));
        current
    }

    pub fn mark_exclusive(&self, addr: u64, size: u64) -> bool {
        self.exclusive_monitor.mark_exclusive(addr, size)
    }

    pub fn clear_exclusive(&self) {
        self.exclusive_monitor.clear_exclusive();
    }

    pub fn exclusive_write_u8(&mut self, addr: u64, value: u8) -> bool {
        if self.exclusive_monitor.check_exclusive(addr, 1) {
            self.write_byte(addr as usize, value);
            self.exclusive_monitor.clear_exclusive();
            true
        } else {
            false
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_mark_exclusive(
        mem_ptr: *mut Memory,
        addr: u64,
        size: u64,
    ) -> bool {
        unsafe {
            if let Some(mem) = mem_ptr.as_ref() {
                mem.mark_exclusive(addr, size)
            } else {
                false
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_clear_exclusive(mem_ptr: *mut Memory) {
        unsafe {
            if let Some(mem) = mem_ptr.as_ref() {
                mem.clear_exclusive();
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_read_exclusive_u8(
        mem_ptr: *mut Memory,
        addr: u64,
        value: *mut u8,
    ) -> bool {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                if mem.mark_exclusive(addr, 1) {
                    *value = mem.read_byte(addr as usize);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn oboromi_memory_write_exclusive_u8(
        mem_ptr: *mut Memory,
        addr: u64,
        value: u8,
    ) -> bool {
        unsafe {
            if let Some(mem) = mem_ptr.as_mut() {
                mem.exclusive_write_u8(addr, value)
            } else {
                false
            }
        }
    }
}
