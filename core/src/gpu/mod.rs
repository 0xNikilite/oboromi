use ash::vk;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

pub mod bnsh;
pub mod compute;
pub mod compute_class;
pub mod demo;
pub mod demo_post;
pub mod engine;
pub mod fifo;
pub mod frame;
pub mod graphics;
pub mod memory;
pub mod pipeline_cache;
pub mod present;
pub mod sm86;
pub mod spirv;
pub mod threed;
pub mod texture;

#[derive(Clone, Copy, Debug)]
pub enum VkError {
    Loader,
    NoDevice,
    Vk(vk::Result),
}

impl From<vk::Result> for VkError {
    fn from(e: vk::Result) -> Self {
        Self::Vk(e)
    }
}

pub struct VkState {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub shader_float64: bool,
    pub device: ash::Device,
    pub queue_family: u32,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
    pub submit: std::sync::Mutex<compute::Submit>,
    pub pipeline_cache: vk::PipelineCache,
    /// Graphics pipelines link from libraries shared between them, compiled once per stage or interface.
    pub graphics_pipeline_library: bool,
    pub(crate) libraries: std::sync::Mutex<std::collections::HashMap<(vk::GraphicsPipelineLibraryFlagsEXT, u64), vk::Pipeline>>,
    /// Present when colour blend enable, blend equation and colour write mask are dynamic state.
    pub extended_dynamic_state3: Option<ash::ext::extended_dynamic_state3::Device>,
    pub signed_zero_inf_nan_preserve: [bool; 3],
    pub denorm_preserve: [bool; 3],
    pub shader_float16: bool,
    pub shader_buffer_int64_atomics: bool,
    pub shader_shared_int64_atomics: bool,
    pub index_type_uint8: bool,
    pub surface: Option<ash::khr::surface::Instance>,
    pub swapchain: Option<ash::khr::swapchain::Device>,
}

static SURFACES: AtomicBool = AtomicBool::new(false);

/// `device`, but created with every window surface extension the loader has if this is the first call.
pub fn surface_device() -> Result<&'static VkState, VkError> {
    SURFACES.store(true, Relaxed);
    device()
}

pub fn device() -> Result<&'static VkState, VkError> {
    static DEVICE: std::sync::OnceLock<VkState> = std::sync::OnceLock::new();
    if let Some(state) = DEVICE.get() {
        return Ok(state);
    }
    let state = VkState::new()?;
    Ok(DEVICE.get_or_init(|| state))
}

impl VkState {
    pub fn is_llvmpipe(&self) -> bool {
        let props = unsafe { self.instance.get_physical_device_properties(self.physical_device) };
        props.device_name_as_c_str().is_ok_and(|n| n.to_bytes().starts_with(b"llvmpipe"))
    }

