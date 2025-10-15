pub struct GpuState {
    pub registers: [u32; 256],
    pub shared_memory: Vec<u8>,
    pub global_memory: Vec<u8>,
    pub pc: u64,
    pub predicates: [bool; 8],
}

impl GpuState {
    pub fn new() -> Self {
        Self {
            registers: [0; 256],
            shared_memory: vec![0; 48 * 1024],
            global_memory: vec![0; 8 * 1024 * 1024],
            pc: 0,
            predicates: [false; 8],
        }
    }
}
