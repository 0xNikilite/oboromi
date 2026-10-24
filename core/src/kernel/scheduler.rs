#![allow(dead_code)]

use crate::cpu::{Cpu, StepResult};
use crate::mmu::Memory;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Created,
    Runnable,
    Running,
    Exited,
}

pub struct KThread {
    pub handle: u32,
    pub thread_id: u64,
    pub entry_pc: u64,
    pub arg_x0: u64,
    pub stack_top: u64,
    pub priority: i32,
    pub core_id: u32,
    pub state: ThreadState,
    pub cpu_context: Cpu,
}

/// Horizon OS 8-Core Cortex-A78C Thread Scheduler.
///
/// Manages guest `KThread` objects created via `svcCreateThread` (`0x08`),
/// started via `svcStartThread` (`0x09`), and terminated via `svcExitThread` (`0x0A`).
pub struct ThreadScheduler {
    next_handle: u32,
    next_thread_id: u64,
    pub threads: HashMap<u32, KThread>,
    pub total_threads_spawned: usize,
    pub total_threads_completed: usize,
}

impl Default for ThreadScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadScheduler {
    pub fn new() -> Self {
        Self {
            next_handle: 0x2000,
            next_thread_id: 2, // Thread ID 1 is the main application thread
            threads: HashMap::new(),
            total_threads_spawned: 0,
            total_threads_completed: 0,
        }
    }

    /// Creates a new `KThread` object (`svcCreateThread`).
    pub fn create_thread(
        &mut self,
        entry_pc: u64,
        arg_x0: u64,
        stack_top: u64,
        priority: i32,
        core_id: u32,
    ) -> u32 {
        let handle = self.next_handle;
        self.next_handle += 1;
        let thread_id = self.next_thread_id;
        self.next_thread_id += 1;

        let mut cpu_context = Cpu::new();
        cpu_context.regs.pc = entry_pc;
        cpu_context.regs.sp = stack_top;
        cpu_context.regs.write_x(0, arg_x0);

        let kthread = KThread {
            handle,
            thread_id,
            entry_pc,
            arg_x0,
            stack_top,
            priority,
            core_id: core_id % 8, // 8 Cortex-A78C cores (0..7)
            state: ThreadState::Created,
            cpu_context,
        };

        self.threads.insert(handle, kthread);
        self.total_threads_spawned += 1;
        handle
    }

    /// Starts a created `KThread` and executes its timeslice against shared guest `Memory` (`svcStartThread`).
    pub fn start_and_run_thread(&mut self, handle: u32, mmu: &mut Memory, max_cycles: usize) -> bool {
        let Some(thread) = self.threads.get_mut(&handle) else {
            return false;
        };
        thread.state = ThreadState::Running;

        for _ in 0..max_cycles {
            match thread.cpu_context.step(mmu) {
                Ok(StepResult::Svc(0x0A)) | Ok(StepResult::Svc(0x07)) => {
                    // 0x0A = svcExitThread, 0x07 = svcExitProcess
                    thread.state = ThreadState::Exited;
                    self.total_threads_completed += 1;
                    return true;
                }
                Ok(_) => {}
                Err(_) => {
                    thread.state = ThreadState::Exited;
                    return false;
                }
            }
        }

        thread.state = ThreadState::Runnable;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_multicore_worker_thread_execution() {
        let mut mmu = Memory::new();
        let mut sched = ThreadScheduler::new();

        // Map worker code at 0x80050000, worker stack at 0x70050000, shared data at 0x80060000
        let code_addr = 0x80050000u64;
        let stack_top = 0x70051000u64;
        let shared_addr = 0x80060000u64;
        mmu.map_range(code_addr, 4096, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(0x70050000, 4096, Permission::READ_WRITE);
        mmu.map_range(shared_addr, 4096, Permission::READ_WRITE);

        // Worker thread takes shared_addr in X0, writes 0xCAFE_BABE_2026_0002 to [X0], and calls SVC #0x0A (svcExitThread)
        let worker_code: [u32; 4] = [
            0xD2800041, // MOVZ X1, #2
            0xF9000001, // STR X1, [X0]
            0xD4000141, // SVC #0x0A (svcExitThread)
            0xD503201F, // NOP
        ];
        for (i, &insn) in worker_code.iter().enumerate() {
            mmu.write_u32(code_addr + (i * 4) as u64, insn).unwrap();
        }

        let handle = sched.create_thread(code_addr, shared_addr, stack_top, 44, 3);
        assert!(sched.start_and_run_thread(handle, &mut mmu, 100));
        assert_eq!(sched.total_threads_completed, 1);
        assert_eq!(mmu.read_u64(shared_addr).unwrap(), 2);
    }
}
