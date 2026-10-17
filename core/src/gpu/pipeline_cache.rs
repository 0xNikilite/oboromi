//! Pipelines built off the frame: worker threads run each pipeline's build while draws look it up by key, and `Disk`
//! keeps the translated SPIR-V and the device's VkPipelineCache per title so the next run starts warm.
use super::{VkError, VkState};
use ash::vk;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::{fs, io, thread};

pub enum PipelinePolicy {
    /// A lookup blocks until its pipeline is built, so every draw lands.
    Wait,
    /// A lookup returns `Pending` while its pipeline builds, so the caller skips that draw.
    SkipDraw,
}

pub enum Lookup<'p, P> {
    Ready(&'p P),
    Pending,
}

type Built<K, P> = (K, Result<P, VkError>);

pub struct PipelineBuilder<'a, K, P> {
    state: &'a VkState,
    disk: Option<Disk>,
    pub policy: PipelinePolicy,
    /// `None` while the pipeline builds.
    pipelines: HashMap<K, Option<Result<P, VkError>>>,
    jobs: mpsc::Sender<Box<dyn FnOnce() -> Built<K, P> + Send>>,
    done: mpsc::Receiver<Built<K, P>>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl<'a, K: Hash + Eq + Clone + Send + 'static, P: Send + 'static> PipelineBuilder<'a, K, P> {
    /// Starts from the pipeline cache `disk` saved, and saves the device's back to it on `flush` and drop.
    pub fn new(state: &'a VkState, policy: PipelinePolicy, disk: Option<Disk>) -> Result<Self, VkError> {
        if let Some(disk) = &disk {
            disk.load(state)?;
        }
        let (jobs, queue) = mpsc::channel::<Box<dyn FnOnce() -> Built<K, P> + Send>>();
        let (finished, done) = mpsc::channel();
        let queue = Arc::new(Mutex::new(queue));
        let count = thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1));
        let workers = (0..count)
            .map(|_| {
                let (queue, finished) = (queue.clone(), finished.clone());
                thread::spawn(move || loop {
                    let Ok(job) = queue.lock().unwrap().recv() else { break };
                    if finished.send(job()).is_err() {
                        break;
                    }
                })
            })
            .collect();
        Ok(Self { state, disk, policy, pipelines: HashMap::new(), jobs, done, workers })
    }

    /// `describe` runs only on a miss, and the build it returns runs on a worker thread.
    pub fn get_or_request<B>(&mut self, key: &K, describe: impl FnOnce() -> B) -> Result<Lookup<'_, P>, VkError>
    where
        B: FnOnce() -> Result<P, VkError> + Send + 'static,
    {
        if !matches!(self.pipelines.get(key), Some(Some(_))) {
            if !self.pipelines.contains_key(key) {
                let (job_key, build) = (key.clone(), describe());
                self.pipelines.insert(key.clone(), None);
                let _ = self.jobs.send(Box::new(move || (job_key, build())));
            }
            self.collect(matches!(self.policy, PipelinePolicy::Wait).then_some(key));
        }
        self.lookup(key)
    }

    pub fn wait(&mut self, key: &K) -> Result<Lookup<'_, P>, VkError> {
        self.collect(Some(key));
        self.lookup(key)
    }

    fn collect(&mut self, wait_for: Option<&K>) {
        loop {
            let waiting = wait_for.is_some_and(|key| matches!(self.pipelines.get(key), Some(None)));
            let Some((key, built)) = (if waiting { self.done.recv().ok() } else { self.done.try_recv().ok() }) else { return };
            self.pipelines.insert(key, Some(built));
        }
    }

    fn lookup(&self, key: &K) -> Result<Lookup<'_, P>, VkError> {
        match self.pipelines.get(key) {
            Some(Some(Ok(pipeline))) => Ok(Lookup::Ready(pipeline)),
            Some(Some(Err(e))) => Err(*e),
            _ => Ok(Lookup::Pending),
        }
    }
}

impl<K, P> PipelineBuilder<'_, K, P> {
    pub fn flush(&self) -> io::Result<()> {
        self.disk.as_ref().map_or(Ok(()), |disk| disk.flush(self.state))
    }
}

impl<K, P> Drop for PipelineBuilder<'_, K, P> {
    fn drop(&mut self) {
        self.jobs = mpsc::channel().0;
        self.workers.drain(..).for_each(|worker| drop(worker.join()));
        let _ = self.flush();
    }
}

