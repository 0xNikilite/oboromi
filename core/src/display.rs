//! last frame presented by the guest, handed to the gui through ffi.

use crate::gpu::compute::HostBuffer;
use crate::gpu::present::FrameImage;
use crate::gpu::{VkError, device};
use std::ffi::c_void;
use std::sync::Mutex;

pub type FrameCallback = Option<extern "C" fn(*mut c_void)>;

pub(crate) struct Display {
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
    pixels_stale: bool,
    image: Option<FrameImage>,
}

pub(crate) static DISPLAY: Mutex<Display> = Mutex::new(Display {
    generation: 0,
    width: 0,
    height: 0,
    pixels: Vec::new(),
    pixels_stale: false,
    image: None,
});

pub(crate) static CALLBACK: Mutex<(FrameCallback, usize)> = Mutex::new((None, 0));

impl Display {
    /// the frame as linear rgba8, read back from the device if it was presented there.
    pub fn pixels(&mut self) -> &[u8] {
        if let (true, Some(image)) = (self.pixels_stale, &self.image) {
            self.pixels.resize(self.width as usize * self.height as usize * 4, 0);
            if let Err(e) = image.read(&mut self.pixels) {
                log::error!("frame readback failed: {e:?}");
            }
            self.pixels_stale = false;
        }
        &self.pixels
    }

    fn image(&mut self, width: u32, height: u32) -> Result<&FrameImage, VkError> {
        let image = match self.image.take() {
            Some(image) if (image.width, image.height) == (width, height) => image,
            old => {
                drop(old);
                FrameImage::new(device()?, width, height)?
            }
        };
        Ok(self.image.insert(image))
    }

    fn presented(&mut self, width: u32, height: u32, pixels_stale: bool) {
        self.generation += 1;
        self.width = width;
        self.height = height;
        self.pixels_stale = pixels_stale;
    }
}

fn notify() {
    // held during the call so set_callback waits for it, so the callback must not call set_callback
    let callback = CALLBACK.lock().unwrap();
    if let (Some(callback), user) = *callback {
        callback(user as *mut c_void);
    }
}

/// stores a linear rgba8 frame with a stride of `width * 4`.
pub fn present(width: u32, height: u32, pixels: &[u8]) {
    assert_eq!(pixels.len(), width as usize * height as usize * 4);
    {
        let mut display = DISPLAY.lock().unwrap();
        display.presented(width, height, false);
        display.pixels.clear();
        display.pixels.extend_from_slice(pixels);
    }
    notify();
}

/// presents the linear rgba8 frame in `buffer` without reading it back to the host.
pub fn present_buffer(buffer: &HostBuffer, width: u32, height: u32) -> Result<(), VkError> {
    {
        let mut display = DISPLAY.lock().unwrap();
        display.image(width, height)?.upload(buffer.buffer)?;
        display.presented(width, height, true);
    }
    notify();
    Ok(())
}
