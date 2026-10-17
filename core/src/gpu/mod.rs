use ash::vk;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

pub mod compute;
pub mod frame;
pub mod present;
pub mod sm86;
pub mod spirv;

#[derive(Debug)]
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
    pub device: ash::Device,
    pub queue_family: u32,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
    pub submit: std::sync::Mutex<compute::Submit>,
    pub signed_zero_inf_nan_preserve: bool,
    pub denorm_preserve: bool,
    pub shader_float16: bool,
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
    pub fn new() -> Result<Self, VkError> {
        let entry = unsafe { ash::Entry::load() }.map_err(|_| VkError::Loader)?;
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_2);
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
        let (physical_device, queue_family, device, command_pool, shader_float16) =
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
            entry,
            instance,
            physical_device,
            device,
            command_pool,
            submit: Default::default(),
            signed_zero_inf_nan_preserve: float_controls
                .shader_signed_zero_inf_nan_preserve_float32
                == vk::TRUE,
            // The entry point only sets the 32-bit mode, which NONE forbids.
            denorm_preserve: float_controls.shader_denorm_preserve_float32 == vk::TRUE
                && float_controls.denorm_behavior_independence
                    != vk::ShaderFloatControlsIndependence::NONE,
            shader_float16,
        };
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1);
        let submit = state.submit.get_mut().unwrap();
        submit.command_buffer = unsafe { state.device.allocate_command_buffers(&alloc)? }[0];
        submit.fence = unsafe { state.device.create_fence(&Default::default(), None)? };
        Ok(state)
    }

    fn open_device(
        instance: &ash::Instance,
        surfaces: bool,
    ) -> Result<(vk::PhysicalDevice, u32, ash::Device, vk::CommandPool, bool), VkError> {
        // Blits into swapchain images need a graphics queue.
        let queue_flags = if surfaces {
            vk::QueueFlags::COMPUTE | vk::QueueFlags::GRAPHICS
        } else {
            vk::QueueFlags::COMPUTE
        };
        let (physical_device, queue_family) = unsafe { instance.enumerate_physical_devices()? }
            .into_iter()
            .filter(|&pd| {
                unsafe { instance.get_physical_device_properties(pd) }.api_version
                    >= vk::API_VERSION_1_2
            })
            .find_map(|pd| {
                unsafe { instance.get_physical_device_queue_family_properties(pd) }
                    .iter()
                    .position(|q| q.queue_flags.contains(queue_flags))
                    .map(|i| (pd, i as u32))
            })
            .ok_or(VkError::NoDevice)?;
        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let mut float16_int8 = vk::PhysicalDeviceShaderFloat16Int8Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut float16_int8);
        unsafe { instance.get_physical_device_features2(physical_device, &mut features) };
        let shader_float16 = float16_int8.shader_float16 == vk::TRUE;
        let swapchain = [ash::khr::swapchain::NAME.as_ptr()];
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(if surfaces { &swapchain } else { &[] })
            .push_next(&mut float16_int8);
        let device = unsafe { instance.create_device(physical_device, &device_info, None)? };
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        match unsafe { device.create_command_pool(&pool_info, None) } {
            Ok(pool) => Ok((physical_device, queue_family, device, pool, shader_float16)),
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
