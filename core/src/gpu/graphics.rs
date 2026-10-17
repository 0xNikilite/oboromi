//! Graphics pipelines drawing with dynamic rendering into an offscreen RGBA8 target that is read back after each draw.
use super::compute::{Descriptors, HostBuffer};
use super::pipeline_cache::hash;
use super::{VkError, VkState};
use ash::vk;

const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

pub struct VertexLayout<'a> {
    pub bindings: &'a [vk::VertexInputBindingDescription],
    pub attributes: &'a [vk::VertexInputAttributeDescription],
    /// Draws may switch topology only within this one's class: points, lines, triangles or patches.
    pub topology: vk::PrimitiveTopology,
}

pub struct TargetState {
    /// Baked only without VK_EXT_extended_dynamic_state3, which takes `RasterState::blend` instead.
    pub blend: vk::PipelineColorBlendAttachmentState,
    pub depth: Option<vk::Format>,
}

/// The 3D state that is Vulkan dynamic state, so one pipeline serves every value of it.
#[derive(Clone, Copy)]
pub struct RasterState {
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
    pub cull_face: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub topology: vk::PrimitiveTopology,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_func: vk::CompareOp,
    pub blend_const: [f32; 4],
    /// SET_BLEND, its equations and SET_CT_WRITE, dynamic with VK_EXT_extended_dynamic_state3.
    pub blend: vk::PipelineColorBlendAttachmentState,
}

pub struct DrawState<'a> {
    /// Each with its SET_VERTEX_STREAM_A_FORMAT stride.
    pub vertex_buffers: &'a [(&'a HostBuffer<'a>, u32)],
    /// With an index buffer, `vertex_count` and `first_vertex` count indices.
    pub index_buffer: Option<(&'a HostBuffer<'a>, vk::IndexType)>,
    pub vertex_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
    pub instance_count: u32,
    /// Without a clear the draw loads what the previous draw into the target left.
    pub clear: Option<[f32; 4]>,
    pub clear_depth: Option<f32>,
    /// The colour starts from what `RenderTarget::write` staged, the target's guest memory.
    pub load: bool,
    pub raster: RasterState,
}

impl DrawState<'_> {
    /// The rows of a target `height` rows high the draw can write: all of them after a clear, else the scissor's rows
    /// within the viewport.
    pub fn rows(&self, height: u32) -> std::ops::Range<u32> {
        let (viewport, scissor) = (self.raster.viewport, self.raster.scissor);
        if self.clear.is_some() {
            return 0..height;
        }
        let [top, bottom] = [viewport.y, viewport.y + viewport.height];
        let start = (scissor.offset.y as u32).max(top.min(bottom).floor() as u32);
        let end = (scissor.offset.y as u32 + scissor.extent.height).min(top.max(bottom).ceil() as u32).min(height);
        start.min(end)..end
    }
}

