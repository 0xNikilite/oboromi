//! The presented frame on the device: one RGBA8 image, read back to the host only when asked and
//! blitted letterboxed into the window swapchain when the gui attached a surface.
use super::compute::HostBuffer;
use super::{VkError, VkState};
use ash::vk;
use vk::{AccessFlags as Access, ImageLayout as Layout, PipelineStageFlags as Stage};

const COLOR: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};
const COLOR_LAYER: vk::ImageSubresourceLayers = vk::ImageSubresourceLayers {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    mip_level: 0,
    base_array_layer: 0,
    layer_count: 1,
};

fn transition(image: vk::Image, from: (Layout, Access), to: (Layout, Access)) -> vk::ImageMemoryBarrier<'static> {
    vk::ImageMemoryBarrier::default()
        .image(image)
        .subresource_range(COLOR)
        .old_layout(from.0)
        .src_access_mask(from.1)
        .new_layout(to.0)
        .dst_access_mask(to.1)
}

pub struct FrameImage {
    state: &'static VkState,
    pub staging: HostBuffer<'static>,
    pub image: vk::Image,
    memory: vk::DeviceMemory,
    pub width: u32,
    pub height: u32,
}

impl FrameImage {
    pub fn new(state: &'static VkState, width: u32, height: u32) -> Result<Self, VkError> {
        let d = &state.device;
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST);
        let mut frame = Self {
            state,
            staging: HostBuffer::new(state, u64::from(width * height * 4))?,
            image: unsafe { d.create_image(&info, None)? },
            memory: vk::DeviceMemory::null(),
            width,
            height,
        };
        let req = unsafe { d.get_image_memory_requirements(frame.image) };
        let props =
            unsafe { state.instance.get_physical_device_memory_properties(state.physical_device) };
        let types = &props.memory_types[..props.memory_type_count as usize];
        let memory_type = (0..)
            .zip(types)
            .find(|&(i, t)| {
                req.memory_type_bits >> i & 1 == 1
                    && t.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .map_or(req.memory_type_bits.trailing_zeros(), |(i, _)| i);
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(memory_type);
        unsafe {
            frame.memory = d.allocate_memory(&alloc, None)?;
            d.bind_image_memory(frame.image, frame.memory, 0)?;
        }
        Ok(frame)
    }

    fn region(&self) -> [vk::BufferImageCopy; 1] {
        [vk::BufferImageCopy::default()
            .image_subresource(COLOR_LAYER)
            .image_extent(vk::Extent3D { width: self.width, height: self.height, depth: 1 })]
    }

    /// Copies linear RGBA8 rows from `buffer` and leaves the image in TRANSFER_SRC_OPTIMAL.
    pub fn upload(&self, buffer: vk::Buffer) -> Result<(), VkError> {
        let region = self.region();
        self.fill(|d, cmd| unsafe { d.cmd_copy_buffer_to_image(cmd, buffer, self.image, Layout::TRANSFER_DST_OPTIMAL, &region) })
    }

    /// Copies `image` of the same size and format, in TRANSFER_SRC_OPTIMAL, and leaves the frame in TRANSFER_SRC_OPTIMAL.
    pub fn copy(&self, image: vk::Image) -> Result<(), VkError> {
        let extent = vk::Extent3D { width: self.width, height: self.height, depth: 1 };
        let region = [vk::ImageCopy::default().src_subresource(COLOR_LAYER).dst_subresource(COLOR_LAYER).extent(extent)];
        let (src, dst) = (Layout::TRANSFER_SRC_OPTIMAL, Layout::TRANSFER_DST_OPTIMAL);
        self.fill(|d, cmd| unsafe { d.cmd_copy_image(cmd, image, src, self.image, dst, &region) })
    }

    fn fill(&self, copy: impl FnOnce(&ash::Device, vk::CommandBuffer)) -> Result<(), VkError> {
        let d = &self.state.device;
        // A barrier's first scope reaches back into earlier submissions, so this makes the
        // producer's shader writes visible to the copy.
        let written = vk::MemoryBarrier::default()
            .src_access_mask(Access::SHADER_WRITE)
            .dst_access_mask(Access::TRANSFER_READ);
        let to_dst = transition(
            self.image,
            (Layout::UNDEFINED, Access::empty()),
            (Layout::TRANSFER_DST_OPTIMAL, Access::TRANSFER_WRITE),
        );
        let to_src = transition(
            self.image,
            (Layout::TRANSFER_DST_OPTIMAL, Access::TRANSFER_WRITE),
            (Layout::TRANSFER_SRC_OPTIMAL, Access::TRANSFER_READ),
        );
        let none = vk::DependencyFlags::empty();
        self.state.submit_and_wait(|cmd| unsafe {
            let src = Stage::COMPUTE_SHADER | Stage::TRANSFER;
            d.cmd_pipeline_barrier(cmd, src, Stage::TRANSFER, none, &[written], &[], &[to_dst]);
            copy(d, cmd);
            d.cmd_pipeline_barrier(cmd, Stage::TRANSFER, Stage::TRANSFER, none, &[], &[], &[to_src]);
        })
    }

    pub fn read(&self, out: &mut [u8]) -> Result<(), VkError> {
        let d = &self.state.device;
        let to_host = vk::MemoryBarrier::default()
            .src_access_mask(Access::TRANSFER_WRITE)
            .dst_access_mask(Access::HOST_READ);
        let none = vk::DependencyFlags::empty();
        self.state.submit_and_wait(|cmd| unsafe {
            let src = Layout::TRANSFER_SRC_OPTIMAL;
            d.cmd_copy_image_to_buffer(cmd, self.image, src, self.staging.buffer, &self.region());
            d.cmd_pipeline_barrier(cmd, Stage::TRANSFER, Stage::HOST, none, &[to_host], &[], &[]);
        })?;
        self.staging.read(out)
    }
}

impl Drop for FrameImage {
    fn drop(&mut self) {
        unsafe {
            self.state.device.destroy_image(self.image, None);
            self.state.device.free_memory(self.memory, None);
        }
    }
}

pub struct Swapchain {
    state: &'static VkState,
    loader: &'static ash::khr::swapchain::Device,
    handle: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    pub extent: vk::Extent2D,
}

impl Swapchain {
    /// `width` and `height` are the window's pixel size, used where the surface has no extent of its own.
    pub fn new(
        state: &'static VkState,
        surface: vk::SurfaceKHR,
        width: u32,
        height: u32,
        old: Option<&Swapchain>,
    ) -> Result<Self, VkError> {
        let unsupported = || VkError::Vk(vk::Result::ERROR_FEATURE_NOT_PRESENT);
        let (Some(surfaces), Some(loader)) = (&state.surface, &state.swapchain) else {
            return Err(unsupported());
        };
        let pd = state.physical_device;
        let family = state.queue_family;
        let caps = unsafe {
            if !surfaces.get_physical_device_surface_support(pd, family, surface)? {
                return Err(unsupported());
            }
            surfaces.get_physical_device_surface_capabilities(pd, surface)?
        };
        let format = unsafe { surfaces.get_physical_device_surface_formats(pd, surface)? }
            .into_iter()
            .find(|f| matches!(f.format, vk::Format::B8G8R8A8_UNORM | vk::Format::R8G8B8A8_UNORM))
            .ok_or_else(unsupported)?;
        if !caps.supported_usage_flags.contains(vk::ImageUsageFlags::TRANSFER_DST) {
            return Err(unsupported());
        }
        // u32::MAX means the swapchain decides the window size, as on Wayland.
        let extent = match caps.current_extent.width {
            u32::MAX => vk::Extent2D { width, height },
            _ => caps.current_extent,
        };
        let alpha = caps.supported_composite_alpha.as_raw();
        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(caps.min_image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::from_raw(alpha & alpha.wrapping_neg()))
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(old.map_or(vk::SwapchainKHR::null(), |old| old.handle));
        let mut swapchain = Self {
            state,
            loader,
            handle: unsafe { loader.create_swapchain(&info, None)? },
            images: Vec::new(),
            extent,
        };
        swapchain.images = unsafe { loader.get_swapchain_images(swapchain.handle)? };
        Ok(swapchain)
    }

