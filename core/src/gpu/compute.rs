//! Host-visible buffers and compute pipelines dispatched synchronously on the `VkState` queue.
use super::texture::TextureHeader;
use super::{VkError, VkState};
use ash::vk;

#[derive(Default)]
pub struct Submit {
    pub command_buffer: vk::CommandBuffer,
    pub fence: vk::Fence,
}

pub struct HostBuffer<'a> {
    device: &'a ash::Device,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
    ptr: *mut u8,
}

impl<'a> HostBuffer<'a> {
    pub fn new(state: &'a VkState, size: u64) -> Result<Self, VkError> {
        use vk::BufferUsageFlags as Usage;
        Self::with_usage(state, size, Usage::STORAGE_BUFFER | Usage::UNIFORM_BUFFER | Usage::VERTEX_BUFFER | Usage::INDEX_BUFFER | Usage::TRANSFER_SRC | Usage::TRANSFER_DST)
    }

    pub fn with_usage(state: &'a VkState, size: u64, usage: vk::BufferUsageFlags) -> Result<Self, VkError> {
        let d = &state.device;
        let info = vk::BufferCreateInfo::default().size(size).usage(usage);
        let mut buf = Self {
            device: d,
            buffer: unsafe { d.create_buffer(&info, None)? },
            memory: vk::DeviceMemory::null(),
            size,
            ptr: std::ptr::null_mut(),
        };
        let req = unsafe { d.get_buffer_memory_requirements(buf.buffer) };
        let instance = &state.instance;
        let props =
            unsafe { instance.get_physical_device_memory_properties(state.physical_device) };
        // Host reads from uncached memory are slow, so cached wins even when it is not coherent.
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE;
        let types = &props.memory_types[..props.memory_type_count as usize];
        let memory_type = [
            vk::MemoryPropertyFlags::HOST_CACHED,
            vk::MemoryPropertyFlags::HOST_COHERENT,
        ]
        .into_iter()
        .find_map(|flags| {
            (0..).zip(types).find(|&(i, t)| {
                req.memory_type_bits >> i & 1 == 1 && t.property_flags.contains(host | flags)
            })
        })
        .ok_or(VkError::Vk(vk::Result::ERROR_FEATURE_NOT_PRESENT))?
        .0;
        let mut addressable = vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(memory_type);
        if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
            alloc = alloc.push_next(&mut addressable);
        }
        unsafe {
            buf.memory = d.allocate_memory(&alloc, None)?;
            d.bind_buffer_memory(buf.buffer, buf.memory, 0)?;
            buf.ptr = d
                .map_memory(buf.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())?
                .cast();
        }
        Ok(buf)
    }

    fn address(&self) -> u64 {
        unsafe { self.device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.buffer)) }
    }

    fn mapped(&self) -> [vk::MappedMemoryRange<'_>; 1] {
        [vk::MappedMemoryRange::default()
            .memory(self.memory)
            .size(vk::WHOLE_SIZE)]
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), VkError> {
        self.write_at(0, data);
        self.flush()
    }

    /// A write without the flush, for several before one.
    pub fn write_at(&mut self, offset: u64, data: &[u8]) {
        assert!(offset + data.len() as u64 <= self.size);
        unsafe { self.ptr.add(offset as usize).copy_from_nonoverlapping(data.as_ptr(), data.len()) }
    }

    pub fn flush(&self) -> Result<(), VkError> {
        unsafe { Ok(self.device.flush_mapped_memory_ranges(&self.mapped())?) }
    }

    pub fn read(&self, out: &mut [u8]) -> Result<(), VkError> {
        assert!(out.len() as u64 <= self.size);
        unsafe {
            self.device
                .invalidate_mapped_memory_ranges(&self.mapped())?;
            self.ptr.copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }
        Ok(())
    }
}

// The pointer is a host mapping of device memory, valid on any thread until Drop unmaps it.
unsafe impl Send for HostBuffer<'_> {}

impl Drop for HostBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

/// One instance of a top-level acceleration structure: indexed triangles and the row-major 3x4 object-to-world transform.
pub struct Triangles<'d> {
    pub positions: &'d [[f32; 3]],
    pub indices: &'d [u32],
    pub transform: [[f32; 4]; 3],
    pub opaque: bool,
}

/// A top-level acceleration structure over one bottom-level structure per instance, built on the device.
pub struct AccelerationStructure<'a> {
    loader: &'a ash::khr::acceleration_structure::Device,
    structures: Vec<vk::AccelerationStructureKHR>,
    buffers: Vec<HostBuffer<'a>>,
    pub handle: vk::AccelerationStructureKHR,
}

