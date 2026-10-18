//! Pipelines built off the frame: worker threads run each pipeline's build while draws look it up by key, and `Disk`
//! keeps the translated SPIR-V and the device's VkPipelineCache per title so the next run starts warm.
use super::sm86::ResourceKind;
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
    Ready(&'p mut P),
    Pending,
}

type Built<K, P> = (K, Result<P, VkError>);

pub struct PipelineBuilder<'a, K, P> {
    state: &'a VkState,
    pub disk: Option<Disk>,
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

    fn lookup(&mut self, key: &K) -> Result<Lookup<'_, P>, VkError> {
        match self.pipelines.get_mut(key) {
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
        stages: &[vk::PipelineShaderStageCreateInfo],
        keys: [u64; 4],
    ) -> Result<vk::Pipeline, VkError> {
        use vk::GraphicsPipelineLibraryFlagsEXT as Part;
        let parts = [
            (Part::VERTEX_INPUT_INTERFACE, &stages[..0]),
            (Part::PRE_RASTERIZATION_SHADERS, &stages[..stages.len() - 1]),
            (Part::FRAGMENT_SHADER, &stages[stages.len() - 1..]),
            (Part::FRAGMENT_OUTPUT_INTERFACE, &stages[..0]),
        ];
        let mut libraries = [vk::Pipeline::null(); 4];
        for ((library, (part, stages)), key) in libraries.iter_mut().zip(parts).zip(keys) {
            let key = (part, key);
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
        Ok(unsafe { self.device.create_graphics_pipelines(*self.pipeline_cache.read().unwrap(), &[info], None) }.map_err(|(_, e)| e)?[0])
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

    /// The module, its resources and whether it stores global memory, as this translator build saved them for the guest
    /// code with `code_hash`.
    pub fn spirv(&self, code_hash: u64) -> Option<(Vec<u32>, Vec<ResourceKind>, bool)> {
        let bytes = fs::read(self.spirv_path(code_hash)).ok()?;
        let words: Vec<u32> = bytes.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        let (&[count, stores_global], rest) = words.split_first_chunk()?;
        let (resources, spirv) = rest.split_at_checked(4 * count as usize)?;
        let resources = resources.chunks_exact(4).map(|w| decode(w.try_into().unwrap())).collect::<Option<_>>()?;
        Some((spirv.to_vec(), resources, stores_global != 0))
    }

    /// Saves the resource count, whether the module stores global memory, each resource as four words, then the module.
    pub fn store_spirv(&self, code_hash: u64, words: &[u32], resources: &[ResourceKind], stores_global: bool) -> io::Result<()> {
        let header = [resources.len() as u32, stores_global.into()].into_iter().chain(resources.iter().flat_map(|&kind| encode(kind)));
        write_atomically(&self.spirv_path(code_hash), &header.chain(words.iter().copied()).flat_map(u32::to_le_bytes).collect::<Vec<_>>())
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
            let merged = d.merge_pipeline_caches(*state.pipeline_cache.write().unwrap(), &[saved]);
            d.destroy_pipeline_cache(saved, None);
            Ok(merged?)
        }
    }

    pub fn flush(&self, state: &VkState) -> io::Result<()> {
        let data = unsafe { state.device.get_pipeline_cache_data(*state.pipeline_cache.read().unwrap()) }.map_err(io::Error::other)?;
        write_atomically(&self.dir.join("pipeline-cache"), &data)
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let partial = path.with_extension("partial");
    fs::write(&partial, bytes)?;
    fs::rename(partial, path)
}

fn encode(kind: ResourceKind) -> [u32; 4] {
    use ResourceKind::*;
    match kind {
        BuiltIn(n) => [0, n, 0, 0],
        Cbuf(n) => [1, n, 0, 0],
        Ssbo(n) => [2, n, 0, 0],
        Support => [3, 0, 0, 0],
        Global => [4, 0, 0, 0],
        Local => [5, 0, 0, 0],
        Shared => [6, 0, 0, 0],
        Shared64 => [7, 0, 0, 0],
        Warp => [8, 0, 0, 0],
        Schedule => [9, 0, 0, 0],
        Global64 => [10, 0, 0, 0],
        Texture2D { cbuf_bank, byte_offset } => [11, cbuf_bank, byte_offset, 0],
        Surface2D { cbuf_bank, byte_offset, format } => [12, cbuf_bank, byte_offset, format.as_raw() as u32],
        VertexInput { location } => [13, location, 0, 0],
        Varying { location, flat } => [14, location, flat.into(), 0],
        FragmentOutput { location } => [15, location, 0, 0],
        Mma => [16, 0, 0, 0],
        Hints => [17, 0, 0, 0],
    }
}

fn decode([tag, a, b, c]: [u32; 4]) -> Option<ResourceKind> {
    use ResourceKind::*;
    Some(match tag {
        0 => BuiltIn(a),
        1 => Cbuf(a),
        2 => Ssbo(a),
        3 => Support,
        4 => Global,
        5 => Local,
        6 => Shared,
        7 => Shared64,
        8 => Warp,
        9 => Schedule,
        10 => Global64,
        11 => Texture2D { cbuf_bank: a, byte_offset: b },
        12 => Surface2D { cbuf_bank: a, byte_offset: b, format: vk::Format::from_raw(c as i32) },
        13 => VertexInput { location: a },
        14 => Varying { location: a, flat: b != 0 },
        15 => FragmentOutput { location: a },
        16 => Mma,
        17 => Hints,
        _ => return None,
    })
}
