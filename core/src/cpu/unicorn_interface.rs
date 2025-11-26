use std::sync::{Arc, Mutex};
use unicorn_engine::{Arch, Mode, Prot, RegisterARM64, Unicorn};

/// Safe wrapper for Unicorn CPU emulator
pub struct UnicornCPU {
    emu: Arc<Mutex<Unicorn<'static, ()>>>,
    pub core_id: u32,
}

impl UnicornCPU {
    /// Create a new Unicorn instance with 8MB of memory (Legacy/Test mode)
    pub fn new() -> Option<Self> {
        let mut emu = Unicorn::new(Arch::ARM64, Mode::LITTLE_ENDIAN)
            .map_err(|e| {
                eprintln!("Failed to create Unicorn instance: {e:?}");
                e
            })
            .ok()?;

        // Map 8MB of memory with full permissions (Legacy size)
        // This uses Unicorn's internal allocation
        emu.mem_map(0x0, 8 * 1024 * 1024, Prot::ALL)
            .map_err(|e| {
                eprintln!("Failed to map memory: {e:?}");
                e
            })
            .ok()?;

        // Initialize stack pointer
        let _ = emu.reg_write(RegisterARM64::SP, (8 * 1024 * 1024) - 0x1000);

        Some(Self {
            emu: Arc::new(Mutex::new(emu)),
            core_id: 0,
        })
    }

    /// Create a new Unicorn instance with shared memory
    /// 
    /// # Safety
    /// The caller must ensure `memory_ptr` is valid for the lifetime of this CPU
    /// and has at least `memory_size` bytes.
    pub unsafe fn new_with_shared_mem(core_id: u32, memory_ptr: *mut u8, memory_size: u64) -> Option<Self> {
        let mut emu = Unicorn::new(Arch::ARM64, Mode::LITTLE_ENDIAN)
            .map_err(|e| {
                eprintln!("Failed to create Unicorn instance for core {}: {:?}", core_id, e);
                e
            })
            .ok()?;

        // Map shared memory
        // unsafe because we are providing a raw pointer
        unsafe {
            emu.mem_map_ptr(0x0, memory_size, Prot::ALL, memory_ptr as *mut std::ffi::c_void)
                .map_err(|e| {
                    eprintln!("Failed to map shared memory for core {}: {:?}", core_id, e);
                    e
                })
                .ok()?;
        }

        // Initialize stack pointer to end of memory, offset by core ID to avoid collision
        // Give each core 1MB of stack space at the top of memory
        let stack_top = memory_size - (core_id as u64 * 0x100000);
        let _ = emu.reg_write(RegisterARM64::SP, stack_top);

        Some(Self {
            emu: Arc::new(Mutex::new(emu)),
            core_id,
        })
    }

    /// Run the core until halt or breakpoint
    pub fn run(&self) -> u64 {
        let mut emu = self.emu.lock().unwrap();
        let pc = emu.reg_read(RegisterARM64::PC).unwrap_or(0);

        // Run until we hit a BRK instruction or error
        match emu.emu_start(pc, 0xFFFF_FFFF_FFFF_FFFF, 0, 0) {
            Ok(_) => 1, // Success - normal completion
            Err(e) => {
                // BRK instruction causes an error, which is expected
                if format!("{e:?}").contains("EXCEPTION") {
                    1 // Success - terminated by BRK
                } else {
                    eprintln!("Emulation error: {e:?}");
                    0 // Failure - actual emulation error
                }
            }
        }
    }

    /// Execute a single step
    pub fn step(&self) -> u64 {
        let mut emu = self.emu.lock().unwrap();
        let pc = emu.reg_read(RegisterARM64::PC).unwrap_or(0);

        match emu.emu_start(pc, pc + 4, 0, 1) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }

    /// Halt execution
    pub fn halt(&self) {
        let mut emu = self.emu.lock().unwrap();
        let _ = emu.emu_stop();
    }

    /// Read register Xn (0-30)
    pub fn get_x(&self, reg_index: u32) -> u64 {
        let emu = self.emu.lock().unwrap();
        if reg_index > 30 {
            return 0;
        }

        let reg = match reg_index {
            0 => RegisterARM64::X0,
            1 => RegisterARM64::X1,
            2 => RegisterARM64::X2,
            3 => RegisterARM64::X3,
            4 => RegisterARM64::X4,
            5 => RegisterARM64::X5,
            6 => RegisterARM64::X6,
            7 => RegisterARM64::X7,
            8 => RegisterARM64::X8,
            9 => RegisterARM64::X9,
            10 => RegisterARM64::X10,
            11 => RegisterARM64::X11,
            12 => RegisterARM64::X12,
            13 => RegisterARM64::X13,
            14 => RegisterARM64::X14,
            15 => RegisterARM64::X15,
            16 => RegisterARM64::X16,
            17 => RegisterARM64::X17,
            18 => RegisterARM64::X18,
            19 => RegisterARM64::X19,
            20 => RegisterARM64::X20,
            21 => RegisterARM64::X21,
            22 => RegisterARM64::X22,
            23 => RegisterARM64::X23,
            24 => RegisterARM64::X24,
            25 => RegisterARM64::X25,
            26 => RegisterARM64::X26,
            27 => RegisterARM64::X27,
            28 => RegisterARM64::X28,
            29 => RegisterARM64::X29,
            30 => RegisterARM64::X30,
            _ => return 0,
        };

        emu.reg_read(reg).unwrap_or(0)
    }

