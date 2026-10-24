pub mod unicorn_interface;
pub use unicorn_interface::UnicornCPU;
pub mod cpu_manager;

// Pure-Rust ARMv8.2-A CPU + IEEE-754 FPU + 128-bit NEON SIMD + Basic-Block JIT
pub mod decoder;
pub mod instructions;
pub mod interpreter;
pub mod jit_cache;
pub mod registers;

pub use instructions::{Condition, Instruction};
pub use interpreter::{Cpu, CpuError, StepResult};
pub use jit_cache::JitBlockCache;
pub use registers::{CpuRegisters, Pstate};



