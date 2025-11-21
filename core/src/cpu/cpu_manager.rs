use crate::cpu::unicorn_interface::UnicornCPU;

pub const CORE_COUNT: usize = 8;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");
// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024; 
pub const MEMORY_BASE: u64 = 0x0;

pub struct CpuManager {
    pub cores: Vec<UnicornCPU>,
    // We keep the memory here to ensure it lives as long as the CPUs
    // In a real implementation, this might be a separate Memory component
    pub shared_memory: Vec<u8>,
}

impl CpuManager {
    pub fn new() -> Self {
        // Allocate 12GB of zeroed memory
        // note: on modern OSs, this is lazily allocated (virtual memory)
        // and won't consume physical RAM until written to.
        let mut shared_memory = vec![0u8; MEMORY_SIZE as usize];
        let memory_ptr = shared_memory.as_mut_ptr();

        let mut cores = Vec::with_capacity(CORE_COUNT);

        for i in 0..CORE_COUNT {
            // Create CPU core sharing the same memory pointer
            // Safety: The memory is owned by CpuManager and pinned in place (Vec won't realloc if we don't push)
            // and UnicornCPU will use it for the lifetime of CpuManager.
            let cpu = unsafe { UnicornCPU::new_with_shared_mem(i as u32, memory_ptr, MEMORY_SIZE) };
            
            if let Some(cpu) = cpu {
                cores.push(cpu);
            } else {
                panic!("Failed to create Core {}", i);
            }
        }

        Self {
            cores,
            shared_memory,
        }
    }

    pub fn run_all(&self) {
        // for now, just step all cores sequentially (round-robin)
        // in the future, this would be threaded
        for (_i, core) in self.cores.iter().enumerate() {
            // just run one step for testing
            core.step();
        }
    }

    pub fn get_core(&self, id: usize) -> Option<&UnicornCPU> {
        self.cores.get(id)
    }
}
