//! The presented frame on the device: one RGBA8 image, read back to the host only when asked.
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
            let dst = Layout::TRANSFER_DST_OPTIMAL;
            d.cmd_copy_buffer_to_image(cmd, buffer, self.image, dst, &self.region());
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
