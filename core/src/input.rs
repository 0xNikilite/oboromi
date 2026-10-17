//! host input for the guest: keys forwarded by the gui and the last active gamepad, merged into
//! one switch npad state. buttons use the npad bit order, sticks are -1..1 with y up.

use gilrs::{Axis, Button, Event, EventType, Gilrs};
use std::sync::{Mutex, Once};

pub const A: u64 = 1 << 0;
pub const B: u64 = 1 << 1;
pub const X: u64 = 1 << 2;
pub const Y: u64 = 1 << 3;
pub const STICK_L: u64 = 1 << 4;
pub const STICK_R: u64 = 1 << 5;
pub const L: u64 = 1 << 6;
pub const R: u64 = 1 << 7;
pub const ZL: u64 = 1 << 8;
pub const ZR: u64 = 1 << 9;
pub const PLUS: u64 = 1 << 10;
pub const MINUS: u64 = 1 << 11;
pub const LEFT: u64 = 1 << 12;
pub const UP: u64 = 1 << 13;
pub const RIGHT: u64 = 1 << 14;
pub const DOWN: u64 = 1 << 15;
pub const STICK_L_LEFT: u64 = 1 << 16;
pub const STICK_L_UP: u64 = 1 << 17;
pub const STICK_L_RIGHT: u64 = 1 << 18;
pub const STICK_L_DOWN: u64 = 1 << 19;
pub const STICK_R_LEFT: u64 = 1 << 20;
pub const STICK_R_UP: u64 = 1 << 21;
pub const STICK_R_RIGHT: u64 = 1 << 22;
pub const STICK_R_DOWN: u64 = 1 << 23;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputState {
    pub buttons: u64,
    pub left: [f32; 2],
    pub right: [f32; 2],
}

struct Host {
    keys: u64,
    pad: InputState,
    pad_name: String,
}

static HOST: Mutex<Host> = Mutex::new(Host {
    keys: 0,
    pad: InputState { buttons: 0, left: [0.0; 2], right: [0.0; 2] },
    pad_name: String::new(),
});

/// qt key codes, positional like a keyboard laid over the joycons.
const KEYS: [(i32, u64); 22] = [
    (b'W' as i32, STICK_L_UP), (b'A' as i32, STICK_L_LEFT), (b'S' as i32, STICK_L_DOWN), (b'D' as i32, STICK_L_RIGHT),
    (0x0100_0013, STICK_L_UP), (0x0100_0012, STICK_L_LEFT), (0x0100_0015, STICK_L_DOWN), (0x0100_0014, STICK_L_RIGHT),
    (b'I' as i32, STICK_R_UP), (b'J' as i32, STICK_R_LEFT), (b'K' as i32, STICK_R_DOWN), (b'L' as i32, STICK_R_RIGHT),
    (b'Z' as i32, A), (b'X' as i32, B), (b'C' as i32, X), (b'V' as i32, Y),
    (b'Q' as i32, L), (b'E' as i32, R), (b'1' as i32, ZL), (b'3' as i32, ZR),
    (0x0100_0004, PLUS), (0x0100_0003, MINUS),
];

const PAD: [(Button, u64); 16] = [
    (Button::East, A), (Button::South, B), (Button::North, X), (Button::West, Y),
    (Button::LeftThumb, STICK_L), (Button::RightThumb, STICK_R),
    (Button::LeftTrigger, L), (Button::RightTrigger, R), (Button::LeftTrigger2, ZL), (Button::RightTrigger2, ZR),
    (Button::Start, PLUS), (Button::Select, MINUS),
    (Button::DPadLeft, LEFT), (Button::DPadUp, UP), (Button::DPadRight, RIGHT), (Button::DPadDown, DOWN),
];

/// a qt key press or release from the gui.
pub fn key(code: i32, down: bool) {
    let Some(&(_, bit)) = KEYS.iter().find(|(k, _)| *k == code) else { return };
    let mut host = HOST.lock().unwrap();
    host.keys = if down { host.keys | bit } else { host.keys & !bit };
}

/// the merged keyboard and gamepad state. starts the gamepad thread on first use.
pub fn state() -> InputState {
    snapshot().0
}

/// [`state`] and the name of the gamepad feeding it, empty without one.
pub fn snapshot() -> (InputState, String) {
    static START: Once = Once::new();
    START.call_once(|| drop(std::thread::Builder::new().name("gamepad".into()).spawn(gamepad_thread)));
    let host = HOST.lock().unwrap();
    (merge(host.keys, host.pad), host.pad_name.clone())
}

fn merge(keys: u64, pad: InputState) -> InputState {
    let axis = |neg, pos| (keys & pos != 0) as i8 as f32 - (keys & neg != 0) as i8 as f32;
    let stick = |kb: [f32; 2], pad| if kb == [0.0; 2] { pad } else { kb };
    InputState {
        buttons: keys | pad.buttons,
        left: stick([axis(STICK_L_LEFT, STICK_L_RIGHT), axis(STICK_L_DOWN, STICK_L_UP)], pad.left),
        right: stick([axis(STICK_R_LEFT, STICK_R_RIGHT), axis(STICK_R_DOWN, STICK_R_UP)], pad.right),
    }
}

fn gamepad_thread() {
    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(e) => return log::warn!("no gamepad support: {e}"),
    };
    while let Some(Event { id, event, .. }) = gilrs.next_event_blocking(None) {
        let pad = gilrs.gamepad(id);
        let mut host = HOST.lock().unwrap();
        if event == EventType::Disconnected {
            host.pad = InputState::default();
            host.pad_name.clear();
            continue;
        }
        host.pad = InputState {
            buttons: PAD.iter().filter(|(b, _)| pad.is_pressed(*b)).fold(0, |acc, (_, bit)| acc | bit),
            left: [pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY)],
            right: [pad.value(Axis::RightStickX), pad.value(Axis::RightStickY)],
        };
        host.pad_name = pad.name().to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_stick_wins_over_a_centered_gamepad_and_buttons_or() {
        let pad = InputState { buttons: B, left: [0.0; 2], right: [0.5, 0.0] };
        let merged = merge(A | STICK_L_RIGHT, pad);
        assert_eq!(merged, InputState { buttons: A | B | STICK_L_RIGHT, left: [1.0, 0.0], right: [0.5, 0.0] });
    }
}
