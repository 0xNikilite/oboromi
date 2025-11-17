pub struct GpuState {
    // pub registers: [u32; 256],
    // pub predicates: u32,
    pub shared_memory: *mut u8,
    pub global_memory: *mut u8,
    pub pc: u64,
}

impl GpuState {
    pub fn new() -> Self {
        Self {
            // registers: [0; 256],
            // predicates: 0,
            shared_memory: core::ptr::null_mut(),
            global_memory: core::ptr::null_mut(),
            pc: 0,
        }
    }
}
