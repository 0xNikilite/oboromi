#![allow(dead_code)]

use crate::audio::AudioOutServer;
use crate::fs::VirtualFileSystem;
use crate::gpu::nvgpu::{NvgpuChannel, NVGPU_IOCTL_CHANNEL_SUBMIT_GPFIFO};
use crate::mmu::Memory;
use std::collections::HashMap;

pub const CMIF_IN_MAGIC: u32 = 0x49434653;  // "SFCI"
pub const CMIF_OUT_MAGIC: u32 = 0x4F434653; // "SFCO"
pub const DEFAULT_TLS_ADDR: u64 = 0x7FFFE00000;
pub const TLS_SIZE: usize = 0x1000;

/// Horizon OS System Service Types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceType {
    /// Service Manager ("sm:")
    Sm,
    /// Nvidia GPU Driver ("nvdrv" / "nvdrv:a")
    Nvdrv,
    /// Video Interface ("vi:u" / "vi:m" / "vi:s")
    Vi,
    /// Human Interface Devices ("hid")
    Hid,
    /// Filesystem Proxy ("fsp-srv")
    FspSrv,
    /// Audio Out ("audout:u")
    AudOut,
    /// Applet OE Application Proxy ("appletOE")
    AppletOE,
    /// System Settings ("set:sys")
    SetSys,
    /// Generic stubbed service
    Generic(String),
}

/// Manages Horizon OS IPC handles and routes HIPC/CMIF messages to HLE services.
pub struct IpcManager {
    pub handles: HashMap<u32, ServiceType>,
    next_handle: u32,
    pub total_ipc_requests: u64,
}

