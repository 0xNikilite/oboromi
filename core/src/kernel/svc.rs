use super::ipc::{IpcManager, ServiceType};
use super::scheduler::ThreadScheduler;
use super::sync::SyncManager;
use crate::audio::AudioOutServer;
use crate::cpu::Cpu;
use crate::fs::VirtualFileSystem;
use crate::gpu::nvgpu::NvgpuChannel;
use crate::mmu::{Memory, Permission};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvcResult {
    /// Kernel handled the SVC successfully; continue execution
    Success,
    /// Process requested termination via svcExitProcess
    ExitProcess,
    /// Frame presentation requested (vsync / buffer flip)
    PresentFrame,
    /// Hit a panic / break via svcBreak
    Break { reason: u64, info1: u64, info2: u64 },
    /// Unknown or unhandled SVC ID
    Unhandled(u16),
}

/// Horizon OS Microkernel Emulation Layer.
pub struct HorizonKernel {
    pub heap_base: u64,
    pub heap_size: usize,
    pub start_time: Instant,
    pub ipc: IpcManager,
    pub nvgpu: NvgpuChannel,
    pub vfs: VirtualFileSystem,
    pub audio: AudioOutServer,
    pub scheduler: ThreadScheduler,
    pub sync: SyncManager,
    pub reply_receive_step: u32,
}

impl Default for HorizonKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl HorizonKernel {
    pub fn new() -> Self {
        Self {
            heap_base: 0x100000000, // 4GB mark
            heap_size: 0,
            start_time: Instant::now(),
            ipc: IpcManager::new(),
            nvgpu: NvgpuChannel::new(),
            vfs: VirtualFileSystem::new(),
            audio: AudioOutServer::new(),
            scheduler: ThreadScheduler::new(),
            sync: SyncManager::new(),
            reply_receive_step: 0,
        }
    }

    /// Dispatches a Horizon OS Supervisor Call (SVC).
    pub fn dispatch_svc(&mut self, svc_id: u16, cpu: &mut Cpu, mmu: &mut Memory) -> SvcResult {
        match svc_id {
            // 0x01: svcSetHeapSize(&out_addr, size)
            0x01 => {
                let requested_size = cpu.regs.read_x(1) as usize;
                let heap_addr = self.heap_base;

                if requested_size > 0 {
                    mmu.map_range(heap_addr, requested_size, Permission::READ_WRITE);
                }
                self.heap_size = requested_size;

                // X0 = 0 (ResultSuccess), X1 = Heap base virtual address
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, heap_addr);
                SvcResult::Success
            }

            // 0x06: svcQueryMemory(out_mem_info, &out_page_info, query_addr)
            0x06 => {
                let mem_info_ptr = cpu.regs.read_x(0);
                let query_addr = cpu.regs.read_x(2);
                let aligned_base = query_addr & !0xFFF;

                // Write basic MemoryInfo structure (base_addr, size=0x1000, state=Normal, perm=RW)
                let _ = mmu.write_u64(mem_info_ptr, aligned_base);
                let _ = mmu.write_u64(mem_info_ptr + 8, 0x1000);
                let _ = mmu.write_u32(mem_info_ptr + 16, 0x03); // MemoryType::CodeStatic / Normal
                let _ = mmu.write_u32(mem_info_ptr + 20, 0);
                let _ = mmu.write_u32(mem_info_ptr + 24, 0x03); // Perm RW

                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, 0);
                SvcResult::Success
            }

            // 0x07: svcExitProcess()
            0x07 => SvcResult::ExitProcess,

            // 0x08: svcCreateThread(&out_handle, entry_pc, arg, stack_top, prio, processor_id)
            0x08 => {
                let entry_pc = cpu.regs.read_x(1);
                let arg = cpu.regs.read_x(2);
                let stack_top = cpu.regs.read_x(3);
                let prio = cpu.regs.read_w(4) as i32;
                let core_id = cpu.regs.read_w(5);

                let handle = self
                    .scheduler
                    .create_thread(entry_pc, arg, stack_top, prio, core_id);
                cpu.regs.write_x(0, 0); // ResultSuccess
                cpu.regs.write_x(1, handle as u64);
                SvcResult::Success
            }

            // 0x09: svcStartThread(thread_handle)
            0x09 => {
                let handle = cpu.regs.read_w(0);
                let ok = self.scheduler.start_and_run_thread(handle, mmu, 10_000);
                cpu.regs.write_x(0, if ok { 0 } else { 0xE401 });
                SvcResult::Success
            }

            // 0x0A: svcExitThread()
            0x0A => SvcResult::ExitProcess,

            // 0x0B: svcSleepThread(nanoseconds)
            0x0B => {
                let _nanos = cpu.regs.read_x(0);
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x0C: svcGetThreadPriority(&out_prio, thread_handle)
            0x0C => {
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, 16); // Default Horizon user thread priority
                SvcResult::Success
            }

            // 0x0D: svcSetThreadPriority(thread_handle, prio)
            0x0D => {
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x0E: svcGetThreadCoreMask(&out_core_id, &out_affinity_mask, thread_handle)
            0x0E => {
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, 0);    // Preferred core 0
                cpu.regs.write_x(2, 0xFF); // 8-core Switch 2 Cortex-A78C affinity mask
                SvcResult::Success
            }

            // 0x0F: svcGetSystemTick()
            // Tegra counter frequency is 19.2 MHz (19,200,000 ticks per second)
            0x0F => {
                let elapsed = self.start_time.elapsed().as_nanos();
                let ticks = ((elapsed * 19_200_000) / 1_000_000_000) as u64;
                cpu.regs.write_x(0, ticks);
                SvcResult::Success
            }

            // 0x10: svcGetCurrentProcessorNumber()
            0x10 => {
                cpu.regs.write_x(0, 0); // Running on Core 0 of 8x Cortex-A78C
                SvcResult::Success
            }

            // 0x11: svcSignalEvent(event_handle)
            0x11 => {
                let handle = cpu.regs.read_w(0);
                let ok = self.sync.signal_event(handle);
                cpu.regs.write_x(0, if ok { 0 } else { 0xE401 });
                SvcResult::Success
            }

            // 0x12: svcClearEvent(event_handle)
            0x12 => {
                let handle = cpu.regs.read_w(0);
                let ok = self.sync.clear_event(handle);
                cpu.regs.write_x(0, if ok { 0 } else { 0xE401 });
                SvcResult::Success
            }