pub(super) fn hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl VkState {
    /// Links the graphics pipeline `info` describes from its four libraries, each created on the first use of its key.
    pub(super) fn link(
        &self,
        info: vk::GraphicsPipelineCreateInfo,
        stages: &[vk::PipelineShaderStageCreateInfo; 2],
        keys: [u64; 4],
    ) -> Result<vk::Pipeline, VkError> {
        use vk::GraphicsPipelineLibraryFlagsEXT as Part;
        let parts = [
            (Part::VERTEX_INPUT_INTERFACE, &stages[..0]),
            (Part::PRE_RASTERIZATION_SHADERS, &stages[..1]),
            (Part::FRAGMENT_SHADER, &stages[1..]),
            (Part::FRAGMENT_OUTPUT_INTERFACE, &stages[..0]),
        ];
        let mut libraries = [vk::Pipeline::null(); 4];
        for ((library, (part, stages)), key) in libraries.iter_mut().zip(parts).zip(keys) {
            let key = (part.as_raw() as i32, key);
            let found = self.libraries.lock().unwrap().get(&key).copied();
            *library = match found {
                Some(library) => library,
                None => {
                    let mut subset = vk::GraphicsPipelineLibraryCreateInfoEXT::default().flags(part);
                    let info = info.flags(vk::PipelineCreateFlags::LIBRARY_KHR).stages(stages).push_next(&mut subset);
                    let created = self.create_graphics_pipeline(info)?;
                    let library = *self.libraries.lock().unwrap().entry(key).or_insert(created);
                    if library != created {
                        unsafe { self.device.destroy_pipeline(created, None) };
                    }
                    library
                }
            };
        }
        let mut linked = vk::PipelineLibraryCreateInfoKHR::default().libraries(&libraries);
        self.create_graphics_pipeline(vk::GraphicsPipelineCreateInfo::default().layout(info.layout).push_next(&mut linked))
    }

    pub(super) fn create_graphics_pipeline(&self, info: vk::GraphicsPipelineCreateInfo) -> Result<vk::Pipeline, VkError> {
        Ok(unsafe { self.device.create_graphics_pipelines(self.pipeline_cache, &[info], None) }.map_err(|(_, e)| e)?[0])
    }
}

pub struct Disk {
    dir: PathBuf,
    /// headerVersion, vendorID, deviceID and pipelineCacheUUID of the VkPipelineCacheHeaderVersionOne this device writes.
    header: Vec<u8>,
}

impl Disk {
    /// Everything lives under `root/title/<pipelineCacheUUID>`, since the SPIR-V also depends on the device's features.
    pub fn open(root: &Path, title: &str, device: &vk::PhysicalDeviceProperties) -> io::Result<Self> {
        let uuid: String = device.pipeline_cache_uuid.iter().map(|b| format!("{b:02x}")).collect();
        let dir = root.join(title).join(uuid);
        fs::create_dir_all(&dir)?;
        let words = [vk::PipelineCacheHeaderVersion::ONE.as_raw() as u32, device.vendor_id, device.device_id];
        let header = words.iter().flat_map(|w| w.to_ne_bytes()).chain(device.pipeline_cache_uuid).collect();
        Ok(Self { dir, header })
    }

    fn spirv_path(&self, code_hash: u64) -> PathBuf {
        self.dir.join(format!("{code_hash:016x}-{}.spv", env!("TRANSLATOR_HASH")))
    }

    /// The module this translator build saved for the guest code with `code_hash`.
    pub fn spirv(&self, code_hash: u64) -> Option<Vec<u32>> {
        let bytes = fs::read(self.spirv_path(code_hash)).ok()?;
        Some(bytes.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect())
    }

    pub fn store_spirv(&self, code_hash: u64, words: &[u32]) -> io::Result<()> {
        write_atomically(&self.spirv_path(code_hash), &words.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>())
    }

    /// The saved VkPipelineCache data, unless its header names another device or driver build.
    pub fn pipeline_cache_data(&self) -> Option<Vec<u8>> {
        let data = fs::read(self.dir.join("pipeline-cache")).ok()?;
        (data.get(4..32) == Some(&self.header[..])).then_some(data)
    }

    pub fn load(&self, state: &VkState) -> Result<(), VkError> {
        let Some(data) = self.pipeline_cache_data() else { return Ok(()) };
        let d = &state.device;
        unsafe {
            let saved = d.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default().initial_data(&data), None)?;
            let merged = d.merge_pipeline_caches(state.pipeline_cache, &[saved]);
            d.destroy_pipeline_cache(saved, None);
            Ok(merged?)
        }
    }

    pub fn flush(&self, state: &VkState) -> io::Result<()> {
        let data = unsafe { state.device.get_pipeline_cache_data(state.pipeline_cache) }.map_err(io::Error::other)?;
        write_atomically(&self.dir.join("pipeline-cache"), &data)
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let partial = path.with_extension("partial");
    fs::write(&partial, bytes)?;
    fs::rename(partial, path)
}
