//! Host-visible buffers and compute pipelines dispatched synchronously on the `VkState` queue.
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
        let d = &state.device;
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::UNIFORM_BUFFER);
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
        // Coherent memory stays mapped and needs no flush or invalidate around host access.
        let flags = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let types = &props.memory_types[..props.memory_type_count as usize];
        let memory_type = (0..)
            .zip(types)
            .find(|&(i, t)| req.memory_type_bits >> i & 1 == 1 && t.property_flags.contains(flags))
            .ok_or(VkError::Vk(vk::Result::ERROR_FEATURE_NOT_PRESENT))?
            .0;
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(memory_type);
        unsafe {
            buf.memory = d.allocate_memory(&alloc, None)?;
            d.bind_buffer_memory(buf.buffer, buf.memory, 0)?;
            buf.ptr = d
                .map_memory(buf.memory, 0, size, vk::MemoryMapFlags::empty())?
                .cast();
        }
        Ok(buf)
    }

    pub fn write(&mut self, data: &[u8]) {
        assert!(data.len() as u64 <= self.size);
        unsafe { self.ptr.copy_from_nonoverlapping(data.as_ptr(), data.len()) };
    }

    pub fn read(&self, out: &mut [u8]) {
        assert!(out.len() as u64 <= self.size);
        unsafe { self.ptr.copy_to_nonoverlapping(out.as_mut_ptr(), out.len()) };
    }
}

impl Drop for HostBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct ComputePipeline<'a> {
    device: &'a ash::Device,
    types: Vec<(u32, vk::DescriptorType)>,
    pub shader: vk::ShaderModule,
    pub set_layout: vk::DescriptorSetLayout,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
}

impl<'a> ComputePipeline<'a> {
    // All bindings live in set 0: 0 host support UBO, 1-8 guest cbufs, storage buffers from 9.
    pub fn new(
        state: &'a VkState,
        spirv: &[u32],
        bindings: &[(u32, vk::DescriptorType)],
    ) -> Result<Self, VkError> {
        let d = &state.device;
        let (layout_bindings, sizes): (Vec<_>, Vec<_>) = bindings
            .iter()
            .map(|&(binding, ty)| {
                let layout = vk::DescriptorSetLayoutBinding::default()
                    .binding(binding)
                    .descriptor_type(ty)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE);
                let size = vk::DescriptorPoolSize::default().ty(ty).descriptor_count(1);
                (layout, size)
            })
            .unzip();
        let module_info = vk::ShaderModuleCreateInfo::default().code(spirv);
        let mut p = Self {
            device: d,
            types: bindings.to_vec(),
            shader: unsafe { d.create_shader_module(&module_info, None)? },
            set_layout: vk::DescriptorSetLayout::null(),
            layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            pool: vk::DescriptorPool::null(),
            set: vk::DescriptorSet::null(),
        };
        let set_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        p.set_layout = unsafe { d.create_descriptor_set_layout(&set_info, None)? };
        let set_layouts = [p.set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        p.layout = unsafe { d.create_pipeline_layout(&layout_info, None)? };
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(p.shader)
            .name(c"main");
        let info = [vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(p.layout)];
        p.pipeline = unsafe { d.create_compute_pipelines(vk::PipelineCache::null(), &info, None) }
            .map_err(|(_, e)| e)?[0];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&sizes);
        p.pool = unsafe { d.create_descriptor_pool(&pool_info, None)? };
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(p.pool)
            .set_layouts(&set_layouts);
        p.set = unsafe { d.allocate_descriptor_sets(&alloc)? }[0];
        Ok(p)
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
}

impl Drop for ComputePipeline<'_> {
    fn drop(&mut self) {
        let d = self.device;
        unsafe {
            d.destroy_pipeline(self.pipeline, None);
            d.destroy_pipeline_layout(self.layout, None);
            d.destroy_descriptor_pool(self.pool, None);
            d.destroy_descriptor_set_layout(self.set_layout, None);
            d.destroy_shader_module(self.shader, None);
        }
    }
}