pub struct GraphicsPipeline<'a> {
    device: &'a ash::Device,
    /// The vertex stage's resources in set 0, the fragment stage's in set 1.
    pub descriptors: [Descriptors<'a>; 2],
    shaders: [vk::ShaderModule; 2],
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

impl<'a> GraphicsPipeline<'a> {
    pub fn new(
        state: &'a VkState,
        vertex_spirv: &[u32],
        fragment_spirv: &[u32],
        bindings: [&[(u32, vk::DescriptorType)]; 2],
        vertex_layout: &VertexLayout,
        target: &TargetState,
    ) -> Result<Self, VkError> {
        use vk::ShaderStageFlags as Stage;
        let d = &state.device;
        let mut p = Self {
            device: d,
            descriptors: [Descriptors::new(d, bindings[0], Stage::VERTEX)?, Descriptors::new(d, bindings[1], Stage::FRAGMENT)?],
            shaders: Default::default(),
            layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
        };
        for (shader, code) in p.shaders.iter_mut().zip([vertex_spirv, fragment_spirv]) {
            *shader = unsafe { d.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code), None)? };
        }
        let set_layouts = p.descriptors.each_ref().map(|descriptors| descriptors.set_layout);
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        p.layout = unsafe { d.create_pipeline_layout(&layout_info, None)? };
        let stages = [(Stage::VERTEX, p.shaders[0]), (Stage::FRAGMENT, p.shaders[1])]
            .map(|(stage, module)| vk::PipelineShaderStageCreateInfo::default().stage(stage).module(module).name(c"main"));
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(vertex_layout.bindings)
            .vertex_attribute_descriptions(vertex_layout.attributes);
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vertex_layout.topology);
        let viewport = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default().line_width(1.0);
        let multisample =
            vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default();
        let blend_attachments = [target.blend];
        let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
        use vk::DynamicState as Dynamic;
        let dynamic_states = [
            Dynamic::VIEWPORT,
            Dynamic::SCISSOR,
            Dynamic::CULL_MODE,
            Dynamic::FRONT_FACE,
            Dynamic::PRIMITIVE_TOPOLOGY,
            Dynamic::DEPTH_TEST_ENABLE,
            Dynamic::DEPTH_WRITE_ENABLE,
            Dynamic::DEPTH_COMPARE_OP,
            Dynamic::BLEND_CONSTANTS,
            Dynamic::VERTEX_INPUT_BINDING_STRIDE,
            Dynamic::COLOR_BLEND_ENABLE_EXT,
            Dynamic::COLOR_BLEND_EQUATION_EXT,
            Dynamic::COLOR_WRITE_MASK_EXT,
        ];
        let dynamic3 = state.extended_dynamic_state3.is_some();
        let dynamic_states = &dynamic_states[..dynamic_states.len() - if dynamic3 { 0 } else { 3 }];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);
        let color_formats = [COLOR_FORMAT];
        let mut rendering = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_formats)
            .depth_attachment_format(target.depth.unwrap_or_default());
        let info = vk::GraphicsPipelineCreateInfo::default()
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(p.layout)
            .push_next(&mut rendering);
        p.pipeline = if state.graphics_pipeline_library {
            let streams: Vec<_> = vertex_layout.bindings.iter().map(|b| (b.binding, b.input_rate)).collect();
            let attributes: Vec<_> = vertex_layout.attributes.iter().map(|a| (a.location, a.binding, a.format, a.offset)).collect();
            let b = target.blend;
            let blend = (!dynamic3).then_some((b.blend_enable, b.src_color_blend_factor, b.dst_color_blend_factor, b.color_blend_op));
            let alpha = (!dynamic3).then_some((b.src_alpha_blend_factor, b.dst_alpha_blend_factor, b.alpha_blend_op, b.color_write_mask));
            let keys = [
                hash((streams, attributes, vertex_layout.topology)),
                hash((vertex_spirv, bindings)),
                hash((fragment_spirv, bindings, target.depth)),
                hash((target.depth, blend, alpha)),
            ];
            state.link(info, &stages, keys)?
        } else {
            state.create_graphics_pipeline(info.stages(&stages))?
        };
        Ok(p)
    }
}

impl Drop for GraphicsPipeline<'_> {
    fn drop(&mut self) {
        let d = self.device;
        unsafe {
            d.destroy_pipeline(self.pipeline, None);
            d.destroy_pipeline_layout(self.layout, None);
            self.shaders.iter().for_each(|&shader| d.destroy_shader_module(shader, None));
        }
    }
}

struct Attachment<'a> {
    device: &'a ash::Device,
    aspect: vk::ImageAspectFlags,
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
}

impl<'a> Attachment<'a> {
    fn new(
        device: &'a ash::Device,
        extent: vk::Extent2D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        aspect: vk::ImageAspectFlags,
    ) -> Result<Self, VkError> {
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(usage);
        let mut a = Self {
            device,
            aspect,
            image: unsafe { device.create_image(&info, None)? },
            memory: vk::DeviceMemory::null(),
            view: vk::ImageView::null(),
        };
        let range = vk::ImageSubresourceRange::default().aspect_mask(aspect).level_count(1).layer_count(1);
        unsafe {
            let req = device.get_image_memory_requirements(a.image);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(req.size)
                .memory_type_index(req.memory_type_bits.trailing_zeros());
            a.memory = device.allocate_memory(&alloc, None)?;
            device.bind_image_memory(a.image, a.memory, 0)?;
            let view = vk::ImageViewCreateInfo::default()
                .image(a.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(range);
            a.view = device.create_image_view(&view, None)?;
        }
        Ok(a)
    }
}

impl Drop for Attachment<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct RenderTarget<'a> {
    pub extent: vk::Extent2D,
    color: Attachment<'a>,
    depth: Option<Attachment<'a>>,
    staging: HostBuffer<'a>,
}

