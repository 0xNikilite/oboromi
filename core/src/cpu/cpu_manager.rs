//! A64 tasks running in a shared guest address space.

use rapid_arm_emu::address_space::{AddrSpace, MemProt, PAGE_SIZE, Sharing};
use rapid_arm_emu::handler::{
    ContextHandler, Exception, InterruptHandler, TaskCtx, TaskMutator, UserHalt,
};
use rapid_arm_emu::state::{ExecState, PState};
use rapid_arm_emu::{Affinity, CoreId, Cpu, Isa, PausedTask, TaskHandle, TaskOptions, rt};
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    num::NonZero,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

pub const CORE_COUNT: usize = 8;
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024;
pub const MEMORY_BASE: u64 = 0;

pub struct Stop {
    pub exception: Exception,
    pub state: ExecState,
}

type Stops = Arc<Mutex<HashMap<TaskHandle, mpsc::Sender<Stop>>>>;

struct Handler(Stops);

impl InterruptHandler for Handler {
    fn on_exception<'t, 'a: 't>(
        &'a self,
        ctx: TaskCtx<'t>,
        mut task: TaskMutator<'t>,
        exception: Exception,
    ) -> impl ContextHandler + 't {
        async move {
            let sender = self.0.lock().unwrap().remove(ctx.task());
            if let Some(sender) = sender {
                let state = task.with_state(|state| state.clone());
                let _ = sender.send(Stop { exception, state });
            }
            task.exit_task(0).await;
        }
    }
    fn on_core_interrupt_in_task<'t, 'a: 't>(
        &'a self,
        _: CoreId,
        _: TaskCtx<'t>,
        _: TaskMutator<'t>,
        _: UserHalt,
    ) -> impl ContextHandler + 't {
        async {}
    }
    fn on_task_exit(&self, task: &TaskHandle, _: i64) {
        self.0.lock().unwrap().remove(task);
    }

    fn on_idle_core_interrupt(&self, _: CoreId, _: UserHalt) {}
}

pub type BackendError = Box<dyn Error + Send + Sync>;

fn validate_a64(state: &ExecState) -> Result<(), BackendError> {
    if state.pstate.0 & (PState::nRW.0 | PState::T.0) != 0 {
        return Err("expected A64 register state".into());
    }
    Ok(())
}

fn pinned_to(core: usize, count: NonZero<usize>) -> Result<TaskOptions, BackendError> {
    let core = CoreId::new(core)
        .filter(|id| id.index() < count.get())
        .ok_or("core index out of range")?;
    Ok(TaskOptions {
        affinity: Affinity::Pinned(core),
        ..TaskOptions::default()
    })
}

/// A running task. Dropping the handle requests termination.
pub struct RapidTask {
    task: TaskHandle,
    stop: mpsc::Receiver<Stop>,
    stops: Stops,
}

impl RapidTask {
    fn resume_registered(paused: PausedTask, stops: &Stops) -> Self {
        // Register the task before the guest can hit its first breakpoint.
        let (sender, stop) = mpsc::channel();
        stops.lock().unwrap().insert(paused.task().clone(), sender);
        Self {
            task: paused.resume(),
            stop,
            stops: stops.clone(),
        }
    }

    /// Start another thread in this task's guest process.
    pub fn spawn_thread(&self, core: usize, initial: ExecState) -> Result<Self, BackendError> {
        validate_a64(&initial)?;
        let options = pinned_to(core, self.task.cpu().core_count())?;
        let paused = self
            .task
            .process()
            .spawn_task_paused(options, |state| *state = initial)?;
        Ok(Self::resume_registered(paused, &self.stops))
    }

    /// Pause at an instruction boundary and resume after applying the register edits.
    pub fn with_paused_state<R>(
        &self,
        edit: impl FnOnce(&mut ExecState) -> R,
    ) -> Result<R, BackendError> {
        let mut paused = rt::block_on(self.task.pause())?;
        let mut state = paused.state().clone();
        let result = edit(&mut state);
        validate_a64(&state)?;
        *paused.state_mut() = state;
        Ok(result)
    }

    /// Wait for the guest to exit and return its final exception and registers.
    pub fn wait(self) -> Result<Stop, BackendError> {
        rt::block_on(self.task.exit_signal());
        Ok(self.stop.try_recv()?)
    }

    /// Cancel the task if it does not reach an exception within the deadline.
    pub fn wait_timeout(self, timeout: Duration) -> Result<Stop, BackendError> {
        let stop = self.stop.recv_timeout(timeout)?;
        rt::block_on(self.task.exit_signal());
        Ok(stop)
    }
}

impl Drop for RapidTask {
    fn drop(&mut self) {
        self.stops.lock().unwrap().remove(&self.task);
        let _ = self.task.kill(-1);
    }
}

/// One address space shared by all emulated cores.
pub struct CpuManager {
    cpu: Cpu,
    space: AddrSpace,
    stops: Stops,
}

impl CpuManager {
    /// Start the console's CPU cores with an empty guest address space.
    pub fn new() -> Result<Self, BackendError> {
        Self::with_cores(NonZero::new(CORE_COUNT).unwrap())
    }

    pub fn with_cores(cores: NonZero<usize>) -> Result<Self, BackendError> {
        let stops = Arc::new(Mutex::new(HashMap::new()));
        let cpu = Cpu::new(cores, Handler(stops.clone()))?;
        let space = cpu.handle().create_addr_space();
        Ok(Self { cpu, space, stops })
    }

    pub fn map_code(&mut self, base: u64, image: File) -> Result<(), BackendError> {
        let size = image.metadata()?.len();
        if size == 0 || !size.is_multiple_of(PAGE_SIZE) || !base.is_multiple_of(PAGE_SIZE) {
            return Err("code image and mapping must be page aligned and nonempty".into());
        }
        self.space.modify_blocking(|space| {
            space.map_object(base, size, image, false, MemProt::READ | MemProt::EXECUTE)
        })?;
        Ok(())
    }

    /// Map zeroed RAM, allocating backing pages on first write.
    pub fn map_ram(&mut self, base: u64, size: u64) -> Result<(), BackendError> {
        self.space.modify_blocking(|space| {
            space.map_anon(
                base,
                size,
                MemProt::READ | MemProt::WRITE,
                Sharing::PrivateLazy,
            )
        })?;
        Ok(())
    }

    pub fn core_count(&self) -> NonZero<usize> {
        self.cpu.handle().core_count()
    }

    /// Start a task pinned to one core. Other tasks can run in the same address space.
    pub fn start(&self, core: usize, initial: ExecState) -> Result<RapidTask, BackendError> {
        validate_a64(&initial)?;
        let options = pinned_to(core, self.core_count())?;
        let (_, paused) = self.cpu.handle().spawn_process_paused(
            self.space.clone(),
            Isa::A64,
            options,
            |state| *state = initial,
        )?;
        Ok(RapidTask::resume_registered(paused, &self.stops))
    }
}
