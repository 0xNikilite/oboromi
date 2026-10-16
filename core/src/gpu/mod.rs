use ash::vk;

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
}

impl VkState {
    pub fn new() -> Result<Self, VkError> {
        let entry = unsafe { ash::Entry::load() }.map_err(|_| VkError::Loader)?;
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_2);
        let instance_info = vk::InstanceCreateInfo::default().application_info(&app);
        let instance =
            unsafe { entry.create_instance(&instance_info, None) }.map_err(|e| match e {
                vk::Result::ERROR_INCOMPATIBLE_DRIVER => VkError::NoDevice,
                e => VkError::Vk(e),
            })?;
        match Self::open_device(&instance) {
            Ok((physical_device, queue_family, device, command_pool)) => Ok(Self {
                queue: unsafe { device.get_device_queue(queue_family, 0) },
                entry,
                instance,
                physical_device,
                device,
                queue_family,
                command_pool,
            }),
            Err(e) => {
                unsafe { instance.destroy_instance(None) };
                Err(e)
            }
        }
    }

    fn open_device(
        instance: &ash::Instance,
    ) -> Result<(vk::PhysicalDevice, u32, ash::Device, vk::CommandPool), VkError> {
        let (physical_device, queue_family) = unsafe { instance.enumerate_physical_devices()? }
            .into_iter()
            .filter(|&pd| {
                unsafe { instance.get_physical_device_properties(pd) }.api_version
                    >= vk::API_VERSION_1_2
            })
            .find_map(|pd| {
                unsafe { instance.get_physical_device_queue_family_properties(pd) }
                    .iter()
                    .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
                    .map(|i| (pd, i as u32))
            })
            .ok_or(VkError::NoDevice)?;
        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let device_info = vk::DeviceCreateInfo::default().queue_create_infos(&queue_info);
        let device = unsafe { instance.create_device(physical_device, &device_info, None)? };
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        match unsafe { device.create_command_pool(&pool_info, None) } {
            Ok(pool) => Ok((physical_device, queue_family, device, pool)),
            Err(e) => {
                unsafe { device.destroy_device(None) };
                Err(e.into())
            }
        }
    }
}

impl Drop for VkState {
    fn drop(&mut self) {
        // Children before parents, and nothing may still be executing on the queue.
        unsafe {
            let _ = self.device.device_wait_idle();
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
    pub vk: Option<VkState>,
}

impl State {
    pub fn new() -> Self {
        Self {
            // registers: [0; 256],
            // predicates: 0,
            shared_memory: core::ptr::null_mut(),
            global_memory: core::ptr::null_mut(),
            pc: 0,
            vk: None,
        }
    }
    pub fn init(&mut self) -> Result<(), VkError> {
        self.vk = Some(VkState::new()?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_state_new_or_skip() {
        match VkState::new() {
            Ok(s) => {
                let p = unsafe { s.instance.get_physical_device_properties(s.physical_device) };
                eprintln!("vulkan device: {:?}", p.device_name_as_c_str());
            }
            Err(e @ (VkError::Loader | VkError::NoDevice)) => eprintln!("skipping: {e:?}"),
            Err(e) => panic!("vulkan device creation failed: {e:?}"),
        }
    }
}
