//! Host-visible buffers and compute pipelines dispatched synchronously on the `VkState` queue.
use super::{VkError, VkState};
use ash::vk;

pub struct HostBuffer<'a> {
    device: &'a ash::Device,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
}

impl<'a> HostBuffer<'a> {
    pub fn new(state: &'a VkState, size: u64) -> Result<Self, VkError> {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::UNIFORM_BUFFER);
        let (d, instance) = (&state.device, &state.instance);
        let mut buf = Self {
            device: d,
            buffer: unsafe { d.create_buffer(&info, None)? },
            memory: vk::DeviceMemory::null(),
            size,
        };
        let req = unsafe { d.get_buffer_memory_requirements(buf.buffer) };
        let props =
            unsafe { instance.get_physical_device_memory_properties(state.physical_device) };
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
        buf.memory = unsafe { d.allocate_memory(&alloc, None)? };
        unsafe { d.bind_buffer_memory(buf.buffer, buf.memory, 0)? };
        Ok(buf)
    }

    pub fn write(&self, data: &[u8]) -> Result<(), VkError> {
        self.mapped(|m| m[..data.len()].copy_from_slice(data))
    }

    pub fn read(&self, out: &mut [u8]) -> Result<(), VkError> {
        self.mapped(|m| out.copy_from_slice(&m[..out.len()]))
    }

    fn mapped(&self, f: impl FnOnce(&mut [u8])) -> Result<(), VkError> {
        unsafe {
            let flags = vk::MemoryMapFlags::empty();
            let ptr = self.device.map_memory(self.memory, 0, self.size, flags)?;
            let mapped = std::slice::from_raw_parts_mut(ptr.cast(), self.size as usize);
            f(mapped);
            self.device.unmap_memory(self.memory);
        }
        Ok(())
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
    pub fn new(
        state: &'a VkState,
        spirv: &[u32],
        bindings: &[(u32, vk::DescriptorType)],
    ) -> Result<Self, VkError> {
        let d = &state.device;
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

    pub fn bind(&self, buffers: &[(u32, &HostBuffer)]) {
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
    pub fn dispatch(&self, pipeline: &ComputePipeline, groups: [u32; 3]) -> Result<(), VkError> {
        let d = &self.device;
        let compute = vk::PipelineBindPoint::COMPUTE;
        let src = vk::PipelineStageFlags::COMPUTE_SHADER;
        let dst = vk::PipelineStageFlags::HOST;
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .command_buffer_count(1);
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);
        unsafe {
            let cmds = d.allocate_command_buffers(&alloc)?;
            let cmd = cmds[0];
            let fence = d.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let result = (|| {
                d.begin_command_buffer(cmd, &begin)?;
                d.cmd_bind_pipeline(cmd, compute, pipeline.pipeline);
                d.cmd_bind_descriptor_sets(cmd, compute, pipeline.layout, 0, &[pipeline.set], &[]);
                d.cmd_dispatch(cmd, groups[0], groups[1], groups[2]);
                d.cmd_pipeline_barrier(cmd, src, dst, Default::default(), &[barrier], &[], &[]);
                d.end_command_buffer(cmd)?;
                let submit = [vk::SubmitInfo::default().command_buffers(&cmds)];
                d.queue_submit(self.queue, &submit, fence)?;
                d.wait_for_fences(&[fence], true, u64::MAX)
            })();
            d.destroy_fence(fence, None);
            d.free_command_buffers(self.command_pool, &cmds);
            Ok(result?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::spirv::{Emitter, decoration as deco, storage_class::STORAGE_BUFFER as SB, *};

    fn store_42_at_binding_9() -> Vec<u32> {
        let mut body = Emitter::new();
        let u32_ty = body.emit_type_int(32, 0);
        let array = body.emit_type_runtime_array(u32_ty);
        let block = body.emit_type_struct(&[array]);
        let block_ptr = body.emit_type_pointer(SB, block);
        let elem_ptr = body.emit_type_pointer(SB, u32_ty);
        let var = body.emit_variable(block_ptr, SB);
        let zero = body.emit_constant_typed(u32_ty, 0u32);
        let value = body.emit_constant_typed(u32_ty, 42u32);
        let void = body.emit_type_void();
        let fn_ty = body.emit_type_function(void, &[]);
        let func = body.emit_function(void, 0, fn_ty);
        body.emit_label();
        let elem = body.emit_access_chain(elem_ptr, var, &[zero, zero]);
        body.emit_store(elem, value);
        body.emit_return();
        body.emit_function_end();

        let mut head = Emitter::new();
        head.emit_header();
        head.emit_capability(capability::SHADER);
        head.emit_memory_model(0, 1);
        head.emit_entry_point(execution_model::GLCOMPUTE, func, "main", &[var]);
        head.emit_execution_mode(func, execution_mode::LOCAL_SIZE, &[1, 1, 1]);
        head.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
        head.emit_decorate(block, deco::BLOCK, &[]);
        head.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
        head.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        head.emit_decorate(var, deco::BINDING, &[9]);
        let mut words = [head.words(), body.words()].concat();
        words[3] = body.id_bound();
        words
    }

    #[test]
    fn dispatch_stores_into_storage_buffer() {
        let words = store_42_at_binding_9();
        let path = std::env::temp_dir().join("oboromi_store_42.spv");
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        std::fs::write(&path, bytes).unwrap();
        let val = std::process::Command::new("spirv-val")
            .args(["--target-env", "vulkan1.2"])
            .arg(&path)
            .output();
        if let Ok(out) = val {
            assert!(out.status.success(), "{out:?}");
        }
        let state = match VkState::new() {
            Ok(s) => s,
            Err(e @ (VkError::Loader | VkError::NoDevice)) => return eprintln!("skipping: {e:?}"),
            Err(e) => panic!("vulkan device creation failed: {e:?}"),
        };
        let buf = HostBuffer::new(&state, 16).unwrap();
        buf.write(&[0; 16]).unwrap();
        let bindings = [(9, vk::DescriptorType::STORAGE_BUFFER)];
        let pipeline = ComputePipeline::new(&state, &words, &bindings).unwrap();
        pipeline.bind(&[(9, &buf)]);
        state.dispatch(&pipeline, [1, 1, 1]).unwrap();
        let mut out = [0u8; 4];
        buf.read(&mut out).unwrap();
        assert_eq!(u32::from_le_bytes(out), 42);
    }
}