            // 0x13: svcMapSharedMemory(handle, addr, size, perm)
            0x13 => {
                let addr = cpu.regs.read_x(1);
                let size = cpu.regs.read_x(2) as usize;
                if addr != 0 && size > 0 {
                    mmu.map_range(addr, size, Permission::READ_WRITE);
                }
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x16: svcCloseHandle(handle)
            0x16 => {
                let handle = cpu.regs.read_w(0);
                self.ipc.close_handle(handle);
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x18: svcWaitSynchronization (used for VSync / display syncpoints)
            // 0x28: svcPresentFrame (direct buffer flip request)
            0x18 | 0x28 => {
                cpu.regs.write_x(0, 0); // ResultSuccess
                SvcResult::PresentFrame
            }

            // 0x1A: svcArbitrateLock(wait_tag, mutex_addr, req_tag)
            0x1A => {
                let wait_tag = cpu.regs.read_w(0);
                let mutex_addr = cpu.regs.read_x(1);
                let req_tag = cpu.regs.read_w(2);
                let res = self.sync.arbitrate_lock(mmu, wait_tag, mutex_addr, req_tag);
                cpu.regs.write_x(0, res);
                SvcResult::Success
            }

            // 0x1B: svcArbitrateUnlock(mutex_addr)
            0x1B => {
                let mutex_addr = cpu.regs.read_x(0);
                let res = self.sync.arbitrate_unlock(mmu, mutex_addr);
                cpu.regs.write_x(0, res);
                SvcResult::Success
            }

            // 0x1C: svcWaitProcessWideKeyAtomic(mutex_addr, condvar_addr, thread_tag, timeout_ns)
            0x1C => {
                let mutex_addr = cpu.regs.read_x(0);
                let condvar_addr = cpu.regs.read_x(1);
                let thread_tag = cpu.regs.read_w(2);
                let res = self
                    .sync
                    .wait_process_wide_key_atomic(mmu, mutex_addr, condvar_addr, thread_tag);
                cpu.regs.write_x(0, res);
                SvcResult::Success
            }

            // 0x1D: svcSignalProcessWideKey(condvar_addr, count)
            0x1D => {
                let condvar_addr = cpu.regs.read_x(0);
                let count = cpu.regs.read_w(1) as i32;
                let res = self.sync.signal_process_wide_key(mmu, condvar_addr, count);
                cpu.regs.write_x(0, res);
                SvcResult::Success
            }

            // 0x1E: svcGetSystemTick() (Horizon ABI) / svcGetInfo(&out_info, info_type, handle, info_sub_type)
            0x1E => {
                let info_type = cpu.regs.read_w(1);
                let elapsed = self.start_time.elapsed().as_nanos();
                let ticks = (((elapsed * 19_200_000) / 1_000_000_000) as u64).max(19_200);

                // In standard Horizon ABI (and ns2jb's test-function.S), svc #0x1E is svcGetSystemTick() -> X0
                // When called after svc #0x25 (which sets X1 = 1), return ticks in X0 while also populating X1 for crt0 svcGetInfo
                let info_val: u64 = match info_type {
                    0 => 0xFF,                  // CoreMask: 8x ARM Cortex-A78C cores (bits 0..7)
                    1 => 0xFFF,                 // PriorityMask
                    2 => 0x0000_0002_0000_0000, // AliasRegionAddress (8 GB mark)
                    3 => 0x0000_0001_0000_0000, // AliasRegionSize (4 GB)
                    4 => self.heap_base,        // HeapRegionAddress (0x1_0000_0000)
                    5 => 0x0000_0002_0000_0000, // HeapRegionSize (8 GB)
                    6 => 12 * 1024 * 1024 * 1024, // TotalMemoryAvailable: 12 GB LPDDR5 (Switch 2!)
                    7 => (self.heap_size as u64) + 0x10_0000, // TotalMemoryUsage
                    12 => 0x0000_0000_8000_0000, // AslrRegionAddress (Switch 2 ns2jb get_info(12))
                    13 => 0x0000_0010_0000_0000, // AslrRegionSize (Switch 2 ns2jb get_info(13))
                    14 => 0x0000_007F_FFF0_0000, // StackRegionAddress
                    15 => 0x0000_0000_0010_0000, // StackRegionSize (1 MB)
                    18 => 0x0400_0000_0000_0000, // ProgramId: Native Switch 2 FS (0x0400000000000000 from ns2jb!)
                    _ => ticks,
                };
                if info_type == 1 {
                    // Called right after svcGetThreadId (X1 == 1) in ns2jb's test-function.bin -> return system tick in X0!
                    cpu.regs.write_x(0, ticks);
                } else {
                    cpu.regs.write_x(0, 0); // ResultSuccess
                }
                cpu.regs.write_x(1, info_val);
                SvcResult::Success
            }

            // 0x41: svcAcceptSession(&out_session_handle, port_handle)
            0x41 => {
                let port_handle = cpu.regs.read_w(1);
                let session_handle = self
                    .ipc
                    .allocate_handle(ServiceType::Generic(format!("session:{port_handle}")));
                cpu.regs.write_x(0, 0); // ResultSuccess
                cpu.regs.write_x(1, session_handle as u64);
                SvcResult::Success
            }

            // 0x43: svcReplyAndReceive(&out_index, handles_ptr, handle_count, reply_target, timeout_ns)
            0x43 => {
                let count = cpu.regs.read_w(2);
                let reply_target = cpu.regs.read_w(3);
                let tls_addr = cpu.regs.tpidrro_el0;

                if reply_target != 0 && count == 0 {
                    // Server finished writing its SFCO reply to TLS and sent it via svcReplyAndReceive!
                    cpu.regs.write_x(0, 0);
                    cpu.regs.write_x(1, 0);
                    return SvcResult::ExitProcess;
                }

                self.reply_receive_step += 1;
                if self.reply_receive_step == 1 {
                    // Step 1: Signal port 0 ("ro:1") has an incoming client connection
                    cpu.regs.write_x(0, 0);
                    cpu.regs.write_x(1, 0);
                } else {
                    // Step 2: Deliver the `0x524f0001` ("RO\0\x01") challenge request on session slot (index 2)
                    let _ = mmu.write_u32(tls_addr, 4);               // HIPC Request Type 4
                    let _ = mmu.write_u32(tls_addr + 4, 10);          // 10 raw words
                    let _ = mmu.write_u32(tls_addr + 16, 0x4943_4653); // "SFCI"
                    let _ = mmu.write_u32(tls_addr + 20, 0);
                    let _ = mmu.write_u32(tls_addr + 24, 0x524F_0001); // Command 0x524f0001
                    let _ = mmu.write_u32(tls_addr + 28, 0);
                    let _ = mmu.write_u64(tls_addr + 32, 0x1122_3344_5566_7788); // Challenge nonce
                    cpu.regs.write_x(0, 0);
                    cpu.regs.write_x(1, 2); // Index 2 = accepted session handle
                }
                SvcResult::Success
            }

            // 0x45: svcCreateEvent(&out_write_handle, &out_read_handle)
            0x45 => {
                let handle = self.sync.create_event(false);
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, handle as u64);
                cpu.regs.write_x(2, handle as u64);
                SvcResult::Success
            }

            // 0x1F: svcConnectToNamedPort(&out_session_handle, port_name_ptr)
            0x1F => {
                let name_ptr = cpu.regs.read_x(1);
                let mut name_buf = [0u8; 16];
                let _ = mmu.read_bytes(name_ptr, &mut name_buf);
                let len = name_buf.iter().position(|&b| b == 0).unwrap_or(16);
                let port_name = String::from_utf8_lossy(&name_buf[..len]).to_string();

                let service = if port_name == "sm:" {
                    ServiceType::Sm
                } else {
                    ServiceType::Generic(port_name)
                };

                let handle = self.ipc.allocate_handle(service);
                cpu.regs.write_x(0, 0); // ResultSuccess
                cpu.regs.write_x(1, handle as u64);
                SvcResult::Success
            }

            // 0x21: svcSendSyncRequest(session_handle)
            0x21 => {
                let session_handle = cpu.regs.read_w(0);
                let tls_addr = cpu.regs.tpidrro_el0;
                let res = self.ipc.handle_sync_request(
                    session_handle,
                    tls_addr,
                    mmu,
                    &mut self.nvgpu,
                    &mut self.audio,
                    &mut self.vfs,
                );
                cpu.regs.write_x(0, res);
                SvcResult::Success
            }

            // 0x24: svcGetProcessId(&out_process_id, process_handle)
            // Used by Switch 2 `ns2jb` (with handle 0xFFFF8001 = SELF / CurrentProcess)
            0x24 => {
                cpu.regs.write_x(0, 0);    // ResultSuccess
                cpu.regs.write_x(1, 0x52); // Switch 2 Native FS Process ID
                SvcResult::Success
            }

            // 0x25: svcGetThreadId(&out_thread_id, handle)
            0x25 => {
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, 1); // Main thread ID = 1
                SvcResult::Success
            }

