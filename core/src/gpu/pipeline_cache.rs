//! Pipelines built off the frame: worker threads run each pipeline's build while draws look it up by key.
use super::VkError;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

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

pub struct PipelineBuilder<K, P> {
    pub policy: PipelinePolicy,
    /// `None` while the pipeline builds.
    pipelines: HashMap<K, Option<Result<P, VkError>>>,
    jobs: mpsc::Sender<Box<dyn FnOnce() -> Built<K, P> + Send>>,
    done: mpsc::Receiver<Built<K, P>>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl<K: Hash + Eq + Clone + Send + 'static, P: Send + 'static> PipelineBuilder<K, P> {
    pub fn new(policy: PipelinePolicy) -> Self {
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
        Self { policy, pipelines: HashMap::new(), jobs, done, workers }
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

impl<K, P> Drop for PipelineBuilder<K, P> {
    fn drop(&mut self) {
        self.jobs = mpsc::channel().0;
        self.workers.drain(..).for_each(|worker| drop(worker.join()));
    }
}
