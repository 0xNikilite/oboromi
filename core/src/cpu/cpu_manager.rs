use crate::cpu::{Cpu, DynarmicCPU, DynarmicGroup, GuestRam, UnicornCPU};
use std::sync::Arc;

pub const CORE_COUNT: usize = 8;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");
// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024; 
pub const MEMORY_BASE: u64 = 0x0;

pub struct CpuManager {
    pub cores: Vec<Box<dyn Cpu>>,
    pub ram: Arc<GuestRam>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuBackend { Unicorn, Dynarmic }

impl CpuManager {
    pub fn new() -> Self {
        let backend = match std::env::var("OBOROMI_CPU").as_deref() { Ok("dynarmic") => CpuBackend::Dynarmic, _ => CpuBackend::Unicorn };
        Self::new_with_backend(backend)
    }

    pub fn new_with_backend(backend: CpuBackend) -> Self {
        let ram = GuestRam::new(MEMORY_SIZE as usize);
        let cores: Vec<Box<dyn Cpu>> = match backend {
            CpuBackend::Unicorn => {
                let first = UnicornCPU::new_with_shared_mem(0, ram.clone()).expect("CPU engine");
                let mut cores: Vec<Box<dyn Cpu>> = (1..CORE_COUNT as u32).map(|id| Box::new(first.logical_view(id)) as _).collect();
                cores.insert(0, Box::new(first));
                cores
            }
            CpuBackend::Dynarmic => {
                let group = DynarmicGroup::new().expect("Dynarmic monitor");
                (0..CORE_COUNT as u32).map(|id| Box::new(DynarmicCPU::new(&group, id, ram.clone()).expect("Dynarmic core")) as _).collect()
            }
        };
        Self { cores, ram }
    }

    pub fn run_all(&self) {
        for core in &self.cores {
            // just run one step for testing
            core.step();
        }
    }

    pub fn get_core(&self, id: usize) -> Option<&dyn Cpu> {
        self.cores.get(id).map(Box::as_ref)
    }
}