impl<'a> RenderTarget<'a> {
    pub fn new(state: &'a VkState, width: u32, height: u32, depth: Option<vk::Format>) -> Result<Self, VkError> {
        use vk::{ImageAspectFlags as Aspect, ImageUsageFlags as Usage};
        let (d, extent) = (&state.device, vk::Extent2D { width, height });
        Ok(Self {
            extent,
            color: Attachment::new(d, extent, COLOR_FORMAT, Usage::COLOR_ATTACHMENT | Usage::TRANSFER_SRC | Usage::TRANSFER_DST, Aspect::COLOR)?,
            depth: depth
                .map(|format| {
                    let stencil = if format == vk::Format::D24_UNORM_S8_UINT { Aspect::STENCIL } else { Aspect::empty() };
                    Attachment::new(d, extent, format, Usage::DEPTH_STENCIL_ATTACHMENT, Aspect::DEPTH | stencil)
                })
                .transpose()?,
            staging: HostBuffer::new(state, u64::from(width * height * 4))?,
        })
    }

    pub fn read(&self, out: &mut [u8]) -> Result<(), VkError> {
        self.staging.read(out)
    }

    /// The target as its last draw left it, row by row.
    pub fn staged(&self) -> Result<&[u8], VkError> {
        self.staging.view()
    }

    pub fn write(&mut self, pixels: &[u8]) -> Result<(), VkError> {
        self.staging.write(pixels)
    }
}

