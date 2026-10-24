use std::sync::{Arc, Mutex};
use crate::cpu::interpreter::{Cpu, StepResult};
use crate::mmu::{Memory, Permission};

struct PureRustArm64Core {
    cpu: Cpu,
    mmu: Memory,
    shared_ptr: Option<(*mut u8, u64)>,
    halted: bool,
}

unsafe impl Send for PureRustArm64Core {}

/// Pure-Rust ARMv8.2-A CPU backend (drop-in replacement for UnicornCPU with zero C/CMake dependencies)
pub struct UnicornCPU {
    inner: Arc<Mutex<PureRustArm64Core>>,
    pub core_id: u32,
}

impl UnicornCPU {
    /// Create a new pure-Rust ARM64 CPU instance with 8MB of mapped memory
    pub fn new() -> Option<Self> {
        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        mmu.map_range(0x0, 8 * 1024 * 1024, Permission::READ_WRITE_EXECUTE);
        cpu.regs.sp = (8 * 1024 * 1024) - 0x1000;

        Some(Self {
            inner: Arc::new(Mutex::new(PureRustArm64Core {
                cpu,
                mmu,
                shared_ptr: None,
                halted: false,
            })),
            core_id: 0,
        })
    }

    /// Create a new ARM64 CPU instance backed by a shared memory buffer across cores
    ///
    /// # Safety
    /// The caller must ensure `memory_ptr` is valid for the lifetime of this CPU
    /// and has at least `memory_size` bytes.
    pub unsafe fn new_with_shared_mem(core_id: u32, memory_ptr: *mut u8, memory_size: u64) -> Option<Self> {
        let mut cpu = Cpu::new();
        let mmu = Memory::new();
        let stack_top = memory_size.saturating_sub(core_id as u64 * 0x100000);
        cpu.regs.sp = stack_top;

        Some(Self {
            inner: Arc::new(Mutex::new(PureRustArm64Core {
                cpu,
                mmu,
                shared_ptr: Some((memory_ptr, memory_size)),
                halted: false,
            })),
            core_id,
        })
    }

    /// Run the core until halt, RET, BRK, or error
    pub fn run(&self) -> u64 {
        let mut core = self.inner.lock().unwrap();
        core.halted = false;
        let max_steps = 100_000usize;

        for _ in 0..max_steps {
            if core.halted {
                return 1;
            }
            let pc = core.cpu.regs.pc;
            let raw = match core.mmu.read_u32(pc) {
                Ok(v) => v,
                Err(_) => return 1,
            };
            // Stop cleanly on BRK (0xD4200000..0xD43FFFFF) or uninitialized zero word
            if raw == 0 || (raw & 0xFFE0_001F) == 0xD420_0000 {
                return 1;
            }

            let PureRustArm64Core { ref mut cpu, ref mut mmu, .. } = *core;
            match cpu.step(mmu) {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Svc(_)) | Ok(StepResult::Halt(_)) => return 1,
                Err(_) => return 0,
            }
        }
        1
    }

    /// Execute a single step
    pub fn step(&self) -> u64 {
        let mut core = self.inner.lock().unwrap();
        let PureRustArm64Core { ref mut cpu, ref mut mmu, .. } = *core;
        match cpu.step(mmu) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }

    /// Halt execution
    pub fn halt(&self) {
        let mut core = self.inner.lock().unwrap();
        core.halted = true;
    }

    /// Read register Xn (0-30)
    pub fn get_x(&self, reg_index: u32) -> u64 {
        let core = self.inner.lock().unwrap();
        if reg_index > 30 {
            return 0;
        }
        core.cpu.regs.read_x(reg_index as u8)
    }

    /// Write register Xn (0-30)
    pub fn set_x(&self, reg_index: u32, value: u64) {
        let mut core = self.inner.lock().unwrap();
        if reg_index <= 30 {
            core.cpu.regs.write_x(reg_index as u8, value);
        }
    }

    /// Read SP
    pub fn get_sp(&self) -> u64 {
        let core = self.inner.lock().unwrap();
        core.cpu.regs.sp
    }

    /// Write SP
    pub fn set_sp(&self, value: u64) {
        let mut core = self.inner.lock().unwrap();
        core.cpu.regs.sp = value;
    }

    /// Read PC
    pub fn get_pc(&self) -> u64 {
        let core = self.inner.lock().unwrap();
        core.cpu.regs.pc
    }

    /// Write PC
    pub fn set_pc(&self, value: u64) {
        let mut core = self.inner.lock().unwrap();
        core.cpu.regs.pc = value;
    }

    /// Write a 32-bit value to emulated memory
    pub fn write_u32(&self, vaddr: u64, value: u32) {
        let mut core = self.inner.lock().unwrap();
        if let Some((ptr, size)) = core.shared_ptr {
            if vaddr + 4 <= size {
                unsafe {
                    let bytes = value.to_le_bytes();
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(vaddr as usize), 4);
                }
            }
        } else {
            let _ = core.mmu.write_u32(vaddr, value);
        }
    }

    /// Read a 32-bit value from emulated memory
    pub fn read_u32(&self, vaddr: u64) -> u32 {
        let core = self.inner.lock().unwrap();
        if let Some((ptr, size)) = core.shared_ptr {
            if vaddr + 4 <= size {
                unsafe {
                    let slice = std::slice::from_raw_parts(ptr.add(vaddr as usize), 4);
                    u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]])
                }
            } else {
                0
            }
        } else {
            core.mmu.read_u32(vaddr).unwrap_or(0)
        }
    }

    /// Write a 64-bit value to emulated memory
    pub fn write_u64(&self, vaddr: u64, value: u64) {
        let mut core = self.inner.lock().unwrap();
        if let Some((ptr, size)) = core.shared_ptr {
            if vaddr + 8 <= size {
                unsafe {
                    let bytes = value.to_le_bytes();
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(vaddr as usize), 8);
                }
            }
        } else {
            let _ = core.mmu.write_u64(vaddr, value);
        }
    }

    /// Read a 64-bit value from emulated memory
    pub fn read_u64(&self, vaddr: u64) -> u64 {
        let core = self.inner.lock().unwrap();
        if let Some((ptr, size)) = core.shared_ptr {
            if vaddr + 8 <= size {
                unsafe {
                    let slice = std::slice::from_raw_parts(ptr.add(vaddr as usize), 8);
                    u64::from_le_bytes([
                        slice[0], slice[1], slice[2], slice[3],
                        slice[4], slice[5], slice[6], slice[7],
                    ])
                }
            } else {
                0
            }
        } else {
            core.mmu.read_u64(vaddr).unwrap_or(0)
        }
    }
}

unsafe impl Send for UnicornCPU {}
unsafe impl Sync for UnicornCPU {}