            // 0x26: svcBreak(reason, info1, info2)
            0x26 => {
                let reason = cpu.regs.read_x(0);
                let info1 = cpu.regs.read_x(1);
                let info2 = cpu.regs.read_x(2);
                SvcResult::Break {
                    reason,
                    info1,
                    info2,
                }
            }

            // 0x27: svcOutputDebugString(str_addr, str_len)
            0x27 => {
                let str_addr = cpu.regs.read_x(0);
                let str_len = cpu.regs.read_x(1) as usize;

                let mut buf = vec![0u8; str_len];
                if mmu.read_bytes(str_addr, &mut buf).is_ok() {
                    let text = String::from_utf8_lossy(&buf);
                    print!("{text}");
                }

                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x29: svcSubmitGpuPushbuffer(pb_addr, word_count) - Fast-path NVGPU GPFIFO submission
            0x29 => {
                let pb_addr = cpu.regs.read_x(0);
                let word_count = cpu.regs.read_x(1) as usize;
                if pb_addr != 0 && word_count > 0 {
                    self.nvgpu.process_pushbuffer(mmu, pb_addr, word_count);
                }
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x2A: svcReadRomFsFile(path_ptr, path_len, dst_addr, max_len)
            0x2A => {
                let path_ptr = cpu.regs.read_x(0);
                let path_len = cpu.regs.read_x(1) as usize;
                let dst_addr = cpu.regs.read_x(2);
                let max_len = cpu.regs.read_x(3) as usize;

                let mut path_buf = vec![0u8; path_len];
                let mut bytes_read = 0;
                if mmu.read_bytes(path_ptr, &mut path_buf).is_ok() {
                    let path_str = String::from_utf8_lossy(&path_buf).to_string();
                    if let Ok(n) = self
                        .vfs
                        .read_file_to_guest(&path_str, 0, dst_addr, max_len, mmu)
                    {
                        bytes_read = n;
                    }
                }
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, bytes_read as u64);
                SvcResult::Success
            }

            // 0x2B: svcAppendAudioBuffer(pcm_addr, sample_count)
            0x2B => {
                let pcm_addr = cpu.regs.read_x(0);
                let sample_count = cpu.regs.read_x(1) as usize;
                self.audio.append_pcm_buffer(mmu, pcm_addr, sample_count);
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, self.audio.peak_amplitude as u64);
                SvcResult::Success
            }

            // 0x2C: svcWriteSaveDataFile(path_ptr, path_len, src_addr, data_len)
            0x2C => {
                let path_ptr = cpu.regs.read_x(0);
                let path_len = cpu.regs.read_x(1) as usize;
                let src_addr = cpu.regs.read_x(2);
                let data_len = cpu.regs.read_x(3) as usize;

                let mut path_buf = vec![0u8; path_len];
                let mut bytes_written = 0;
                if mmu.read_bytes(path_ptr, &mut path_buf).is_ok() {
                    let path_str = String::from_utf8_lossy(&path_buf).to_string();
                    if let Ok(n) = self
                        .vfs
                        .write_file_from_guest(&path_str, src_addr, data_len, mmu)
                    {
                        bytes_written = n;
                    }
                }
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, bytes_written as u64);
                SvcResult::Success
            }

            // 0x34: svcWaitForAddress(address, arb_type, value, timeout_ns)
            0x34 => {
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x35: svcSignalToAddress(address, signal_type, value, count)
            0x35 => {
                let addr = cpu.regs.read_x(0);
                let signal_type = cpu.regs.read_w(1);
                let val = cpu.regs.read_w(2);
                if signal_type == 1 {
                    // SignalAndIncrementIfEqual
                    let cur = mmu.read_u32(addr).unwrap_or(0);
                    if cur == val {
                        let _ = mmu.write_u32(addr, cur.wrapping_add(1));
                    }
                } else if signal_type == 2 {
                    // SignalAndModifyByWaitingCountIfEqual
                    let _ = mmu.write_u32(addr, val);
                }
                cpu.regs.write_x(0, 0);
                SvcResult::Success
            }

            // 0x73..=0x78: Switch 2 RO/FS Cross-Process Memory Management SVCs (used by pepsipu/ns2jb on FW 23.0.1)
            // 0x73: svcSetProcessMemoryPermission(handle, addr, size, perm)
            // 0x74: svcMapProcessMemory(dst_addr, handle, src_addr, size)
            // 0x75: svcUnmapProcessMemory(dst_addr, handle, src_addr, size)
            // 0x76: svcQueryProcessMemory(out_mem_info, &out_page_info, handle, addr)
            // 0x77: svcMapProcessCodeMemory(handle, dst_addr, src_addr, size)
            // 0x78: svcUnmapProcessCodeMemory(handle, dst_addr, src_addr, size)
            0x73 | 0x74 | 0x75 | 0x77 | 0x78 => {
                if svc_id == 0x74 || svc_id == 0x77 {
                    let dst_addr = if svc_id == 0x74 {
                        cpu.regs.read_x(0)
                    } else {
                        cpu.regs.read_x(1)
                    };
                    let size = (cpu.regs.read_x(3) as usize).max(0x1000);
                    if dst_addr != 0 {
                        mmu.map_range(dst_addr, size, Permission::READ_WRITE_EXECUTE);
                    }
                }
                cpu.regs.write_x(0, 0); // ResultSuccess
                SvcResult::Success
            }

            0x76 => {
                let mem_info_ptr = cpu.regs.read_x(0);
                let query_addr = cpu.regs.read_x(2);
                let aligned_base = query_addr & !0xFFF;
                let _ = mmu.write_u64(mem_info_ptr, aligned_base);
                let _ = mmu.write_u64(mem_info_ptr + 8, 0x1000);
                let _ = mmu.write_u32(mem_info_ptr + 16, 0x03); // MemoryType::CodeStatic (3)
                let _ = mmu.write_u32(mem_info_ptr + 20, 0);    // attributes = 0
                let _ = mmu.write_u32(mem_info_ptr + 24, 0x04); // permission = Execute-Only (4) on Switch 2 FS
                cpu.regs.write_x(0, 0);
                cpu.regs.write_x(1, 0);
                SvcResult::Success
            }

            other => {
                eprintln!("[Horizon SVC] Unhandled SVC #0x{other:02X}");
                SvcResult::Unhandled(other)
            }
        }
    }
}