impl VkState {
    pub fn draw(&self, pipeline: &GraphicsPipeline, target: &RenderTarget, draw: &DrawState) -> Result<(), VkError> {
        use vk::{AccessFlags as Access, ImageLayout as Layout, PipelineStageFlags as Stage};
        let d = &self.device;
        let graphics = vk::PipelineBindPoint::GRAPHICS;
        let load = |clear: bool| if clear { vk::AttachmentLoadOp::CLEAR } else { vk::AttachmentLoadOp::LOAD };
        let barrier = |attachment: &Attachment| {
            vk::ImageMemoryBarrier::default()
                .image(attachment.image)
                .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(attachment.aspect).level_count(1).layer_count(1))
        };
        // A cleared attachment starts from UNDEFINED, a loaded one from where the previous draw left it.
        let discard_or = |clear: bool, layout| if clear { Layout::UNDEFINED } else { layout };
        let to_upload = barrier(&target.color).new_layout(Layout::TRANSFER_DST_OPTIMAL).dst_access_mask(Access::TRANSFER_WRITE);
        let to_color = barrier(&target.color)
            .old_layout(if draw.load { Layout::TRANSFER_DST_OPTIMAL } else { discard_or(draw.clear.is_some(), Layout::TRANSFER_SRC_OPTIMAL) })
            .new_layout(Layout::COLOR_ATTACHMENT_OPTIMAL)
            .src_access_mask(if draw.load { Access::TRANSFER_WRITE } else { Access::empty() })
            .dst_access_mask(Access::COLOR_ATTACHMENT_READ | Access::COLOR_ATTACHMENT_WRITE);
        let color = [vk::RenderingAttachmentInfo::default()
            .image_view(target.color.view)
            .image_layout(Layout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(load(draw.clear.is_some()))
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: draw.clear.unwrap_or_default() } })];
        let mut rendering = vk::RenderingInfo::default()
            .render_area(target.extent.into())
            .layer_count(1)
            .color_attachments(&color);
        let mut to_attachments = [to_color, to_color];
        let depth_info;
        if let Some(depth) = &target.depth {
            let clear = draw.clear_depth.is_some();
            to_attachments[1] = barrier(depth)
                .old_layout(discard_or(clear, Layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL))
                .new_layout(Layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .src_access_mask(Access::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .dst_access_mask(Access::DEPTH_STENCIL_ATTACHMENT_READ | Access::DEPTH_STENCIL_ATTACHMENT_WRITE);
            depth_info = vk::RenderingAttachmentInfo::default()
                .image_view(depth.view)
                .image_layout(Layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(load(clear))
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: draw.clear_depth.unwrap_or_default(), stencil: 0 } });
            rendering = rendering.depth_attachment(&depth_info);
        }
        let to_attachments = &to_attachments[..1 + usize::from(target.depth.is_some())];
        let to_transfer = barrier(&target.color)
            .old_layout(Layout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(Layout::TRANSFER_SRC_OPTIMAL)
            .src_access_mask(Access::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(Access::TRANSFER_READ);
        let copy = [vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers::default().aspect_mask(vk::ImageAspectFlags::COLOR).layer_count(1))
            .image_extent(target.extent.into())];
        let rows = draw.rows(target.extent.height);
        let readback = [copy[0]
            .buffer_offset(u64::from(rows.start * target.extent.width * 4))
            .image_offset(vk::Offset3D { y: rows.start as i32, ..Default::default() })
            .image_extent(vk::Extent3D { height: rows.len() as u32, ..target.extent.into() })];
        let to_host = vk::MemoryBarrier::default().src_access_mask(Access::TRANSFER_WRITE).dst_access_mask(Access::HOST_READ);
        let fragment_tests = Stage::EARLY_FRAGMENT_TESTS | Stage::LATE_FRAGMENT_TESTS;
        let none = vk::DependencyFlags::empty();
        let r = &draw.raster;
        self.submit_and_wait(|cmd| unsafe {
            if draw.load {
                d.cmd_pipeline_barrier(cmd, Stage::TOP_OF_PIPE, Stage::TRANSFER, none, &[], &[], &[to_upload]);
                d.cmd_copy_buffer_to_image(cmd, target.staging.buffer, target.color.image, Layout::TRANSFER_DST_OPTIMAL, &copy);
            }
            d.cmd_pipeline_barrier(
                cmd,
                Stage::TRANSFER | fragment_tests,
                Stage::COLOR_ATTACHMENT_OUTPUT | fragment_tests,
                none,
                &[],
                &[],
                to_attachments,
            );
            d.cmd_begin_rendering(cmd, &rendering);
            d.cmd_bind_pipeline(cmd, graphics, pipeline.pipeline);
            d.cmd_bind_descriptor_sets(cmd, graphics, pipeline.layout, 0, &pipeline.descriptors.each_ref().map(|descriptors| descriptors.set), &[]);
            d.cmd_set_viewport(cmd, 0, &[r.viewport]);
            d.cmd_set_scissor(cmd, 0, &[r.scissor]);
            d.cmd_set_cull_mode(cmd, r.cull_face);
            d.cmd_set_front_face(cmd, r.front_face);
            d.cmd_set_primitive_topology(cmd, r.topology);
            d.cmd_set_depth_test_enable(cmd, r.depth_test);
            d.cmd_set_depth_write_enable(cmd, r.depth_write);
            d.cmd_set_depth_compare_op(cmd, r.depth_func);
            d.cmd_set_blend_constants(cmd, &r.blend_const);
            if let Some(dynamic3) = &self.extended_dynamic_state3 {
                let b = r.blend;
                let equation = vk::ColorBlendEquationEXT::default()
                    .src_color_blend_factor(b.src_color_blend_factor)
                    .dst_color_blend_factor(b.dst_color_blend_factor)
                    .color_blend_op(b.color_blend_op)
                    .src_alpha_blend_factor(b.src_alpha_blend_factor)
                    .dst_alpha_blend_factor(b.dst_alpha_blend_factor)
                    .alpha_blend_op(b.alpha_blend_op);
                dynamic3.cmd_set_color_blend_enable(cmd, 0, &[b.blend_enable]);
                dynamic3.cmd_set_color_blend_equation(cmd, 0, &[equation]);
                dynamic3.cmd_set_color_write_mask(cmd, 0, &[b.color_write_mask]);
            }
            for (binding, (buffer, stride)) in (0..).zip(draw.vertex_buffers) {
                d.cmd_bind_vertex_buffers2(cmd, binding, &[buffer.buffer], &[0], None, Some(&[(*stride).into()]));
            }
            match draw.index_buffer {
                Some((buffer, ty)) => {
                    d.cmd_bind_index_buffer(cmd, buffer.buffer, 0, ty);
                    d.cmd_draw_indexed(cmd, draw.vertex_count, draw.instance_count, draw.first_vertex, 0, draw.first_instance);
                }
                None => d.cmd_draw(cmd, draw.vertex_count, draw.instance_count, draw.first_vertex, draw.first_instance),
            }
            d.cmd_end_rendering(cmd);
            d.cmd_pipeline_barrier(cmd, Stage::COLOR_ATTACHMENT_OUTPUT, Stage::TRANSFER, none, &[], &[], &[to_transfer]);
            if !rows.is_empty() {
                d.cmd_copy_image_to_buffer(cmd, target.color.image, Layout::TRANSFER_SRC_OPTIMAL, target.staging.buffer, &readback);
            }
            d.cmd_pipeline_barrier(cmd, Stage::TRANSFER, Stage::HOST, none, &[to_host], &[], &[]);
        })
    }
}