impl<'a> AccelerationStructure<'a> {
    pub fn new(state: &'a VkState, instances: &[Triangles]) -> Result<Self, VkError> {
        use vk::BufferUsageFlags as Usage;
        let loader = state.acceleration_structure.as_ref().expect("ray queries need VK_KHR_acceleration_structure and VK_KHR_ray_query");
        let mut s = Self { loader, structures: Vec::new(), buffers: Vec::new(), handle: vk::AccelerationStructureKHR::null() };
        let input = |bytes: &[u8]| {
            let mut buffer = HostBuffer::with_usage(state, bytes.len().max(4) as u64, Usage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR | Usage::SHADER_DEVICE_ADDRESS)?;
            buffer.write(bytes)?;
            Ok::<_, VkError>(buffer)
        };
        let mut records = Vec::new();
        for instance in instances {
            let positions = input(&instance.positions.as_flattened().iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>())?;
            let indices = input(&instance.indices.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>())?;
            let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
                .vertex_format(vk::Format::R32G32B32_SFLOAT)
                .vertex_data(vk::DeviceOrHostAddressConstKHR { device_address: positions.address() })
                .vertex_stride(12)
                .max_vertex(instance.positions.len().saturating_sub(1) as u32)
                .index_type(vk::IndexType::UINT32)
                .index_data(vk::DeviceOrHostAddressConstKHR { device_address: indices.address() });
            let flags = if instance.opaque { vk::GeometryFlagsKHR::OPAQUE } else { vk::GeometryFlagsKHR::empty() };
            let geometry = vk::AccelerationStructureGeometryKHR::default().geometry_type(vk::GeometryTypeKHR::TRIANGLES).geometry(vk::AccelerationStructureGeometryDataKHR { triangles }).flags(flags);
            let blas = s.build(state, vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL, geometry, instance.indices.len() as u32 / 3)?;
            s.buffers.extend([positions, indices]);
            let address = unsafe { loader.get_acceleration_structure_device_address(&vk::AccelerationStructureDeviceAddressInfoKHR::default().acceleration_structure(blas)) };
            records.push(vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR { matrix: instance.transform.as_flattened().try_into().unwrap() },
                instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(0, vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR { device_handle: address },
            });
        }
        let records = input(unsafe { std::slice::from_raw_parts(records.as_ptr().cast(), std::mem::size_of_val(records.as_slice())) })?;
        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::default().data(vk::DeviceOrHostAddressConstKHR { device_address: records.address() });
        let geometry = vk::AccelerationStructureGeometryKHR::default().geometry_type(vk::GeometryTypeKHR::INSTANCES).geometry(vk::AccelerationStructureGeometryDataKHR { instances: instances_data });
        s.handle = s.build(state, vk::AccelerationStructureTypeKHR::TOP_LEVEL, geometry, instances.len() as u32)?;
        s.buffers.push(records);
        Ok(s)
    }

    fn build(&mut self, state: &'a VkState, ty: vk::AccelerationStructureTypeKHR, geometry: vk::AccelerationStructureGeometryKHR, primitives: u32) -> Result<vk::AccelerationStructureKHR, VkError> {
        use vk::BufferUsageFlags as Usage;
        // minAccelerationStructureScratchOffsetAlignment is at most 256.
        const SCRATCH_ALIGNMENT: u64 = 256;
        let geometries = [geometry];
        let info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(ty)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometries);
        let mut sizes = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe { self.loader.get_acceleration_structure_build_sizes(vk::AccelerationStructureBuildTypeKHR::DEVICE, &info, &[primitives], &mut sizes) };
        let storage = HostBuffer::with_usage(state, sizes.acceleration_structure_size, Usage::ACCELERATION_STRUCTURE_STORAGE_KHR | Usage::SHADER_DEVICE_ADDRESS)?;
        let create = vk::AccelerationStructureCreateInfoKHR::default().buffer(storage.buffer).size(sizes.acceleration_structure_size).ty(ty);
        let structure = unsafe { self.loader.create_acceleration_structure(&create, None)? };
        self.structures.push(structure);
        self.buffers.push(storage);
        let scratch = HostBuffer::with_usage(state, sizes.build_scratch_size + SCRATCH_ALIGNMENT, Usage::STORAGE_BUFFER | Usage::SHADER_DEVICE_ADDRESS)?;
        let info = info.dst_acceleration_structure(structure).scratch_data(vk::DeviceOrHostAddressKHR { device_address: scratch.address().next_multiple_of(SCRATCH_ALIGNMENT) });
        let range = [vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(primitives)];
        state.submit_and_wait(|cmd| unsafe { self.loader.cmd_build_acceleration_structures(cmd, &[info], &[&range]) })?;
        Ok(structure)
    }
}

impl Drop for AccelerationStructure<'_> {
    fn drop(&mut self) {
        for &structure in &self.structures {
            unsafe { self.loader.destroy_acceleration_structure(structure, None) };
        }
    }
}