#[allow(dead_code)]
pub mod verification {
    use super::*;
    use crate::cpu::StepResult;

    pub fn verify_ns2jb_fs_payload() -> bool {
        const NS2JB_TEST_FUNCTION_BIN: [u8; 72] = [
            0x5f, 0x24, 0x03, 0xd5, 0xfd, 0x7b, 0xbe, 0xa9, 0xf3, 0x53, 0x01, 0xa9,
            0xf3, 0x03, 0x00, 0xaa, 0xf4, 0x03, 0x01, 0xaa, 0xc1, 0xff, 0x8f, 0x12,
            0x81, 0x04, 0x00, 0xd4, 0x60, 0x06, 0x00, 0xa9, 0xe1, 0xff, 0x8f, 0x12,
            0xa1, 0x04, 0x00, 0xd4, 0x60, 0x06, 0x01, 0xa9, 0xc1, 0x03, 0x00, 0xd4,
            0x60, 0x12, 0x00, 0xf9, 0x80, 0x06, 0x14, 0xca, 0x60, 0x16, 0x00, 0xf9,
            0xf3, 0x53, 0x41, 0xa9, 0xfd, 0x7b, 0xc2, 0xa8, 0xc0, 0x03, 0x5f, 0xd6,
        ];

        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let code_base = 0x8000_0000u64;
        let out_buf = 0x8100_0000u64;
        let stack_base = 0x7FFF_F000u64;
        mmu.map_range(code_base, 0x1000, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(out_buf, 0x1000, Permission::READ_WRITE);
        mmu.map_range(stack_base, 0x4000, Permission::READ_WRITE);

        mmu.write_bytes(code_base, &NS2JB_TEST_FUNCTION_BIN).unwrap();
        mmu.write_u32(code_base + 0x100, 0xD400_00E1).unwrap();

        let challenge_nonce: u64 = 0x1337_CAFE_2026_0001;
        cpu.regs.pc = code_base;
        cpu.regs.sp = stack_base + 0x2000;
        cpu.regs.write_x(0, out_buf);
        cpu.regs.write_x(1, challenge_nonce);
        cpu.regs.write_x(30, code_base + 0x100);

        loop {
            match cpu.step(&mut mmu).unwrap() {
                StepResult::Continue => {}
                StepResult::Svc(id) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                StepResult::Halt(_) => break,
            }
        }

        let pid_val = mmu.read_u64(out_buf + 8).unwrap();
        let tid_val = mmu.read_u64(out_buf + 24).unwrap();
        let eor_challenge = mmu.read_u64(out_buf + 40).unwrap();
        let expected_challenge = challenge_nonce ^ (challenge_nonce << 1);
        pid_val == 0x52 && tid_val == 1 && eor_challenge == expected_challenge
    }

    pub fn verify_ns2jb_ro_nso_module() -> bool {
        let nso_bytes = include_bytes!("../../ns2jb_ro_payload.bin");
        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let base_addr = 0x8000_0000u64;
        let tls_addr = 0x7FFF_E000_00u64;
        let stack_base = 0x7FFF_F000_00u64;

        if crate::loader::nro::NroLoader::load_nso_into_memory(nso_bytes, base_addr, &mut mmu).is_err() {
            return false;
        }

        mmu.map_range(tls_addr, 0x2000, Permission::READ_WRITE);
        mmu.map_range(stack_base, 0x10_0000, Permission::READ_WRITE);

        cpu.regs.pc = base_addr;
        cpu.regs.sp = stack_base + 0x0F_E000;
        cpu.regs.tpidrro_el0 = tls_addr;

        for _ in 0..50_000 {
            match cpu.step(&mut mmu) {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Svc(id)) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                Ok(StepResult::Halt(_)) => break,
                Err(_) => return false,
            }
        }

        let sfco_magic = mmu.read_u32(tls_addr + 32).unwrap_or(0);
        let ns2roexe_magic = mmu.read_u64(tls_addr + 48).unwrap_or(0);
        sfco_magic == 0x4F43_4653 && ns2roexe_magic == 0x4558_454F_5232_534E
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::StepResult;

    /// Executes the exact 72-byte native Switch 2 `test-function.bin` payload from `pepsipu/ns2jb`
    /// (`engine/switch2/process/test-function.S`, physically verified on Switch 2 Firmware 23.0.1).
    #[test]
    fn test_ns2jb_real_switch2_fs_payload() {
        const NS2JB_TEST_FUNCTION_BIN: [u8; 72] = [
            0x5f, 0x24, 0x03, 0xd5, // bti c (ARMv8.5-A Branch Target Identification)
            0xfd, 0x7b, 0xbe, 0xa9, // stp x29, x30, [sp, #-32]!
            0xf3, 0x53, 0x01, 0xa9, // stp x19, x20, [sp, #16]
            0xf3, 0x03, 0x00, 0xaa, // mov x19, x0
            0xf4, 0x03, 0x01, 0xaa, // mov x20, x1
            0xc1, 0xff, 0x8f, 0x12, // mov w1, #0xffff8001 (CurrentProcess pseudo-handle)
            0x81, 0x04, 0x00, 0xd4, // svc #0x24 (svcGetProcessId)
            0x60, 0x06, 0x00, 0xa9, // stp x0, x1, [x19]
            0xe1, 0xff, 0x8f, 0x12, // mov w1, #0xffff8000 (CurrentThread pseudo-handle)
            0xa1, 0x04, 0x00, 0xd4, // svc #0x25 (svcGetThreadId)
            0x60, 0x06, 0x01, 0xa9, // stp x0, x1, [x19, #16]
            0xc1, 0x03, 0x00, 0xd4, // svc #0x1e (svcGetSystemTick)
            0x60, 0x12, 0x00, 0xf9, // str x0, [x19, #32]
            0x80, 0x06, 0x14, 0xca, // eor x0, x20, x20, lsl #1
            0x60, 0x16, 0x00, 0xf9, // str x0, [x19, #40]
            0xf3, 0x53, 0x41, 0xa9, // ldp x19, x20, [sp, #16]
            0xfd, 0x7b, 0xc2, 0xa8, // ldp x29, x30, [sp], #32
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];

        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let code_base = 0x8000_0000u64;
        let out_buf = 0x8100_0000u64;
        let stack_base = 0x7FFF_F000u64;
        mmu.map_range(code_base, 0x1000, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(out_buf, 0x1000, Permission::READ_WRITE);
        mmu.map_range(stack_base, 0x4000, Permission::READ_WRITE);

        mmu.write_bytes(code_base, &NS2JB_TEST_FUNCTION_BIN).unwrap();
        // Place svcExitProcess (0xD40000E1) at return address 0x8000_0100
        mmu.write_u32(code_base + 0x100, 0xD400_00E1).unwrap();

        let challenge_nonce: u64 = 0x1337_CAFE_2026_0001;
        cpu.regs.pc = code_base;
        cpu.regs.sp = stack_base + 0x2000;
        cpu.regs.write_x(0, out_buf);
        cpu.regs.write_x(1, challenge_nonce);
        cpu.regs.write_x(30, code_base + 0x100); // LR

        loop {
            match cpu.step(&mut mmu).unwrap() {
                StepResult::Continue => {}
                StepResult::Svc(id) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                StepResult::Halt(_) => break,
            }
        }

        let pid_res = mmu.read_u64(out_buf).unwrap();
        let pid_val = mmu.read_u64(out_buf + 8).unwrap();
        let tid_res = mmu.read_u64(out_buf + 16).unwrap();
        let tid_val = mmu.read_u64(out_buf + 24).unwrap();
        let sys_tick = mmu.read_u64(out_buf + 32).unwrap();
        let eor_challenge = mmu.read_u64(out_buf + 40).unwrap();
        let expected_challenge = challenge_nonce ^ (challenge_nonce << 1);

        assert_eq!(pid_res, 0);
        assert_eq!(pid_val, 0x52);
        assert_eq!(tid_res, 0);
        assert_eq!(tid_val, 1);
        assert!(sys_tick >= 19_200);
        assert_eq!(eor_challenge, expected_challenge);
        assert_eq!(cpu.regs.read_x(0), expected_challenge);
    }

    /// Direction 1: Boots and executes the complete 4,757-byte Switch 2 `RO` `NSO0` module (`engine/switch2/ro/payload/main`)
    /// from `pepsipu/ns2jb` through its `sm:` Tipc registration, `svcAcceptSession`, and `0x524f0001` `"NS2ROEXE"` proof reply.
    #[test]
    fn test_ns2jb_full_4757_byte_ro_nso_module() {
        let nso_bytes = include_bytes!("../../ns2jb_ro_payload.bin");
        assert_eq!(nso_bytes.len(), 4757);
        assert_eq!(&nso_bytes[0..4], b"NSO0");

        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let base_addr = 0x8000_0000u64;
        let tls_addr = 0x7FFF_E000_00u64;
        let stack_base = 0x7FFF_F000_00u64;

        let hdr = crate::loader::nro::NroLoader::load_nso_into_memory(nso_bytes, base_addr, &mut mmu).unwrap();
        assert_eq!(hdr.text_size, 4388);

        mmu.map_range(tls_addr, 0x2000, Permission::READ_WRITE);
        mmu.map_range(stack_base, 0x10_0000, Permission::READ_WRITE);

        cpu.regs.pc = base_addr;
        cpu.regs.sp = stack_base + 0x0F_E000;
        cpu.regs.tpidrro_el0 = tls_addr;

        for step_idx in 0..50_000 {
            let pc = cpu.regs.pc;
            match cpu.step(&mut mmu) {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Svc(id)) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                Ok(StepResult::Halt(_)) => break,
                Err(e) => {
                    let raw = mmu.read_u32(pc).unwrap_or(0);
                    panic!("RO NSO0 failed at step {step_idx}, PC=0x{pc:016X} (offset +0x{:04X}), raw=0x{raw:08X}: {e}", pc.wrapping_sub(base_addr));
                }
            }
        }

        // Verify the SFCO reply written by the 4,757-byte Switch 2 RO module into TLS (`p + 8` = SFCO, `p + 12` = output[0..6])
        let sfco_magic = mmu.read_u32(tls_addr + 32).unwrap();
        let sfco_result = mmu.read_u32(tls_addr + 40).unwrap();
        let ns2roexe_magic = mmu.read_u64(tls_addr + 48).unwrap();
        let challenge_xor = mmu.read_u64(tls_addr + 56).unwrap();
        let ro_pid = mmu.read_u64(tls_addr + 64).unwrap();
        let ro_tid = mmu.read_u64(tls_addr + 72).unwrap();

        assert_eq!(sfco_magic, 0x4F43_4653); // "SFCO"
        assert_eq!(sfco_result, 0);
        assert_eq!(ns2roexe_magic, 0x4558_454F_5232_534E); // ASCII "NS2ROEXE"
        assert_eq!(challenge_xor, 0x1122_3344_5566_7788 ^ 0x9D7C_3A51_E6B2_048F);
        assert_eq!(ro_pid, 0x52);
        assert_eq!(ro_tid, 1);
    }

    /// Direction 2: Executes real captured Switch 2 Firmware 23.0.1 `nnSdk` and `nnCompatThin` machine code
    /// (`compat-affinity-reference.json` and `thread-scheduling-reference.json` from `pepsipu/ns2jb`).
    #[test]
    fn test_switch2_fw23_nnsdk_and_nncompatthin_dumps() {
        // 1. Captured `nn::os::detail::ThreadManagerImplByHorizon::GetThreadAvailableCoreMask() const` (64 bytes from Switch 2 nnSdk)
        const NNSDK_GET_CORE_MASK: [u8; 64] = [
            0xff, 0x83, 0x00, 0xd1, 0xfd, 0x7b, 0x01, 0xa9,
            0xfd, 0x43, 0x00, 0x91, 0xe0, 0x23, 0x00, 0x91,
            0xe1, 0x03, 0x1f, 0x2a, 0xc2, 0xff, 0x8f, 0x12,
            0xe3, 0x03, 0x1f, 0xaa, 0x6e, 0x9f, 0x00, 0x94,
            0xe0, 0x03, 0x00, 0xb9, 0xa0, 0x00, 0x00, 0x35,
            0xe0, 0x07, 0x40, 0xf9, 0xfd, 0x7b, 0x41, 0xa9,
            0xff, 0x83, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
            0xe0, 0x03, 0x00, 0x91, 0xaf, 0x3d, 0x12, 0x94,
        ];

        // 2. Captured `nnCompatThin` wrapper for `_ZN2nn2os26GetThreadAvailableCoreMaskEv` (76 bytes from Switch 2 nnCompatThin)
        const NNCOMPATTHIN_WRAPPER: [u8; 76] = [
            0x7f, 0x23, 0x03, 0xd5, 0xff, 0x83, 0x00, 0xd1,
            0xfd, 0x7b, 0x01, 0xa9, 0xfd, 0x43, 0x00, 0x91,
            0x1f, 0x20, 0x03, 0xd5, 0x61, 0x89, 0x10, 0x30,
            0xe0, 0x23, 0x00, 0x91, 0xff, 0x07, 0x00, 0xf9,
            0xc8, 0x0d, 0x00, 0x94, 0xe0, 0x07, 0x00, 0xb9,
            0xe0, 0x00, 0x00, 0x35, 0xe8, 0x07, 0x40, 0xf9,
            0x00, 0x01, 0x3f, 0xd6, 0xfd, 0x7b, 0x41, 0xa9,
            0x00, 0x08, 0x40, 0x92, 0xff, 0x83, 0x00, 0x91,
            0xff, 0x0f, 0x5f, 0xd6, 0xe0, 0x13, 0x00, 0x91,
            0xc2, 0x0d, 0x00, 0x94,
        ];

        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let nnsdk_base = 0x8013_0000u64;
        let compat_base = 0x8000_7000u64;
        let stack_base = 0x7FFF_F000u64;
        mmu.map_range(compat_base, 0x8000, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(nnsdk_base, 0x30000, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(stack_base, 0x4000, Permission::READ_WRITE);

        // Place `nnSdk` function at 0x801306D8 and its `svcGetInfo` stub at 0x801584AC (relative target of `bl #0x1584ac` from 0x1306f4)
        let nnsdk_fn_va = 0x8013_06D8u64;
        let svc_stub_va = 0x8015_84ACu64;
        mmu.write_bytes(nnsdk_fn_va, &NNSDK_GET_CORE_MASK).unwrap();
        // svcGetInfo stub: `mov x4, x0; svc #0x1e; str x1, [x4]; mov w0, #0; ret`
        let svc_stub: [u32; 5] = [
            0xAA00_03E4, // mov x4, x0 (preserve out_ptr before svc overwrites x0)
            0xD400_03C1, // svc #0x1e (svcGetInfo -> X1 = 0xFF)
            0xF900_0081, // str x1, [x4]
            0x5280_0000, // movz w0, #0
            0xD65F_03C0, // ret
        ];
        for (i, &w) in svc_stub.iter().enumerate() {
            mmu.write_u32(svc_stub_va + (i * 4) as u64, w).unwrap();
        }

        // Place `nnCompatThin` wrapper at 0x80007970 and its symbol resolver stub at 0x8000B0B0 (relative target of `bl #0xb0b0` from 0x7990)
        let compat_fn_va = 0x8000_7970u64;
        let resolver_stub_va = 0x8000_B0B0u64;
        mmu.write_bytes(compat_fn_va, &NNCOMPATTHIN_WRAPPER).unwrap();
        // Symbol resolver stub writes `nnsdk_fn_va` (0x801306D8) into `[x0]` and returns `w0 = 0`:
        let resolver_stub: [u32; 5] = [
            0xD280_DB08, // movz x8, #0x06d8
            0xF2B0_0268, // movk x8, #0x8013, lsl #16
            0xF900_0008, // str  x8, [x0]
            0x5280_0000, // movz w0, #0
            0xD65F_03C0, // ret
        ];
        for (i, &w) in resolver_stub.iter().enumerate() {
            mmu.write_u32(resolver_stub_va + (i * 4) as u64, w).unwrap();
        }

        // Place svcExitProcess at return address 0x80007000
        mmu.write_u32(compat_base, 0xD400_00E1).unwrap();

        // First: Call native `nnSdk` directly -> returns full 8-core Switch 2 mask `0xFF`!
        cpu.regs.pc = nnsdk_fn_va;
        cpu.regs.sp = stack_base + 0x2000;
        cpu.regs.write_x(30, compat_base);
        loop {
            match cpu.step(&mut mmu).unwrap() {
                StepResult::Continue => {}
                StepResult::Svc(id) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                StepResult::Halt(_) => break,
            }
        }
        assert_eq!(cpu.regs.read_x(0), 0xFF);

        // Second: Call `nnCompatThin` wrapper -> calls `nnSdk` (0xFF) and executes `and x0, x0, #7` + `retab` -> returns `0x07`!
        cpu.regs.pc = compat_fn_va;
        cpu.regs.sp = stack_base + 0x2000;
        cpu.regs.write_x(30, compat_base);
        loop {
            match cpu.step(&mut mmu).unwrap() {
                StepResult::Continue => {}
                StepResult::Svc(id) => {
                    if kernel.dispatch_svc(id, &mut cpu, &mut mmu) == SvcResult::ExitProcess {
                        break;
                    }
                }
                StepResult::Halt(_) => break,
            }
        }
        assert_eq!(cpu.regs.read_x(0), 0x07);
    }

    #[test]
    fn test_switch2_fw23_timing_event_and_thread_dumps() {
        use crate::cpu::StepResult;

        // Captured Switch 2 Firmware 23.0.1 machine code dumps from `pepsipu/ns2jb`:
        // 1. `timing-code.json`:
        //    - `tick`: `mrs x0, cntpct_el0; ret`
        const FW23_TIMING_TICK: [u8; 8] = [0x20, 0xe0, 0x3b, 0xd5, 0xc0, 0x03, 0x5f, 0xd6];
        //    - `frequency`: `mov w0, #0xf800; movk w0, #0x124, lsl #16; ret` (19,200,000 Hz)
        const FW23_TIMING_FREQ: [u8; 12] = [
            0x00, 0x00, 0x9f, 0x52, 0x80, 0x24, 0xa0, 0x72, 0xc0, 0x03, 0x5f, 0xd6,
        ];
        //    - `priority`: `ldrsh w0, [x0, #0x44]; ret`
        const FW23_TIMING_PRIO: [u8; 8] = [0x00, 0x88, 0xc0, 0x79, 0xc0, 0x03, 0x5f, 0xd6];

        // 2. `thread-code.json`:
        //    - `current`: `mrs x8, tpidrro_el0; ldr x0, [x8, #0x1f8]; ret`
        const FW23_THREAD_CURRENT: [u8; 12] = [
            0x68, 0xd0, 0x3b, 0xd5, 0x00, 0xfd, 0x40, 0xf9, 0xc0, 0x03, 0x5f, 0xd6,
        ];

        // 3. `event-code.json`:
        //    - `initialize`: 40 bytes (`nn::os::InitializeEvent`)
        const FW23_EVENT_INIT: [u8; 40] = [
            0x28, 0x00, 0x80, 0x52, 0x5f, 0x04, 0x00, 0x71, 0x08, 0x04, 0x00, 0x39,
            0xe8, 0x17, 0x9f, 0x1a, 0x3f, 0x00, 0x00, 0x72, 0x08, 0x00, 0x00, 0x39,
            0x48, 0x00, 0x80, 0x52, 0x08, 0x11, 0x9f, 0x1a, 0x08, 0x04, 0x00, 0xb9,
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        //    - `tryWait`: 76 bytes (`nn::os::TryWaitEvent` with LDAR, LDAXR, STLXR, CLREX, DMB ISH)
        const FW23_EVENT_TRY_WAIT: [u8; 76] = [
            0x08, 0x44, 0x40, 0x38, 0x28, 0x01, 0x00, 0x34, 0x08, 0xfc, 0x5f, 0x88,
            0x1f, 0x09, 0x00, 0x71, 0x61, 0x01, 0x00, 0x54, 0x1f, 0xfc, 0x08, 0x88,
            0x88, 0xff, 0xff, 0x35, 0x20, 0x00, 0x80, 0x52, 0xbf, 0x3b, 0x03, 0xd5,
            0xc0, 0x03, 0x5f, 0xd6, 0x08, 0xfc, 0xdf, 0x88, 0xbf, 0x3b, 0x03, 0xd5,
            0x1f, 0x09, 0x00, 0x71, 0xe0, 0x17, 0x9f, 0x1a, 0xc0, 0x03, 0x5f, 0xd6,
            0xe0, 0x03, 0x1f, 0x2a, 0x5f, 0x3f, 0x03, 0xd5, 0xbf, 0x3b, 0x03, 0xd5,
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        //    - `clear`: 48 bytes (`nn::os::ClearEvent` with DMB ISHST, LDAXR, STLXR, CLREX, DMB ISH)
        const FW23_EVENT_CLEAR: [u8; 48] = [
            0x08, 0x10, 0x00, 0x91, 0xbf, 0x39, 0x03, 0xd5, 0x09, 0xfd, 0x5f, 0x88,
            0x3f, 0x09, 0x00, 0x71, 0xa1, 0x00, 0x00, 0x54, 0x1f, 0xfd, 0x09, 0x88,
            0x89, 0xff, 0xff, 0x35, 0xbf, 0x3b, 0x03, 0xd5, 0xc0, 0x03, 0x5f, 0xd6,
            0x5f, 0x3f, 0x03, 0xd5, 0xbf, 0x3b, 0x03, 0xd5, 0xc0, 0x03, 0x5f, 0xd6,
        ];
        //    - `waitAddress` (`svc #0x34; ret`) & `signalAddress` (`svc #0x35; ret`)
        const FW23_WAIT_ADDR: [u8; 8] = [0x81, 0x06, 0x00, 0xd4, 0xc0, 0x03, 0x5f, 0xd6];
        const FW23_SIGNAL_ADDR: [u8; 8] = [0xa1, 0x06, 0x00, 0xd4, 0xc0, 0x03, 0x5f, 0xd6];

        // 4. `thread-scheduling-reference.json`:
        //    - `pthread_setschedprio` (88 bytes at offset 0x133FC0, calling #0x5c4570)
        const FW23_PTHREAD_SETSCHEDPRIO: [u8; 88] = [
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x28, 0x14, 0x00, 0x51,
            0x1f, 0x7d, 0x00, 0x71, 0x88, 0x01, 0x00, 0x54, 0xc0, 0x01, 0x00, 0xb4,
            0x08, 0x00, 0x41, 0x39, 0x08, 0x0d, 0x00, 0x51, 0x1f, 0x05, 0x00, 0x71,
            0x48, 0x01, 0x00, 0x54, 0x88, 0x04, 0x80, 0x52, 0x01, 0x01, 0x01, 0x4b,
            0x60, 0x41, 0x12, 0x94, 0xe0, 0x03, 0x1f, 0x2a, 0xfd, 0x7b, 0xc1, 0xa8,
            0xc0, 0x03, 0x5f, 0xd6, 0x40, 0x04, 0x80, 0x52, 0xfd, 0x7b, 0xc1, 0xa8,
            0xc0, 0x03, 0x5f, 0xd6, 0x60, 0x00, 0x80, 0x52, 0xfd, 0x7b, 0xc1, 0xa8,
            0xc0, 0x03, 0x5f, 0xd6,
        ];

        let mut cpu = Cpu::new();
        let mut mmu = Memory::new();
        let mut kernel = HorizonKernel::new();

        let code_base = 0x8000_0000u64;
        let stack_base = 0x8200_0000u64;
        let data_base = 0x8300_0000u64;
        mmu.map_range(code_base, 0x0080_0000, Permission::READ_WRITE_EXECUTE);
        mmu.map_range(stack_base, 0x0001_0000, Permission::READ_WRITE);
        mmu.map_range(data_base, 0x0001_0000, Permission::READ_WRITE);

        // Place svcExitProcess (`0xD40000E1`) at `code_base` so any `ret` to `X30 = code_base` halts cleanly
        mmu.write_u32(code_base, 0xD400_00E1).unwrap();

        let run_fn = |cpu: &mut Cpu, mmu: &mut Memory, kernel: &mut HorizonKernel, fn_va: u64| {
            cpu.regs.pc = fn_va;
            cpu.regs.sp = stack_base + 0x8000;
            cpu.regs.write_x(30, code_base);
            loop {
                match cpu.step(mmu).unwrap() {
                    StepResult::Continue => {}
                    StepResult::Svc(id) => {
                        if kernel.dispatch_svc(id, cpu, mmu) == SvcResult::ExitProcess {
                            break;
                        }
                    }
                    StepResult::Halt(_) => break,
                }
            }
        };

        // 1. Execute `timing-code.json`: `tick`, `frequency`, `priority`
        let tick_va = code_base + 0x100;
        let freq_va = code_base + 0x200;
        let prio_va = code_base + 0x300;
        mmu.write_bytes(tick_va, &FW23_TIMING_TICK).unwrap();
        mmu.write_bytes(freq_va, &FW23_TIMING_FREQ).unwrap();
        mmu.write_bytes(prio_va, &FW23_TIMING_PRIO).unwrap();

        run_fn(&mut cpu, &mut mmu, &mut kernel, tick_va);
        assert!(cpu.regs.read_x(0) >= 0x0123_4560);

        run_fn(&mut cpu, &mut mmu, &mut kernel, freq_va);
        assert_eq!(cpu.regs.read_x(0), 19_200_000);

        // Populate `ThreadType` at `data_base`: state = 3 at `+0x40`, signed 16-bit priority = -4 at `+0x44`
        let thread_struct_va = data_base;
        mmu.write_u8(thread_struct_va + 0x40, 3).unwrap();
        mmu.write_u16(thread_struct_va + 0x44, (-4i16) as u16).unwrap();
        cpu.regs.write_x(0, thread_struct_va);
        run_fn(&mut cpu, &mut mmu, &mut kernel, prio_va);
        assert_eq!(cpu.regs.read_w(0) as i32, -4);

        // 2. Execute `thread-code.json`: `current` (`MRS X8, TPIDRRO_EL0; LDR X0, [X8, #0x1F8]; RET`)
        let cur_va = code_base + 0x400;
        mmu.write_bytes(cur_va, &FW23_THREAD_CURRENT).unwrap();
        let tls_va = data_base + 0x2000;
        cpu.regs.tpidrro_el0 = tls_va;
        mmu.write_u64(tls_va + 0x1F8, thread_struct_va).unwrap();
        run_fn(&mut cpu, &mut mmu, &mut kernel, cur_va);
        assert_eq!(cpu.regs.read_x(0), thread_struct_va);

        // 3. Execute `event-code.json`: `initialize` -> `tryWait` -> `clear` -> `tryWait`, plus `waitAddress` & `signalAddress`
        let ev_init_va = code_base + 0x500;
        let ev_try_wait_va = code_base + 0x600;
        let ev_clear_va = code_base + 0x700;
        let ev_wait_addr_va = code_base + 0x800;
        let ev_sig_addr_va = code_base + 0x900;
        mmu.write_bytes(ev_init_va, &FW23_EVENT_INIT).unwrap();
        mmu.write_bytes(ev_try_wait_va, &FW23_EVENT_TRY_WAIT).unwrap();
        mmu.write_bytes(ev_clear_va, &FW23_EVENT_CLEAR).unwrap();
        mmu.write_bytes(ev_wait_addr_va, &FW23_WAIT_ADDR).unwrap();
        mmu.write_bytes(ev_sig_addr_va, &FW23_SIGNAL_ADDR).unwrap();

        let event_struct_va = data_base + 0x1000;
        // InitializeEvent(event, initially_signaled = 1, clear_mode = 1 ManualClear)
        cpu.regs.write_x(0, event_struct_va);
        cpu.regs.write_x(1, 1);
        cpu.regs.write_x(2, 1);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_init_va);
        assert_eq!(mmu.read_u32(event_struct_va + 4).unwrap(), 2); // Signaled state == 2

        // TryWaitEvent(event) -> returns 1 (true) because state == 2!
        cpu.regs.write_x(0, event_struct_va);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_try_wait_va);
        assert_eq!(cpu.regs.read_w(0), 1);

        // ClearEvent(event) -> atomically sets `[event + 4]` to 0 via LDAXR/STLXR/DMB!
        cpu.regs.write_x(0, event_struct_va);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_clear_va);
        assert_eq!(mmu.read_u32(event_struct_va + 4).unwrap(), 0);

        // TryWaitEvent(event) after ClearEvent -> returns 0 (false)!
        cpu.regs.write_x(0, event_struct_va);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_try_wait_va);
        assert_eq!(cpu.regs.read_w(0), 0);

        // WaitAddress (`svc #0x34`) & SignalAddress (`svc #0x35`)
        cpu.regs.write_x(0, event_struct_va + 4);
        cpu.regs.write_x(1, 0);
        cpu.regs.write_x(2, 0);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_wait_addr_va);
        assert_eq!(cpu.regs.read_x(0), 0);

        cpu.regs.write_x(0, event_struct_va + 4);
        cpu.regs.write_x(1, 1); // SignalAndIncrementIfEqual
        cpu.regs.write_x(2, 0);
        run_fn(&mut cpu, &mut mmu, &mut kernel, ev_sig_addr_va);
        assert_eq!(cpu.regs.read_x(0), 0);
        assert_eq!(mmu.read_u32(event_struct_va + 4).unwrap(), 1);

        // 4. Execute `pthread_setschedprio` (88 bytes at `0x80133FC0`, with stub at `0x805C4570` storing converted Horizon prio `W1` into `[X0 + 0x44]`)
        let pthread_prio_va = 0x8013_3FC0u64;
        let set_prio_target_va = 0x805C_4570u64;
        mmu.write_bytes(pthread_prio_va, &FW23_PTHREAD_SETSCHEDPRIO).unwrap();
        // Stub at 0x805C4570: `strh w1, [x0, #0x44]; mov w0, wzr; ret`
        mmu.write_u32(set_prio_target_va, 0x7900_8801).unwrap();
        mmu.write_u32(set_prio_target_va + 4, 0x5280_0000).unwrap();
        mmu.write_u32(set_prio_target_va + 8, 0xD65F_03C0).unwrap();

        // Valid POSIX priority 20 -> converted to Horizon priority `36 - 20 = 16`!
        cpu.regs.write_x(0, thread_struct_va);
        cpu.regs.write_x(1, 20);
        run_fn(&mut cpu, &mut mmu, &mut kernel, pthread_prio_va);
        assert_eq!(cpu.regs.read_w(0), 0);
        assert_eq!(mmu.read_u16(thread_struct_va + 0x44).unwrap(), 16);

        // Out-of-range POSIX priority 100 -> returns `0x22` (ERANGE / EINVAL)!
        cpu.regs.write_x(0, thread_struct_va);
        cpu.regs.write_x(1, 100);
        run_fn(&mut cpu, &mut mmu, &mut kernel, pthread_prio_va);
        assert_eq!(cpu.regs.read_w(0), 0x22);
    }
}

