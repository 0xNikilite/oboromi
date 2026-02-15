use ash::vk;

pub mod sm86;
pub mod spirv;
pub mod test;

pub struct VkState {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}
impl VkState {
    pub fn new() -> Self {
        let s = std::mem::MaybeUninit::<Self>::uninit();
        unsafe { s.assume_init() }
    }
    pub fn init(&mut self) -> ash::prelude::VkResult<()> {
        self.entry = unsafe { ash::Entry::load().unwrap() };
        self.instance = unsafe {
            self.entry.create_instance(&vk::InstanceCreateInfo {
                p_application_info: &vk::ApplicationInfo {
                    api_version: vk::make_api_version(0, 1, 0, 0),
                    ..Default::default()
                },
                ..Default::default()
            }, None)?
        };
        Ok(())
    }
}
impl Default for VkState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct State {
    // pub registers: [u32; 256],
    // pub predicates: u32,
    pub shared_memory: *mut u8,
    pub global_memory: *mut u8,
    pub pc: u64,
    pub vk: VkState,
}

impl State {
    pub fn new() -> Self {
        Self {
            // registers: [0; 256],
            // predicates: 0,
            shared_memory: core::ptr::null_mut(),
            global_memory: core::ptr::null_mut(),
            pc: 0,
            vk: VkState::new(),
        }
    }
    pub fn init(&mut self) -> ash::prelude::VkResult<()> {
        self.vk.init()?;
        Ok(())
    }
}
