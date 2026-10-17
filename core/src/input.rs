//! host input for the guest, buttons in npad bit order and sticks -1..1 with y up. this is the reader's shape only:
//! the keyboard and gamepad sources come with gpu/host-input, which replaces this file.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputState {
    pub buttons: u64,
    pub left: [f32; 2],
    pub right: [f32; 2],
    /// a gamepad is present or a key has been pressed.
    pub connected: bool,
}

pub fn state() -> InputState {
    InputState::default()
}