impl Default for IpcManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IpcManager {
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            next_handle: 0x10001,
            total_ipc_requests: 0,
        }
    }

    pub fn allocate_handle(&mut self, service: ServiceType) -> u32 {
        let h = self.next_handle;
        self.next_handle += 1;
        self.handles.insert(h, service);
        h
    }

    pub fn close_handle(&mut self, handle: u32) -> bool {
        self.handles.remove(&handle).is_some()
    }

    /// Processes a synchronous HIPC/CMIF request located at `tls_addr` for `session_handle`.
    pub fn handle_sync_request(
        &mut self,
        session_handle: u32,
        tls_addr: u64,
        mmu: &mut Memory,
        nvgpu: &mut NvgpuChannel,
        audio: &mut AudioOutServer,
        _vfs: &mut VirtualFileSystem,
    ) -> u64 {
        self.total_ipc_requests += 1;

        let service = match self.handles.get(&session_handle).cloned() {
            Some(s) => s,
            None => return 0xF601, // Invalid handle result code
        };

        // Check for Switch 2 Tipc (Tiny IPC) commands on `sm:` (used by native Switch 2 RO/FS modules on FW 23.0.1)
        let tipc_hdr = mmu.read_u32(tls_addr).unwrap_or(0);
        if service == ServiceType::Sm && (tipc_hdr == 0x10 || tipc_hdr == 0x12) {
            if tipc_hdr == 0x10 {
                // Tipc 0x10: RegisterClient -> p[2] = 0 (ResultSuccess)
                let _ = mmu.write_u32(tls_addr + 8, 0);
                return 0;
            } else {
                // Tipc 0x12: RegisterService(space, name, max_sessions)
                let mut name_bytes = [0u8; 8];
                let _ = mmu.read_bytes(tls_addr + 12, &mut name_bytes);
                let len = name_bytes.iter().position(|&b| b == 0).unwrap_or(8);
                let srv_name = String::from_utf8_lossy(&name_bytes[..len]).to_string();
                let port_handle = self.allocate_handle(ServiceType::Generic(srv_name));

                let _ = mmu.write_u32(tls_addr, 0x12);
                let _ = mmu.write_u32(tls_addr + 4, 0x8000_0001);
                let _ = mmu.write_u32(tls_addr + 8, 0x20);
                let _ = mmu.write_u32(tls_addr + 12, port_handle);
                let _ = mmu.write_u32(tls_addr + 16, 0); // ResultSuccess
                return 0;
            }
        }

        // CMIF payload starts at TLS + 0x10
        let cmif_base = tls_addr + 0x10;
        let magic = mmu.read_u32(cmif_base).unwrap_or(0);
        if magic != CMIF_IN_MAGIC {
            // Write generic success even if raw HIPC control
            self.write_cmif_response(tls_addr, mmu, 0, &[], None);
            return 0;
        }

        let cmd_id = mmu.read_u32(cmif_base + 8).unwrap_or(0);
        let args_base = cmif_base + 0x10;

        match service {
            ServiceType::Sm => match cmd_id {
                // Command 0: Initialize
                0 => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[], None);
                    0
                }
                // Command 1: GetService(ServiceName u64)
                1 => {
                    let mut name_bytes = [0u8; 8];
                    let _ = mmu.read_bytes(args_base, &mut name_bytes);
                    let len = name_bytes.iter().position(|&b| b == 0).unwrap_or(8);
                    let srv_name = String::from_utf8_lossy(&name_bytes[..len]).to_string();

                    let target_service = match srv_name.as_str() {
                        "nvdrv" | "nvdrv:a" | "nvdrv:s" | "nvdrv:t" => ServiceType::Nvdrv,
                        "vi:u" | "vi:m" | "vi:s" => ServiceType::Vi,
                        "hid" => ServiceType::Hid,
                        "fsp-srv" => ServiceType::FspSrv,
                        "audout:u" => ServiceType::AudOut,
                        "appletOE" => ServiceType::AppletOE,
                        "set:sys" => ServiceType::SetSys,
                        other => ServiceType::Generic(other.to_string()),
                    };

                    let srv_handle = self.allocate_handle(target_service);
                    self.write_cmif_response(tls_addr, mmu, 0, &[srv_handle], Some(srv_handle));
                    0
                }
                _ => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[], None);
                    0
                }
            },

            ServiceType::Nvdrv => match cmd_id {
                // Command 0: Open -> returns (fd = 1, error = 0)
                0 => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[1, 0], None);
                    0
                }
                // Command 1: Ioctl(fd, ioctl_cmd, buf_addr, word_count)
                1 => {
                    let _fd = mmu.read_u32(args_base).unwrap_or(0);
                    let ioctl_cmd = mmu.read_u32(args_base + 4).unwrap_or(0);
                    let pb_addr = mmu.read_u64(args_base + 8).unwrap_or(0);
                    let word_count = mmu.read_u32(args_base + 16).unwrap_or(0) as usize;

                    if ioctl_cmd == NVGPU_IOCTL_CHANNEL_SUBMIT_GPFIFO && pb_addr != 0 && word_count > 0 {
                        nvgpu.process_pushbuffer(mmu, pb_addr, word_count);
                    } else if ioctl_cmd == 0x80B0_4705 && pb_addr != 0 {
                        // NVGPU_GPU_IOCTL_GET_CHARACTERISTICS: Write NVIDIA T239 Ampere GA10B (SM 8.7) characteristics
                        let _ = mmu.write_u32(pb_addr, 0x0170);      // arch = NVGPU_GPU_ARCH_GA100 (Ampere)
                        let _ = mmu.write_u32(pb_addr + 4, 0x000B);  // impl = NVGPU_GPU_IMPL_GA10B (Tegra T239)
                        let _ = mmu.write_u32(pb_addr + 8, 0x00A1);  // rev = A1
                        let _ = mmu.write_u32(pb_addr + 12, 12);     // num_gpc / 12 SMs (1536 CUDA cores)
                        let _ = mmu.write_u32(pb_addr + 16, 0x0807); // sm_arch_sm_version = SM 8.7
                    }

                    // Return nvdrv status = 0 (NVOS_STATUS_SUCCESS)
                    self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                    0
                }
                // Command 3: Initialize
                3 => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                    0
                }
                _ => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                    0
                }
            },

            ServiceType::AudOut => match cmd_id {
                // Command 0/1: OpenAudioOut / StartAudioOut -> returns (sample_rate=48000, channels=2)
                0 | 1 => {
                    audio.is_playing = true;
                    self.write_cmif_response(
                        tls_addr,
                        mmu,
                        0,
                        &[audio.sample_rate, audio.channels],
                        None,
                    );
                    0
                }
                // Command 3: AppendAudioOutBuffer(pcm_addr, sample_count)
                3 => {
                    let pcm_addr = mmu.read_u64(args_base).unwrap_or(0);
                    let sample_count = mmu.read_u32(args_base + 8).unwrap_or(0) as usize;
                    audio.append_pcm_buffer(mmu, pcm_addr, sample_count);
                    self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                    0
                }
                _ => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                    0
                }
            },

            ServiceType::AppletOE => match cmd_id {
                // Command 0: OpenApplicationProxy -> returns new AppletOE sub-session handle
                0 => {
                    let proxy_h = self.allocate_handle(ServiceType::AppletOE);
                    self.write_cmif_response(tls_addr, mmu, 0, &[proxy_h], Some(proxy_h));
                    0
                }
                // Command 1 / GetCurrentFocusState -> returns 1 (InFocus)
                _ => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[1], None);
                    0
                }
            },

            ServiceType::SetSys => match cmd_id {
                // Command 3 / 4: GetFirmwareVersion -> returns Switch 2 Firmware 23.0.1 ("Ounce" / ns2jb target)
                3 | 4 => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[23, 0, 1], None);
                    0
                }
                _ => {
                    self.write_cmif_response(tls_addr, mmu, 0, &[23, 0, 1], None);
                    0
                }
            },

            ServiceType::Vi | ServiceType::Hid | ServiceType::FspSrv | ServiceType::Generic(_) => {
                // Stubbed HLE service response: ResultSuccess (0)
                self.write_cmif_response(tls_addr, mmu, 0, &[0], None);
                0
            }
        }
    }

    /// Writes a standard CMIF ("SFCO") response frame back into the thread's TLS buffer.
    fn write_cmif_response(
        &self,
        tls_addr: u64,
        mmu: &mut Memory,
        result_code: u32,
        out_words: &[u32],
        move_handle: Option<u32>,
    ) {
        // HIPC Header at TLS + 0x00
        let has_handle = move_handle.is_some();
        let word0 = if has_handle { 0x80000004 } else { 0x00000004 };
        let word1 = (4 + out_words.len()) as u32;
        let _ = mmu.write_u32(tls_addr, word0);
        let _ = mmu.write_u32(tls_addr + 4, word1);

        if let Some(h) = move_handle {
            // Special header descriptor: 1 move handle at TLS + 0x0C
            let _ = mmu.write_u32(tls_addr + 8, 1 << 4);
            let _ = mmu.write_u32(tls_addr + 12, h);
        }

        // CMIF Out Header ("SFCO") at TLS + 0x10
        let cmif_base = tls_addr + 0x10;
        let _ = mmu.write_u32(cmif_base, CMIF_OUT_MAGIC);
        let _ = mmu.write_u32(cmif_base + 4, 0);
        let _ = mmu.write_u32(cmif_base + 8, result_code);
        let _ = mmu.write_u32(cmif_base + 12, 0);

        for (i, &w) in out_words.iter().enumerate() {
            let _ = mmu.write_u32(cmif_base + 16 + (i * 4) as u64, w);
        }
    }
}
