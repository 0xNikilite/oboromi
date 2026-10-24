#![allow(dead_code)]

use crate::mmu::{Memory, MemoryError, Permission};

pub const HID_SHARED_MEM_ADDR: u64 = 0x10000000;
pub const HID_SHARED_MEM_SIZE: usize = 0x40000; // 256 KB standard Horizon HID pool

/// Platform-independent host key codes for `oboromi-core` input translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    A,
    D,
    W,
    S,
    Space,
    J,
    K,
    U,
    I,
    Q,
    E,
    Z,
    C,
    Enter,
    Backspace,
    Escape,
}

/// Nintendo Switch & Switch 2 Controller Button Bitmask (NpadButton).
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpadButton {
    A = 1 << 0,
    B = 1 << 1,
    X = 1 << 2,
    Y = 1 << 3,
    LStick = 1 << 4,
    RStick = 1 << 5,
    L = 1 << 6,
    R = 1 << 7,
    ZL = 1 << 8,
    ZR = 1 << 9,
    Plus = 1 << 10,
    Minus = 1 << 11,
    DLeft = 1 << 12,
    DUp = 1 << 13,
    DRight = 1 << 14,
    DDown = 1 << 15,
}

/// Controller state for Player 1.
#[derive(Debug, Clone, Default)]
pub struct ControllerState {
    pub buttons: u64,
    pub analog_lx: i32,
    pub analog_ly: i32,
    pub analog_rx: i32,
    pub analog_ry: i32,
}

/// The Horizon Human Interface Device (HID) Service.
pub struct HidServer {
    pub shared_mem_addr: u64,
    pub state: ControllerState,
    pub sample_counter: u64,
}

impl Default for HidServer {
    fn default() -> Self {
        Self::new(HID_SHARED_MEM_ADDR)
    }
}

impl HidServer {
    pub fn new(shared_mem_addr: u64) -> Self {
        Self {
            shared_mem_addr,
            state: ControllerState::default(),
            sample_counter: 0,
        }
    }

    /// Initializes and maps the HID shared memory region in virtual memory.
    pub fn init_memory(&self, mmu: &mut Memory) {
        mmu.map_range(
            self.shared_mem_addr,
            HID_SHARED_MEM_SIZE,
            Permission::READ_WRITE,
        );
    }

    /// Translates host keyboard events into Nintendo Switch controller buttons.
    pub fn poll_keyboard(&mut self, keys: &[Key]) {
        let mut buttons: u64 = 0;
        let mut lx: i32 = 0;
        let mut ly: i32 = 0;

        for key in keys {
            match key {
                Key::Left | Key::A => {
                    buttons |= NpadButton::DLeft as u64;
                    lx = -32767;
                }
                Key::Right | Key::D => {
                    buttons |= NpadButton::DRight as u64;
                    lx = 32767;
                }
                Key::Up | Key::W => {
                    buttons |= NpadButton::DUp as u64;
                    ly = 32767;
                }
                Key::Down | Key::S => {
                    buttons |= NpadButton::DDown as u64;
                    ly = -32767;
                }
                Key::Space | Key::J => buttons |= NpadButton::A as u64,
                Key::K => buttons |= NpadButton::B as u64,
                Key::U => buttons |= NpadButton::X as u64,
                Key::I => buttons |= NpadButton::Y as u64,
                Key::Q => buttons |= NpadButton::L as u64,
                Key::E => buttons |= NpadButton::R as u64,
                Key::Z => buttons |= NpadButton::ZL as u64,
                Key::C => buttons |= NpadButton::ZR as u64,
                Key::Enter => buttons |= NpadButton::Plus as u64,
                Key::Backspace => buttons |= NpadButton::Minus as u64,
                Key::Escape => {}
            }
        }

        self.state.buttons = buttons;
        self.state.analog_lx = lx;
        self.state.analog_ly = ly;
    }

    /// Writes controller state into Horizon Shared Memory for the guest game to read.
    pub fn update_shared_memory(&mut self, mmu: &mut Memory) -> Result<(), MemoryError> {
        self.sample_counter += 1;

        let base = self.shared_mem_addr;
        mmu.write_u64(base, self.sample_counter)?;
        mmu.write_u64(base + 8, self.state.buttons)?;
        mmu.write_u32(base + 16, self.state.analog_lx as u32)?;
        mmu.write_u32(base + 20, self.state.analog_ly as u32)?;
        mmu.write_u32(base + 24, self.state.analog_rx as u32)?;
        mmu.write_u32(base + 28, self.state.analog_ry as u32)?;

        Ok(())
    }
}
