use crate::cpu::UnicornCPU;
use std::pin::Pin;

pub const CORE_COUNT: usize = 8;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");
// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024; 
pub const MEMORY_BASE: u64 = 0x0;

pub struct CpuManager {
    pub cores: Vec<UnicornCPU>,
    // Pin prevents reallocation from invalidating pointers
    #[allow(dead_code)]
    pub shared_memory: Pin<Box<[u8]>>,
}

impl CpuManager {
    pub fn new() -> Self {
        // Allocate 12GB of zeroed memory
        // note: on modern OSs, this is lazily allocated (virtual memory)
        // and won't consume physical RAM until written to.
        let shared_memory = Pin::new(vec![0u8; MEMORY_SIZE as usize].into_boxed_slice());
        let memory_ptr = shared_memory.as_ptr() as *mut u8;

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
