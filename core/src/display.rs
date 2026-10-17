//! last frame presented by the guest, handed to the gui through ffi.

use crate::gpu::compute::HostBuffer;
use crate::gpu::present::{FrameImage, Swapchain};
use crate::gpu::{VkError, device};
use ash::vk;
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
    swapchain: Option<Swapchain>,
}

pub(crate) static DISPLAY: Mutex<Display> = Mutex::new(Display {
    generation: 0,
    width: 0,
    height: 0,
    pixels: Vec::new(),
    pixels_stale: false,
    image: None,
    swapchain: None,
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

    fn image(&mut self, width: u32, height: u32) -> Result<&mut FrameImage, VkError> {
        let image = match self.image.take() {
            Some(image) if (image.width, image.height) == (width, height) => image,
            old => {
                drop(old);
                FrameImage::new(device()?, width, height)?
            }
        };
        Ok(self.image.insert(image))
    }

    fn show(&self) -> Result<(), VkError> {
        self.swapchain.as_ref().map_or(Ok(()), |s| s.show(self.image.as_ref()))
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
        if display.swapchain.is_some() {
            let shown = display.image(width, height).and_then(|image| {
                image.staging.write(pixels)?;
                image.upload(image.staging.buffer)
            });
            if let Err(e) = shown.and_then(|_| display.show()) {
                log::error!("frame present failed: {e:?}");
            }
        }
    }
    notify();
}

/// presents the linear rgba8 frame in `buffer` without reading it back to the host.
pub fn present_buffer(buffer: &HostBuffer, width: u32, height: u32) -> Result<(), VkError> {
    {
        let mut display = DISPLAY.lock().unwrap();
        display.image(width, height)?.upload(buffer.buffer)?;
        display.presented(width, height, true);
        display.show()?;
    }
    notify();
    Ok(())
}

/// presents into the window `surface` of `width` x `height` pixels from now on, and shows the last
/// frame in it. a null surface detaches the window, which the gui must do before destroying it.
pub fn attach_surface(surface: vk::SurfaceKHR, width: u32, height: u32) -> Result<(), VkError> {
    let mut display = DISPLAY.lock().unwrap();
    let old = display.swapchain.take();
    if surface == vk::SurfaceKHR::null() {
        return Ok(());
    }
    display.swapchain = Some(match old {
        Some(s) if s.extent == (vk::Extent2D { width, height }) => s,
        old => Swapchain::new(device()?, surface, width, height, old.as_ref())?,
    });
    display.show()
}
