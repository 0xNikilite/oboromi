use crate::cpu::{GuestRam, UnicornCPU};
use std::sync::Arc;

pub const CORE_COUNT: usize = 8;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");
// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024; 
pub const MEMORY_BASE: u64 = 0x0;

pub struct CpuManager {
    pub cores: Vec<UnicornCPU>,
    pub ram: Arc<GuestRam>,
}

impl CpuManager {
    pub fn new() -> Self {
        let ram = GuestRam::new(MEMORY_SIZE as usize);
        let cores = (0..CORE_COUNT as u32)
            .map(|i| UnicornCPU::new_with_shared_mem(i, ram.clone()).unwrap_or_else(|| panic!("Failed to create Core {i}")))
            .collect();
        Self { cores, ram }
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
