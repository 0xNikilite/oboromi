use std::{any::Any, fmt};

pub mod dynarmic_interface;
pub mod unicorn_interface;
pub use dynarmic_interface::{DynarmicCPU, DynarmicGroup};
pub use unicorn_interface::UnicornCPU;
pub mod cpu_manager;
pub mod guest_ram;
pub use guest_ram::GuestRam;

pub const TICK_HZ: u64 = 19_200_000;

pub enum CpuContext {
    Dynarmic(dynarmic_interface::DynarmicContext),
    Unicorn(unicorn_engine::Context),
}

#[derive(Clone, Copy, Debug)]
pub struct CpuError;

impl fmt::Display for CpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str("CPU backend error") }
}

pub trait Cpu: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn run(&self) -> u64;
    fn step(&self) -> u64;
    fn halt(&self);
    fn get_x(&self, index: u32) -> u64;
    fn set_x(&self, index: u32, value: u64);
    fn get_sp(&self) -> u64;
    fn set_sp(&self, value: u64);
    fn get_pc(&self) -> u64;
    fn set_pc(&self, value: u64);
    fn svc_pending(&self) -> Option<u32>;
    fn set_tls(&self, value: u64);
    fn tls(&self) -> u64;
    fn read_mem(&self, address: u64, bytes: &mut [u8]) -> Result<(), CpuError>;
    fn write_mem(&self, address: u64, bytes: &[u8]) -> Result<(), CpuError>;
    fn ticks(&self) -> u64;
    fn idle_until(&self, deadline: u64);
    fn run_slice(&self, deadline: u64, core: usize) -> Result<bool, CpuError>;
    fn context(&self) -> Result<CpuContext, CpuError>;
    fn save(&self, context: &mut CpuContext) -> Result<(), CpuError>;
    fn restore(&self, context: &CpuContext) -> Result<(), CpuError>;
    fn thread_context(&self, entry: u64, arg: u64, stack: u64, tls: u64, handle: u32) -> Result<CpuContext, CpuError>;
    fn set_virtual_time(&self, enabled: bool);
    fn set_block_ticks(&self, ticks: u64);

    fn write_u32(&self, address: u64, value: u32) { let _ = self.write_mem(address, &value.to_le_bytes()); }
    fn read_u32(&self, address: u64) -> u32 {
        let mut bytes = [0; 4];
        self.read_mem(address, &mut bytes).map_or(0, |()| u32::from_le_bytes(bytes))
    }
    fn write_u64(&self, address: u64, value: u64) { let _ = self.write_mem(address, &value.to_le_bytes()); }
    fn read_u64(&self, address: u64) -> u64 {
        let mut bytes = [0; 8];
        self.read_mem(address, &mut bytes).map_or(0, |()| u64::from_le_bytes(bytes))
    }
}