    /// Shows `frame` letterboxed on black, or only black before the first frame.
    pub fn show(&self, frame: Option<&FrameImage>) -> Result<(), VkError> {
        let state = self.state;
        let (d, loader) = (&state.device, self.loader);
        let index = {
            let submit = state.submit.lock().unwrap_or_else(|e| e.into_inner());
            let fence = submit.fence;
            let null = vk::Semaphore::null();
            unsafe {
                d.reset_fences(&[fence])?;
                match loader.acquire_next_image(self.handle, u64::MAX, null, fence) {
                    Ok((index, _)) => d.wait_for_fences(&[fence], true, u64::MAX).map(|_| index)?,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(()),
                    Err(e) => return Err(e.into()),
                }
            }
        };
        let image = self.images[index as usize];
        let to_dst = transition(
            image,
            (Layout::UNDEFINED, Access::empty()),
            (Layout::TRANSFER_DST_OPTIMAL, Access::TRANSFER_WRITE),
        );
        let to_present = transition(
            image,
            (Layout::TRANSFER_DST_OPTIMAL, Access::TRANSFER_WRITE),
            (Layout::PRESENT_SRC_KHR, Access::empty()),
        );
        let cleared = vk::MemoryBarrier::default()
            .src_access_mask(Access::TRANSFER_WRITE)
            .dst_access_mask(Access::TRANSFER_WRITE);
        let black = vk::ClearColorValue::default();
        let none = vk::DependencyFlags::empty();
        let dst = Layout::TRANSFER_DST_OPTIMAL;
        state.submit_and_wait(|cmd| unsafe {
            let (top, transfer) = (Stage::TOP_OF_PIPE, Stage::TRANSFER);
            d.cmd_pipeline_barrier(cmd, top, transfer, none, &[], &[], &[to_dst]);
            d.cmd_clear_color_image(cmd, image, dst, &black, &[COLOR]);
            if let Some(frame) = frame {
                d.cmd_pipeline_barrier(cmd, transfer, transfer, none, &[cleared], &[], &[]);
                let blit = [letterbox(frame, self.extent)];
                let src = Layout::TRANSFER_SRC_OPTIMAL;
                let linear = vk::Filter::LINEAR;
                d.cmd_blit_image(cmd, frame.image, src, image, dst, &blit, linear);
            }
            let bottom = Stage::BOTTOM_OF_PIPE;
            d.cmd_pipeline_barrier(cmd, transfer, bottom, none, &[], &[], &[to_present]);
        })?;
        let (swapchains, indices) = ([self.handle], [index]);
        let present = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&indices);
        let _queue = state.submit.lock().unwrap_or_else(|e| e.into_inner());
        match unsafe { loader.queue_present(state.queue, &present) } {
            Ok(_) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

// Same rule as the widget: the largest centred rectangle with the frame's aspect ratio.
fn letterbox(frame: &FrameImage, window: vk::Extent2D) -> vk::ImageBlit {
    let (w, h) = (u64::from(frame.width), u64::from(frame.height));
    let (ww, wh) = (u64::from(window.width), u64::from(window.height));
    let (bw, bh) = if wh * w / h <= ww { (wh * w / h, wh) } else { (ww, ww * h / w) };
    let (x, y) = ((ww - bw) / 2, (wh - bh) / 2);
    let corner = |x: u64, y: u64, z| vk::Offset3D { x: x as i32, y: y as i32, z };
    vk::ImageBlit {
        src_subresource: COLOR_LAYER,
        src_offsets: [corner(0, 0, 0), corner(w, h, 1)],
        dst_subresource: COLOR_LAYER,
        dst_offsets: [corner(x, y, 0), corner(x + bw, y + bh, 1)],
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_swapchain(self.handle, None) };
    }
}
