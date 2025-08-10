#![allow(non_camel_case_types)]
use std::ffi::c_void;

#[repr(C)]
pub struct DynarmicCPUIface {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn dynarmic_create_instance(memory_backend_ptr: *mut c_void) -> *mut DynarmicCPUIface;
    pub fn dynarmic_destroy_instance(cpu: *mut DynarmicCPUIface);
    pub fn dynarmic_run(cpu: *mut DynarmicCPUIface) -> u64;
    pub fn dynarmic_step(cpu: *mut DynarmicCPUIface) -> u64;
    pub fn dynarmic_halt(cpu: *mut DynarmicCPUIface);
    pub fn dynarmic_get_x(cpu: *mut DynarmicCPUIface, reg_index: u32) -> u64;
    pub fn dynarmic_set_x(cpu: *mut DynarmicCPUIface, reg_index: u32, value: u64);
    pub fn dynarmic_get_sp(cpu: *mut DynarmicCPUIface) -> u64;
    pub fn dynarmic_set_sp(cpu: *mut DynarmicCPUIface, value: u64);
    pub fn dynarmic_get_pc(cpu: *mut DynarmicCPUIface) -> u64;
    pub fn dynarmic_set_pc(cpu: *mut DynarmicCPUIface, value: u64);
    pub fn dynarmic_write_u32(cpu: *mut DynarmicCPUIface, vaddr: u64, value: u32);
}