    /// Write register Xn
    pub fn set_x(&self, reg_index: u32, value: u64) {
        let mut emu = self.emu.lock().unwrap();
        if reg_index > 30 {
            return;
        }

        let reg = match reg_index {
            0 => RegisterARM64::X0,
            1 => RegisterARM64::X1,
            2 => RegisterARM64::X2,
            3 => RegisterARM64::X3,
            4 => RegisterARM64::X4,
            5 => RegisterARM64::X5,
            6 => RegisterARM64::X6,
            7 => RegisterARM64::X7,
            8 => RegisterARM64::X8,
            9 => RegisterARM64::X9,
            10 => RegisterARM64::X10,
            11 => RegisterARM64::X11,
            12 => RegisterARM64::X12,
            13 => RegisterARM64::X13,
            14 => RegisterARM64::X14,
            15 => RegisterARM64::X15,
            16 => RegisterARM64::X16,
            17 => RegisterARM64::X17,
            18 => RegisterARM64::X18,
            19 => RegisterARM64::X19,
            20 => RegisterARM64::X20,
            21 => RegisterARM64::X21,
            22 => RegisterARM64::X22,
            23 => RegisterARM64::X23,
            24 => RegisterARM64::X24,
            25 => RegisterARM64::X25,
            26 => RegisterARM64::X26,
            27 => RegisterARM64::X27,
            28 => RegisterARM64::X28,
            29 => RegisterARM64::X29,
            30 => RegisterARM64::X30,
            _ => return,
        };

        let _ = emu.reg_write(reg, value);
    }

    /// Read SP
    pub fn get_sp(&self) -> u64 {
        let emu = self.emu.lock().unwrap();
        emu.reg_read(RegisterARM64::SP).unwrap_or(0)
    }

    /// Write SP
    pub fn set_sp(&self, value: u64) {
        let mut emu = self.emu.lock().unwrap();
        let _ = emu.reg_write(RegisterARM64::SP, value);
    }

    /// Read PC
    pub fn get_pc(&self) -> u64 {
        let emu = self.emu.lock().unwrap();
        emu.reg_read(RegisterARM64::PC).unwrap_or(0)
    }

    /// Write PC
    pub fn set_pc(&self, value: u64) {
        let mut emu = self.emu.lock().unwrap();
        let _ = emu.reg_write(RegisterARM64::PC, value);
    }

    /// Write a 32-bit value to emulated memory
    pub fn write_u32(&self, vaddr: u64, value: u32) {
        let mut emu = self.emu.lock().unwrap();
        let bytes = value.to_le_bytes();
        let _ = emu.mem_write(vaddr, &bytes);
    }

    /// Read a 32-bit value from emulated memory
    pub fn read_u32(&self, vaddr: u64) -> u32 {
        let emu = self.emu.lock().unwrap();
        let mut bytes = [0u8; 4];
        if emu.mem_read(vaddr, &mut bytes).is_ok() {
            u32::from_le_bytes(bytes)
        } else {
            0
        }
    }

    /// Write a 64-bit value to emulated memory
    pub fn write_u64(&self, vaddr: u64, value: u64) {
        let mut emu = self.emu.lock().unwrap();
        let bytes = value.to_le_bytes();
        let _ = emu.mem_write(vaddr, &bytes);
    }

    /// Read a 64-bit value from emulated memory
    pub fn read_u64(&self, vaddr: u64) -> u64 {
        let emu = self.emu.lock().unwrap();
        let mut bytes = [0u8; 8];
        if emu.mem_read(vaddr, &mut bytes).is_ok() {
            u64::from_le_bytes(bytes)
        } else {
            0
        }
    }
}

impl Clone for UnicornCPU {
    fn clone(&self) -> Self {
        // Shallow clone of the Arc, sharing the same underlying Unicorn instance
        // This allows multiple references to the same core
        Self {
            emu: self.emu.clone(),
            core_id: self.core_id,
        }
    }
}

unsafe impl Send for UnicornCPU {}
unsafe impl Sync for UnicornCPU {}
