//! last frame presented by the guest, handed to the gui through ffi.

use std::ffi::c_void;
use std::sync::Mutex;

pub type FrameCallback = Option<extern "C" fn(*mut c_void)>;

pub(crate) struct Display {
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub on_present: (FrameCallback, usize),
}

pub(crate) static DISPLAY: Mutex<Display> = Mutex::new(Display {
    generation: 0,
    width: 0,
    height: 0,
    pixels: Vec::new(),
    on_present: (None, 0),
});

/// stores a linear rgba8 frame with a stride of `width * 4`.
pub fn present(width: u32, height: u32, pixels: &[u8]) {
    assert_eq!(pixels.len(), width as usize * height as usize * 4);
    let on_present = {
        let mut display = DISPLAY.lock().unwrap();
        display.generation += 1;
        display.width = width;
        display.height = height;
        display.pixels.clear();
        display.pixels.extend_from_slice(pixels);
        display.on_present
    };
    // outside the lock, so a callback that reads the frame cannot deadlock
    if let (Some(callback), user) = on_present {
        callback(user as *mut c_void);
    }
}