pub struct HostImage<'a> {
    device: &'a ash::Device,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl<'a> HostImage<'a> {
    /// A 2D image with every level of `header` uploaded from the guest texels at the start of `memory`, left in GENERAL
    /// so the one view serves both as a sampled texture and, where the format allows it, as a storage image.
    pub fn new(
        state: &'a VkState,
        header: &TextureHeader,
        memory: &[u8],
        sampler: &vk::SamplerCreateInfo,
    ) -> Result<Self, VkError> {
        let d = &state.device;
        let extent = vk::Extent3D {
            width: header.width,
            height: header.height,
            depth: 1,
        };
        let features = unsafe { state.instance.get_physical_device_format_properties(state.physical_device, header.format) };
        let storage = if features.optimal_tiling_features.contains(vk::FormatFeatureFlags::STORAGE_IMAGE) {
            vk::ImageUsageFlags::STORAGE
        } else {
            vk::ImageUsageFlags::empty()
        };
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(header.format)
            .extent(extent)
            .mip_levels(header.levels)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(vk::ImageUsageFlags::SAMPLED | storage | vk::ImageUsageFlags::TRANSFER_DST);
        let mut img = Self {
            device: d,
            image: unsafe { d.create_image(&info, None)? },
            memory: vk::DeviceMemory::null(),
            view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
        };
        let req = unsafe { d.get_image_memory_requirements(img.image) };
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(req.memory_type_bits.trailing_zeros());
        let range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(header.levels)
            .layer_count(1);
        let (texels, copies) = header.upload(memory);
        let mut staging = HostBuffer::new(state, texels.len() as u64)?;
        staging.write(&texels)?;
        let to_transfer = vk::ImageMemoryBarrier::default()
            .image(img.image)
            .subresource_range(range)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
        let to_shader = to_transfer
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE);
        use vk::PipelineStageFlags as Stage;
        let none = vk::DependencyFlags::empty();
        unsafe {
            img.memory = d.allocate_memory(&alloc, None)?;
            d.bind_image_memory(img.image, img.memory, 0)?;
            state.submit_and_wait(|cmd| {
                d.cmd_pipeline_barrier(
                    cmd,
                    Stage::TOP_OF_PIPE,
                    Stage::TRANSFER,
                    none,
                    &[],
                    &[],
                    &[to_transfer],
                );
                d.cmd_copy_buffer_to_image(
                    cmd,
                    staging.buffer,
                    img.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &copies,
                );
                d.cmd_pipeline_barrier(
                    cmd,
                    Stage::TRANSFER,
                    Stage::COMPUTE_SHADER,
                    none,
                    &[],
                    &[],
                    &[to_shader],
                );
            })?;
            let view = vk::ImageViewCreateInfo::default()
                .image(img.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(header.format)
                .subresource_range(range);
            img.view = d.create_image_view(&view, None)?;
            img.sampler = d.create_sampler(&sampler.unnormalized_coordinates(!header.normalized), None)?;
        }
        Ok(img)
    }
}

impl Drop for HostImage<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct Descriptors<'a> {
    device: &'a ash::Device,
    types: Vec<(u32, vk::DescriptorType)>,
    pub set_layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
}

impl<'a> Descriptors<'a> {
    // All bindings live in set 0: 0 host support UBO, 1-32 guest cbufs, storage buffers 33-35, textures and surfaces from 36.
    pub fn new(
        device: &'a ash::Device,
        bindings: &[(u32, vk::DescriptorType)],
        stages: vk::ShaderStageFlags,
    ) -> Result<Self, VkError> {
        let (layout_bindings, sizes): (Vec<_>, Vec<_>) = bindings
            .iter()
            .map(|&(binding, ty)| {
                let layout = vk::DescriptorSetLayoutBinding::default()
                    .binding(binding)
                    .descriptor_type(ty)
                    .descriptor_count(1)
                    .stage_flags(stages);
                let size = vk::DescriptorPoolSize::default().ty(ty).descriptor_count(1);
                (layout, size)
            })
            .unzip();
        let mut s = Self {
            device,
            types: bindings.to_vec(),
            set_layout: vk::DescriptorSetLayout::null(),
            pool: vk::DescriptorPool::null(),
            set: vk::DescriptorSet::null(),
        };
        let set_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        s.set_layout = unsafe { device.create_descriptor_set_layout(&set_info, None)? };
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&sizes);
        s.pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };
        let set_layouts = [s.set_layout];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(s.pool)
            .set_layouts(&set_layouts);
        s.set = unsafe { device.allocate_descriptor_sets(&alloc)? }[0];
        Ok(s)
    }

    pub fn bind(&mut self, buffers: &[(u32, &HostBuffer)]) {
        for &(binding, buf) in buffers {
            let ty = self.types.iter().find(|t| t.0 == binding);
            let info = [vk::DescriptorBufferInfo::default()
                .buffer(buf.buffer)
                .range(buf.size)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(self.set)
                .dst_binding(binding)
                .descriptor_type(ty.expect("unknown binding").1)
                .buffer_info(&info);
            unsafe { self.device.update_descriptor_sets(&[write], &[]) };
        }
    }

    pub fn bind_acceleration_structure(&mut self, binding: u32, structure: &AccelerationStructure) {
        let structures = [structure.handle];
        let mut info = vk::WriteDescriptorSetAccelerationStructureKHR::default().acceleration_structures(&structures);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.set)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .push_next(&mut info);
        unsafe { self.device.update_descriptor_sets(&[write], &[]) };
    }

    pub fn bind_images(&mut self, images: &[(u32, &HostImage)]) {
        for &(binding, img) in images {
            let ty = self.types.iter().find(|t| t.0 == binding);
            let info = [vk::DescriptorImageInfo::default()
                .sampler(img.sampler)
                .image_view(img.view)
                .image_layout(vk::ImageLayout::GENERAL)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(self.set)
                .dst_binding(binding)
                .descriptor_type(ty.expect("unknown binding").1)
                .image_info(&info);
            unsafe { self.device.update_descriptor_sets(&[write], &[]) };
        }
    }
}

