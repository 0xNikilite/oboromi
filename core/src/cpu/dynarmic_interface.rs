//! Dynarmic A64 cores share RAM and a global exclusive monitor. Fastmem covers the power-of-two
//! RAM arena; GPU-watched mappings switch execution to callbacks so writes remain observable.
use super::{Cpu, CpuContext, CpuError, GuestRam, TICK_HZ};
use std::{ffi::c_void, ptr::NonNull, sync::{Arc, Mutex}, time::Duration};

#[repr(C)]
#[derive(Clone)]
pub struct DynarmicContext {
    x: [u64; 31],
    v: [u64; 64],
    sp: u64,
    pc: u64,
    tls: u64,
    tpidr: u64,
    daif: u64,
    pstate: u32,
    fpcr: u32,
    fpsr: u32,
}

impl Default for DynarmicContext {
    fn default() -> Self { Self { x: [0; 31], v: [0; 64], sp: 0, pc: 0, tls: 0, tpidr: 0, daif: 0, pstate: 0, fpcr: 0, fpsr: 0 } }
}

enum Monitor {}
enum Engine {}

unsafe extern "C" {
    fn oboromi_dynarmic_monitor_new() -> *mut Monitor;
    fn oboromi_dynarmic_monitor_free(monitor: *mut Monitor);
    fn oboromi_dynarmic_cpu_new(monitor: *mut Monitor, id: usize, memory: *mut u8, size: u64,
        callback: unsafe extern "C" fn(*mut c_void, u64, usize), data: *mut c_void) -> *mut Engine;
    fn oboromi_dynarmic_cpu_free(cpu: *mut Engine);
    fn oboromi_dynarmic_set_watched(cpu: *mut Engine, watched: bool);
    fn oboromi_dynarmic_set_virtual_time(cpu: *mut Engine, enabled: bool);
    fn oboromi_dynarmic_set_tick_scale(cpu: *mut Engine, scale: u64);
    fn oboromi_dynarmic_ticks(cpu: *const Engine) -> u64;
    fn oboromi_dynarmic_idle_until(cpu: *mut Engine, deadline: u64) -> bool;
    fn oboromi_dynarmic_run(cpu: *mut Engine, deadline: u64, core: u32, sliced: *mut bool) -> bool;
    fn oboromi_dynarmic_step(cpu: *mut Engine) -> bool;
    fn oboromi_dynarmic_halt(cpu: *mut Engine);
    fn oboromi_dynarmic_get_x(cpu: *const Engine, index: usize) -> u64;
    fn oboromi_dynarmic_set_x(cpu: *mut Engine, index: usize, value: u64);
    fn oboromi_dynarmic_get_sp(cpu: *const Engine) -> u64;
    fn oboromi_dynarmic_set_sp(cpu: *mut Engine, value: u64);
    fn oboromi_dynarmic_get_pc(cpu: *const Engine) -> u64;
    fn oboromi_dynarmic_set_pc(cpu: *mut Engine, value: u64);
    fn oboromi_dynarmic_set_tls(cpu: *mut Engine, value: u64);
    fn oboromi_dynarmic_take_svc(cpu: *mut Engine) -> u32;
    fn oboromi_dynarmic_clear_exclusive(cpu: *mut Engine);
    fn oboromi_dynarmic_save(cpu: *const Engine, context: *mut DynarmicContext);
    fn oboromi_dynarmic_load(cpu: *mut Engine, context: *const DynarmicContext);
    fn oboromi_dynarmic_invalidate(cpu: *mut Engine, address: u64, size: usize);
}

struct MonitorPtr(NonNull<Monitor>);
unsafe impl Send for MonitorPtr {}
unsafe impl Sync for MonitorPtr {}
impl Drop for MonitorPtr { fn drop(&mut self) { unsafe { oboromi_dynarmic_monitor_free(self.0.as_ptr()) } } }

pub struct DynarmicGroup(Arc<MonitorPtr>);

impl DynarmicGroup {
    pub fn new() -> Option<Self> { NonNull::new(unsafe { oboromi_dynarmic_monitor_new() }).map(MonitorPtr).map(Arc::new).map(Self) }
}

struct EnginePtr(NonNull<Engine>);
unsafe impl Send for EnginePtr {}

pub struct DynarmicCPU {
    engine: Mutex<EnginePtr>,
    ram: Arc<GuestRam>,
    _monitor: Arc<MonitorPtr>,
    callback_data: Box<Arc<GuestRam>>,
}

unsafe extern "C" fn mark_written(data: *mut c_void, address: u64, size: usize) {
    unsafe { &*data.cast::<Arc<GuestRam>>() }.mark_cpu_written(address, size as u64);
}

impl DynarmicCPU {
    pub fn new(group: &DynarmicGroup, core: u32, ram: Arc<GuestRam>) -> Option<Self> {
        let mut callback_data = Box::new(ram.clone());
        let engine = NonNull::new(unsafe { oboromi_dynarmic_cpu_new(group.0.0.as_ptr(), core as usize, ram.base(), ram.size() as u64,
            mark_written, (&mut *callback_data as *mut Arc<GuestRam>).cast()) })?;
        unsafe { oboromi_dynarmic_set_sp(engine.as_ptr(), ram.size() as u64 - u64::from(core) * 0x10_0000) };
        let tick_scale = std::env::var("OBOROMI_DYNARMIC_TICK_SCALE").ok().and_then(|value| value.parse().ok()).unwrap_or(64);
        unsafe { oboromi_dynarmic_set_tick_scale(engine.as_ptr(), tick_scale) };
        Some(Self { engine: Mutex::new(EnginePtr(engine)), ram, _monitor: group.0.clone(), callback_data })
    }