impl VkState {
    // The command pool, its buffer and the queue are externally synchronized; the lock is that sync.
    pub fn dispatch(&self, pipeline: &ComputePipeline, groups: [u32; 3]) -> Result<(), VkError> {
        let submit = self.submit.lock().unwrap_or_else(|e| e.into_inner());
        let (d, cmd, fence) = (&self.device, submit.command_buffer, submit.fence);
        let compute = vk::PipelineBindPoint::COMPUTE;
        let src = vk::PipelineStageFlags::COMPUTE_SHADER;
        let dst = vk::PipelineStageFlags::HOST;
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // Shader writes are not visible to host reads after the fence wait without this barrier.
        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);
        let cmds = [cmd];
        let submit_info = [vk::SubmitInfo::default().command_buffers(&cmds)];
        unsafe {
            d.reset_fences(&[fence])?;
            d.begin_command_buffer(cmd, &begin)?;
            d.cmd_bind_pipeline(cmd, compute, pipeline.pipeline);
            d.cmd_bind_descriptor_sets(cmd, compute, pipeline.layout, 0, &[pipeline.set], &[]);
            d.cmd_dispatch(cmd, groups[0], groups[1], groups[2]);
            d.cmd_pipeline_barrier(cmd, src, dst, Default::default(), &[barrier], &[], &[]);
            d.end_command_buffer(cmd)?;
            d.queue_submit(self.queue, &submit_info, fence)?;
            d.wait_for_fences(&[fence], true, u64::MAX)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::spirv::{Emitter, decoration as deco, storage_class::STORAGE_BUFFER as SB, *};

    fn store_42_at_binding_9() -> Vec<u32> {
        let mut e = Emitter::new();
        e.emit_header();
        e.emit_capability(capability::SHADER);
        e.emit_memory_model(0, 1);
        let u32_ty = e.emit_type_int(32, 0);
        let array = e.emit_type_runtime_array(u32_ty);
        let block = e.emit_type_struct(&[array]);
        let block_ptr = e.emit_type_pointer(SB, block);
        let var = e.emit_variable(block_ptr, SB);
        let (zero, value) = (
            e.emit_constant_typed(u32_ty, 0u32),
            e.emit_constant_typed(u32_ty, 42u32),
        );
        let void = e.emit_type_void();
        let fn_ty = e.emit_type_function(void, &[]);
        let func = e.emit_function(void, 0, fn_ty);
        e.emit_label();
        let elem_ptr = e.emit_type_pointer(SB, u32_ty);
        let elem = e.emit_access_chain(elem_ptr, var, &[zero, zero]);
        e.emit_store(elem, value);
        e.emit_return();
        e.emit_function_end();
        e.emit_entry_point(execution_model::GLCOMPUTE, func, "main", &[var]);
        e.emit_execution_mode(func, execution_mode::LOCAL_SIZE, &[1, 1, 1]);
        e.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
        e.emit_decorate(block, deco::BLOCK, &[]);
        e.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
        e.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        e.emit_decorate(var, deco::BINDING, &[9]);
        e.finalize();
        e.words()
    }

    #[test]
    fn dispatch_stores_into_storage_buffer() {
        let words = store_42_at_binding_9();
        let state = match VkState::new() {
            Ok(s) => s,
            Err(e @ (VkError::Loader | VkError::NoDevice)) => return eprintln!("skipping: {e:?}"),
            Err(e) => panic!("vulkan device creation failed: {e:?}"),
        };
        let mut buf = HostBuffer::new(&state, 16).unwrap();
        buf.write(&[0; 16]);
        let bindings = [(9, vk::DescriptorType::STORAGE_BUFFER)];
        let mut pipeline = ComputePipeline::new(&state, &words, &bindings).unwrap();
        pipeline.bind(&[(9, &buf)]);
        state.dispatch(&pipeline, [1, 1, 1]).unwrap();
        let mut out = [0u8; 4];
        buf.read(&mut out);
        assert_eq!(u32::from_le_bytes(out), 42);
    }
}