impl Drop for Descriptors<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
            self.device.destroy_descriptor_set_layout(self.set_layout, None);
        }
    }
}

pub struct ComputePipeline<'a> {
    device: &'a ash::Device,
    pub descriptors: Descriptors<'a>,
    pub shader: vk::ShaderModule,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

impl<'a> ComputePipeline<'a> {
    pub fn new(
        state: &'a VkState,
        spirv: &[u32],
        bindings: &[(u32, vk::DescriptorType)],
    ) -> Result<Self, VkError> {
        let d = &state.device;
        let module_info = vk::ShaderModuleCreateInfo::default().code(spirv);
        let mut p = Self {
            device: d,
            descriptors: Descriptors::new(d, bindings, vk::ShaderStageFlags::COMPUTE)?,
            shader: unsafe { d.create_shader_module(&module_info, None)? },
            layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
        };
        let set_layouts = [p.descriptors.set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        p.layout = unsafe { d.create_pipeline_layout(&layout_info, None)? };
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(p.shader)
            .name(c"main");
        let info = [vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(p.layout)];
        p.pipeline = unsafe { d.create_compute_pipelines(state.pipeline_cache, &info, None) }
            .map_err(|(_, e)| e)?[0];
        Ok(p)
    }
}

impl Drop for ComputePipeline<'_> {
    fn drop(&mut self) {
        let d = self.device;
        unsafe {
            d.destroy_pipeline(self.pipeline, None);
            d.destroy_pipeline_layout(self.layout, None);
            d.destroy_shader_module(self.shader, None);
        }
    }
}

impl VkState {
    pub fn dispatch(&self, pipeline: &ComputePipeline, groups: [u32; 3]) -> Result<(), VkError> {
        let compute = vk::PipelineBindPoint::COMPUTE;
        let src = vk::PipelineStageFlags::COMPUTE_SHADER;
        let dst = vk::PipelineStageFlags::HOST;
        // Shader writes are not visible to host reads after the fence wait without this barrier.
        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);
        let d = &self.device;
        self.submit_and_wait(|cmd| unsafe {
            d.cmd_bind_pipeline(cmd, compute, pipeline.pipeline);
            d.cmd_bind_descriptor_sets(cmd, compute, pipeline.layout, 0, &[pipeline.descriptors.set], &[]);
            d.cmd_dispatch(cmd, groups[0], groups[1], groups[2]);
            d.cmd_pipeline_barrier(cmd, src, dst, Default::default(), &[barrier], &[], &[]);
        })
    }

    // The command pool, its buffer and the queue are externally synchronized; the lock is that sync.
    pub(super) fn submit_and_wait(&self, record: impl FnOnce(vk::CommandBuffer)) -> Result<(), VkError> {
        let submit = self.submit.lock().unwrap_or_else(|e| e.into_inner());
        let (d, cmd, fence) = (&self.device, submit.command_buffer, submit.fence);
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let cmds = [cmd];
        let submit_info = [vk::SubmitInfo::default().command_buffers(&cmds)];
        unsafe {
            d.reset_fences(&[fence])?;
            d.begin_command_buffer(cmd, &begin)?;
            record(cmd);
            d.end_command_buffer(cmd)?;
            d.queue_submit(self.queue, &submit_info, fence)?;
            d.wait_for_fences(&[fence], true, u64::MAX)?;
        }
        Ok(())
    }
}