    fn engine(&self) -> std::sync::MutexGuard<'_, EnginePtr> { self.engine.lock().unwrap() }

    fn sync_watched(&self, engine: *mut Engine) {
        unsafe { oboromi_dynarmic_set_watched(engine, !self.ram.watched().is_empty()) };
    }
}

impl Drop for DynarmicCPU {
    fn drop(&mut self) {
        let engine = self.engine.get_mut().unwrap();
        unsafe { oboromi_dynarmic_cpu_free(engine.0.as_ptr()) };
        let _ = &self.callback_data;
    }
}

impl Cpu for DynarmicCPU {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn run(&self) -> u64 { self.run_slice(u64::MAX, 0).map_or(0, |_| 1) }
    fn step(&self) -> u64 { let engine = self.engine(); u64::from(!unsafe { oboromi_dynarmic_step(engine.0.as_ptr()) }) }
    fn halt(&self) { unsafe { oboromi_dynarmic_halt(self.engine().0.as_ptr()) } }
    fn get_x(&self, index: u32) -> u64 { unsafe { oboromi_dynarmic_get_x(self.engine().0.as_ptr(), index as usize) } }
    fn set_x(&self, index: u32, value: u64) { unsafe { oboromi_dynarmic_set_x(self.engine().0.as_ptr(), index as usize, value) } }
    fn get_sp(&self) -> u64 { unsafe { oboromi_dynarmic_get_sp(self.engine().0.as_ptr()) } }
    fn set_sp(&self, value: u64) { unsafe { oboromi_dynarmic_set_sp(self.engine().0.as_ptr(), value) } }
    fn get_pc(&self) -> u64 { unsafe { oboromi_dynarmic_get_pc(self.engine().0.as_ptr()) } }
    fn set_pc(&self, value: u64) { unsafe { oboromi_dynarmic_set_pc(self.engine().0.as_ptr(), value) } }
    fn svc_pending(&self) -> Option<u32> { match unsafe { oboromi_dynarmic_take_svc(self.engine().0.as_ptr()) } { u32::MAX => None, value => Some(value) } }
    fn set_tls(&self, value: u64) { unsafe { oboromi_dynarmic_set_tls(self.engine().0.as_ptr(), value) } }
    fn tls(&self) -> u64 { let mut context = DynarmicContext::default(); unsafe { oboromi_dynarmic_save(self.engine().0.as_ptr(), &raw mut context) }; context.tls }

    fn read_mem(&self, address: u64, bytes: &mut [u8]) -> Result<(), CpuError> {
        if address.checked_add(bytes.len() as u64).is_none_or(|end| end > self.ram.size() as u64) { return Err(CpuError); }
        self.ram.read(address, bytes);
        Ok(())
    }
    fn write_mem(&self, address: u64, bytes: &[u8]) -> Result<(), CpuError> {
        if address.checked_add(bytes.len() as u64).is_none_or(|end| end > self.ram.size() as u64) { return Err(CpuError); }
        let engine = self.engine();
        self.ram.write(address, bytes);
        unsafe { oboromi_dynarmic_invalidate(engine.0.as_ptr(), address, bytes.len()) };
        Ok(())
    }
    fn ticks(&self) -> u64 { unsafe { oboromi_dynarmic_ticks(self.engine().0.as_ptr()) } }
    fn idle_until(&self, deadline: u64) {
        let engine = self.engine();
        let now = unsafe { oboromi_dynarmic_ticks(engine.0.as_ptr()) };
        let virtual_time = unsafe { oboromi_dynarmic_idle_until(engine.0.as_ptr(), deadline) };
        if !virtual_time && now < deadline { std::thread::sleep(Duration::from_nanos(((deadline - now) as u128 * 1_000_000_000 / u128::from(TICK_HZ)) as u64)); }
    }
    fn run_slice(&self, deadline: u64, core: usize) -> Result<bool, CpuError> {
        let engine = self.engine();
        let _turn = self.ram.turn();
        self.sync_watched(engine.0.as_ptr());
        let mut sliced = false;
        unsafe { oboromi_dynarmic_run(engine.0.as_ptr(), deadline, core as u32, &raw mut sliced) }.then_some(sliced).ok_or(CpuError)
    }
    fn context(&self) -> Result<CpuContext, CpuError> { Ok(CpuContext::Dynarmic(DynarmicContext::default())) }
    fn save(&self, context: &mut CpuContext) -> Result<(), CpuError> {
        let CpuContext::Dynarmic(context) = context else { return Err(CpuError) };
        let engine = self.engine();
        unsafe { oboromi_dynarmic_clear_exclusive(engine.0.as_ptr()); oboromi_dynarmic_save(engine.0.as_ptr(), context) };
        Ok(())
    }
    fn restore(&self, context: &CpuContext) -> Result<(), CpuError> {
        let CpuContext::Dynarmic(context) = context else { return Err(CpuError) };
        unsafe { oboromi_dynarmic_load(self.engine().0.as_ptr(), context) };
        Ok(())
    }
    fn thread_context(&self, entry: u64, arg: u64, stack: u64, tls: u64, handle: u32) -> Result<CpuContext, CpuError> {
        let mut context = DynarmicContext::default();
        context.pc = entry; context.sp = stack; context.x[0] = arg; context.x[1] = u64::from(handle); context.tls = tls;
        Ok(CpuContext::Dynarmic(context))
    }
    fn set_virtual_time(&self, enabled: bool) { unsafe { oboromi_dynarmic_set_virtual_time(self.engine().0.as_ptr(), enabled) } }
    fn set_block_ticks(&self, ticks: u64) { unsafe { oboromi_dynarmic_set_tick_scale(self.engine().0.as_ptr(), ticks) } }
}
