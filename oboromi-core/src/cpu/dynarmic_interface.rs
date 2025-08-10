use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use crate::dynarmic_sys::*;
use crate::memory::Memory;

/// Safe wrapper for Dynarmic CPU
pub struct DynarmicCPU {
    handle: Arc<Mutex<*mut DynarmicCPUIface>>,
    _memory: Arc<Box<Memory>>, // Keep Memory alive as long as CPU uses it
}

impl DynarmicCPU {
    /// Create a new Dynarmic instance linked to your Rust Memory
    pub fn new() -> Option<Self> {
        // Allocate memory and put it in a Box for a stable address
        let memory = Box::new(Memory::new(8 * 1024 * 1024)); // 8MB di memoria
        
        // Generic C pointer to Memory
        let mem_ptr: *mut c_void = &*memory as *const _ as *mut c_void;
        
        // Create CPU instance passing the memory backend
        let cpu_ptr = unsafe { dynarmic_create_instance(mem_ptr) };
        if cpu_ptr.is_null() {
            return None;
        }
        
        Some(Self {
            handle: Arc::new(Mutex::new(cpu_ptr)),
            _memory: Arc::new(memory),
        })
    }

    /// Run the core until halt or end of code
    pub fn run(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_run(*handle) }
    }

    /// Execute a single step
    pub fn step(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_step(*handle) }
    }

    /// Halt execution
    pub fn halt(&self) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_halt(*handle) }
    }

    /// Read register Xn
    pub fn get_x(&self, reg_index: u32) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_x(*handle, reg_index) }
    }

    /// Write register Xn
    pub fn set_x(&self, reg_index: u32, value: u64) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_x(*handle, reg_index, value) }
    }

    /// Read SP
    pub fn get_sp(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_sp(*handle) }
    }

    /// Write SP
    pub fn set_sp(&self, value: u64) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_sp(*handle, value) }
    }

    /// Read PC
    pub fn get_pc(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_pc(*handle) }
    }

    /// Write PC
    pub fn set_pc(&self, value: u64) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_pc(*handle, value) }
    }

    /// Write a 32-bit value to emulated memory
    pub fn write_u32(&self, vaddr: u64, value: u32) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_write_u32(*handle, vaddr, value) }
    }
}

impl Drop for DynarmicCPU {
    fn drop(&mut self) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_destroy_instance(*handle) };
    }
}

impl Clone for DynarmicCPU {
    fn clone(&self) -> Self {
        DynarmicCPU {
            handle: Arc::clone(&self.handle),
            _memory: Arc::clone(&self._memory),
        }
    }
}
