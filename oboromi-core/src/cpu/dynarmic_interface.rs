use std::sync::{Arc, Mutex};
use std::ptr::NonNull;
use crate::dynarmic_sys::*;

/// Thread-safe wrapper for Dynarmic CPU pointer
#[derive(Debug)]
struct DynarmicHandle {
    ptr: NonNull<DynarmicCPUIface>,
}

unsafe impl Send for DynarmicHandle {}
unsafe impl Sync for DynarmicHandle {}

impl DynarmicHandle {
    fn new() -> Option<Self> {
        let ptr = unsafe { dynarmic_create_instance() };
        NonNull::new(ptr).map(|ptr| DynarmicHandle { ptr })
    }

    fn as_ptr(&self) -> *mut DynarmicCPUIface {
        self.ptr.as_ptr()
    }
}

impl Drop for DynarmicHandle {
    fn drop(&mut self) {
        unsafe { dynarmic_destroy_instance(self.as_ptr()) };
    }
}

/// Dynarmic CPU interface for Rust
pub struct DynarmicCPU {
    handle: Arc<Mutex<DynarmicHandle>>,
}

impl DynarmicCPU {
    /// Create new Dynarmic CPU instance
    pub fn new() -> Option<Self> {
        DynarmicHandle::new().map(|handle| DynarmicCPU {
            handle: Arc::new(Mutex::new(handle)),
        })
    }

    /// Run CPU for given number of cycles
    pub fn run(&self, cycles: u64) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_run(handle.as_ptr(), cycles) }
    }
    
    /// NEW: Execute single instruction
    pub fn step(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_step(handle.as_ptr()) }
    }

    /// Get program counter
    pub fn get_pc(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_pc(handle.as_ptr()) }
    }

    /// Set program counter
    pub fn set_pc(&self, value: u64) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_pc(handle.as_ptr(), value) }
    }

    /// Read general purpose register X[index] (0-30)
    pub fn get_x(&self, index: u32) -> u64 {
        if index > 30 {
            panic!("Register index {} out of bounds (0-30)", index);
        }
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_x(handle.as_ptr(), index) }
    }

    /// Write general purpose register X[index] (0-30)
    pub fn set_x(&self, index: u32, value: u64) {
        if index > 30 {
            panic!("Register index {} out of bounds (0-30)", index);
        }
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_x(handle.as_ptr(), index, value) }
    }

    /// Get stack pointer
    pub fn get_sp(&self) -> u64 {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_get_sp(handle.as_ptr()) }
    }

    /// Set stack pointer
    pub fn set_sp(&self, value: u64) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_set_sp(handle.as_ptr(), value) }
    }

    /// Write 32-bit value to memory
    pub fn write_u32(&self, addr: u64, value: u32) {
        let handle = self.handle.lock().unwrap();
        unsafe { dynarmic_write_u32(handle.as_ptr(), addr, value) }
    }
}

impl Clone for DynarmicCPU {
    fn clone(&self) -> Self {
        DynarmicCPU {
            handle: Arc::clone(&self.handle),
        }
    }
}