    pub fn new() -> Result<Self, VkError> {
        let entry = unsafe { ash::Entry::load() }.map_err(|_| VkError::Loader)?;
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
        let extensions = if SURFACES.load(Relaxed) {
            unsafe { entry.enumerate_instance_extension_properties(None)? }
        } else {
            Vec::new()
        };
        let surface_extensions: Vec<_> = extensions
            .iter()
            .filter_map(|e| e.extension_name_as_c_str().ok())
            .filter(|name| name.to_bytes().ends_with(b"_surface"))
            .map(std::ffi::CStr::as_ptr)
            .collect();
        let surfaces = !surface_extensions.is_empty();
        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app)
            .enabled_extension_names(&surface_extensions);
        let instance =
            unsafe { entry.create_instance(&instance_info, None) }.map_err(|e| match e {
                vk::Result::ERROR_INCOMPATIBLE_DRIVER => VkError::NoDevice,
                e => VkError::Vk(e),
            })?;
        let (physical_device, queue_family, device, command_pool, shader_float16, shader_float64, shader_buffer_int64_atomics, shader_shared_int64_atomics, index_type_uint8, [graphics_pipeline_library, extended_dynamic_state3]) =
            Self::open_device(&instance, surfaces)
                .inspect_err(|_| unsafe { instance.destroy_instance(None) })?;
        let mut float_controls = vk::PhysicalDeviceFloatControlsProperties::default();
        let mut props = vk::PhysicalDeviceProperties2::default().push_next(&mut float_controls);
        unsafe { instance.get_physical_device_properties2(physical_device, &mut props) };
        let mut state = Self {
            queue_family,
            queue: unsafe { device.get_device_queue(queue_family, 0) },
            surface: surfaces.then(|| ash::khr::surface::Instance::new(&entry, &instance)),
            swapchain: surfaces.then(|| ash::khr::swapchain::Device::new(&instance, &device)),
            extended_dynamic_state3: extended_dynamic_state3.then(|| ash::ext::extended_dynamic_state3::Device::new(&instance, &device)),
            entry,
            instance,
            physical_device,
            shader_float64,
            device,
            command_pool,
            submit: Default::default(),
            pipeline_cache: vk::PipelineCache::null(),
            graphics_pipeline_library,
            libraries: Default::default(),
            signed_zero_inf_nan_preserve: [
                float_controls.shader_signed_zero_inf_nan_preserve_float16,
                float_controls.shader_signed_zero_inf_nan_preserve_float32,
                float_controls.shader_signed_zero_inf_nan_preserve_float64,
            ]
            .map(|b| b == vk::TRUE),
            denorm_preserve: {
                let [f16, f32, f64] = [
                    float_controls.shader_denorm_preserve_float16,
                    float_controls.shader_denorm_preserve_float32,
                    float_controls.shader_denorm_preserve_float64,
                ]
                .map(|b| b == vk::TRUE);
                match float_controls.denorm_behavior_independence {
                    vk::ShaderFloatControlsIndependence::ALL => [f16, f32, f64],
                    vk::ShaderFloatControlsIndependence::TYPE_32_ONLY => [f16 && f64, f32, f16 && f64],
                    _ => [f16 && f32 && f64; 3],
                }
            },
            shader_float16,
            shader_buffer_int64_atomics,
            shader_shared_int64_atomics,
            index_type_uint8,
        };
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let submit = state.submit.get_mut().unwrap();
        submit.command_buffer = unsafe { state.device.allocate_command_buffers(&alloc)? }[0];
        submit.fence = unsafe { state.device.create_fence(&Default::default(), None)? };
        // Created without EXTERNALLY_SYNCHRONIZED_BIT, so the implementation synchronises it and any thread may create
        // pipelines through it (VUID-vkCreateGraphicsPipelines-pipelineCache-02876, fundamentals-threadingbehavior).
        state.pipeline_cache = unsafe { state.device.create_pipeline_cache(&Default::default(), None)? };
        Ok(state)
    }

    fn open_device(
        instance: &ash::Instance,
        surfaces: bool,
    ) -> Result<(vk::PhysicalDevice, u32, ash::Device, vk::CommandPool, bool, bool, bool, bool, bool, [bool; 2]), VkError> {
        let (physical_device, queue_family) = unsafe { instance.enumerate_physical_devices()? }
            .into_iter()
            .filter(|&pd| {
                unsafe { instance.get_physical_device_properties(pd) }.api_version
                    >= vk::API_VERSION_1_3
            })
            .find_map(|pd| {
                unsafe { instance.get_physical_device_queue_family_properties(pd) }
                    .iter()
                    .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE))
                    .map(|i| (pd, i as u32))
            })
            .ok_or(VkError::NoDevice)?;
        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let mut float16_int8 = vk::PhysicalDeviceShaderFloat16Int8Features::default();
        let mut atomic_int64 = vk::PhysicalDeviceShaderAtomicInt64Features::default();
        let mut derivatives = vk::PhysicalDeviceComputeShaderDerivativesFeaturesNV::default();
        let mut uint8_indices = vk::PhysicalDeviceIndexTypeUint8FeaturesKHR::default();
        let mut library = vk::PhysicalDeviceGraphicsPipelineLibraryFeaturesEXT::default();
        let mut dynamic3 = vk::PhysicalDeviceExtendedDynamicState3FeaturesEXT::default();
        let mut features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut float16_int8)
            .push_next(&mut atomic_int64)
            .push_next(&mut derivatives)
            .push_next(&mut uint8_indices)
            .push_next(&mut library)
            .push_next(&mut dynamic3);
        unsafe { instance.get_physical_device_features2(physical_device, &mut features) };
        let shader_float64 = features.features.shader_float64 == vk::TRUE;
        let shader_int64 = features.features.shader_int64 == vk::TRUE;
        let texture_compression_bc = features.features.texture_compression_bc == vk::TRUE;
        let _ = features;
        let shader_float16 = float16_int8.shader_float16 == vk::TRUE;
        let shader_buffer_int64_atomics = atomic_int64.shader_buffer_int64_atomics == vk::TRUE;
        let shader_shared_int64_atomics = atomic_int64.shader_shared_int64_atomics == vk::TRUE;
        // The KHR extension reuses the NV feature struct, and only it is exposed by lavapipe.
        let derivatives_extension = c"VK_KHR_compute_shader_derivatives";
        let extensions = unsafe { instance.enumerate_device_extension_properties(physical_device)? };
        let has = |name| extensions.iter().any(|e| e.extension_name_as_c_str() == Ok(name));
        let linear_derivatives = derivatives.compute_derivative_group_linear == vk::TRUE && has(derivatives_extension);
        let index_type_uint8 = uint8_indices.index_type_uint8 == vk::TRUE && has(vk::KHR_INDEX_TYPE_UINT8_NAME);
        let graphics_pipeline_library =
            library.graphics_pipeline_library == vk::TRUE && has(vk::EXT_GRAPHICS_PIPELINE_LIBRARY_NAME);
        let extended_dynamic_state3 = [
            dynamic3.extended_dynamic_state3_color_blend_enable,
            dynamic3.extended_dynamic_state3_color_blend_equation,
            dynamic3.extended_dynamic_state3_color_write_mask,
        ] == [vk::TRUE; 3]
            && has(vk::EXT_EXTENDED_DYNAMIC_STATE3_NAME);
        let enabled_extensions: Vec<_> = [
            (linear_derivatives, derivatives_extension),
            (index_type_uint8, vk::KHR_INDEX_TYPE_UINT8_NAME),
            (graphics_pipeline_library, vk::KHR_PIPELINE_LIBRARY_NAME),
            (graphics_pipeline_library, vk::EXT_GRAPHICS_PIPELINE_LIBRARY_NAME),
            (extended_dynamic_state3, vk::EXT_EXTENDED_DYNAMIC_STATE3_NAME),
            (surfaces, ash::khr::swapchain::NAME),
        ]
        .into_iter()
        .filter_map(|(enabled, name)| enabled.then_some(name.as_ptr()))
        .collect();
        let device_features = vk::PhysicalDeviceFeatures::default()
            .shader_float64(shader_float64)
            .shader_int64(shader_int64)
            .texture_compression_bc(texture_compression_bc);
        let mut enabled_float16 = vk::PhysicalDeviceShaderFloat16Int8Features::default()
            .shader_float16(shader_float16);
        let mut enabled_atomic_int64 = vk::PhysicalDeviceShaderAtomicInt64Features::default()
            .shader_buffer_int64_atomics(shader_buffer_int64_atomics)
            .shader_shared_int64_atomics(shader_shared_int64_atomics);
        let mut enabled_derivatives = vk::PhysicalDeviceComputeShaderDerivativesFeaturesNV::default()
            .compute_derivative_group_linear(linear_derivatives);
        let mut enabled_uint8_indices = vk::PhysicalDeviceIndexTypeUint8FeaturesKHR::default().index_type_uint8(index_type_uint8);
        let mut enabled_library = vk::PhysicalDeviceGraphicsPipelineLibraryFeaturesEXT::default().graphics_pipeline_library(true);
        let mut enabled_dynamic3 = vk::PhysicalDeviceExtendedDynamicState3FeaturesEXT::default()
            .extended_dynamic_state3_color_blend_enable(true)
            .extended_dynamic_state3_color_blend_equation(true)
            .extended_dynamic_state3_color_write_mask(true);
        let mut enabled_vulkan13 = vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);
        let mut device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&enabled_extensions)
            .enabled_features(&device_features)
            .push_next(&mut enabled_float16)
            .push_next(&mut enabled_atomic_int64)
            .push_next(&mut enabled_derivatives)
            .push_next(&mut enabled_vulkan13);
        if index_type_uint8 {
            device_info = device_info.push_next(&mut enabled_uint8_indices);
        }
        if graphics_pipeline_library {
            device_info = device_info.push_next(&mut enabled_library);
        }
        if extended_dynamic_state3 {
            device_info = device_info.push_next(&mut enabled_dynamic3);
        }
        let device = unsafe { instance.create_device(physical_device, &device_info, None)? };
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        match unsafe { device.create_command_pool(&pool_info, None) } {
            Ok(pool) => Ok((physical_device, queue_family, device, pool, shader_float16, shader_float64, shader_buffer_int64_atomics, shader_shared_int64_atomics, index_type_uint8, [graphics_pipeline_library, extended_dynamic_state3])),
            Err(e) => {
                unsafe { device.destroy_device(None) };
                Err(e.into())
            }
        }
    }
}

impl Drop for VkState {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            let submit = self.submit.get_mut().unwrap_or_else(|e| e.into_inner());
            self.device.destroy_fence(submit.fence, None);
            self.device.destroy_pipeline_cache(self.pipeline_cache, None);
            let libraries = self.libraries.get_mut().unwrap_or_else(|e| e.into_inner());
            libraries.values().for_each(|&library| self.device.destroy_pipeline(library, None));
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Default)]
pub struct State {
    // pub registers: [u32; 256],
    // pub predicates: u32,
    pub shared_memory: *mut u8,
    pub global_memory: *mut u8,
    pub pc: u64,
}

impl State {
    pub fn new() -> Self {
        Self {
            // registers: [0; 256],
            // predicates: 0,
            shared_memory: core::ptr::null_mut(),
            global_memory: core::ptr::null_mut(),
            pc: 0,
        }
    }
}
