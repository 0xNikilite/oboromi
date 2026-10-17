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
        let first = UnicornCPU::new_with_shared_mem(0, ram.clone()).expect("CPU engine");
        let mut cores = Vec::with_capacity(CORE_COUNT);
        for id in 1..CORE_COUNT as u32 { cores.push(first.logical_view(id)); }
        cores.insert(0, first);
        Self { cores, ram }
    }

    pub fn run_all(&self) {
        for core in &self.cores {
            // just run one step for testing
            core.step();
        }
    }

    pub fn get_core(&self, id: usize) -> Option<&UnicornCPU> {
        self.cores.get(id)
    }
}
