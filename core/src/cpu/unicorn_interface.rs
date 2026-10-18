use super::{Cpu, CpuContext, CpuError, GuestRam};
use crate::gpu::memory::{PAGE, pages};
use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use unicorn_engine::{Arch, Arm64Insn, Context, HookType, Mode, Prot, Query, RegisterARM64, Unicorn, uc_error};

const EXCP_SWI: u32 = 2;
const INSTRUCTIONS_PER_BLOCK: u64 = 64;

pub use super::TICK_HZ;
const CLREX: u64 = 0xffff_0000_0000;

struct Clock {
    epoch: Instant,
    virtual_time: Cell<bool>,
    block_ticks: Cell<u64>,
    ticks: Cell<u64>,
    core: Cell<u32>,
}

impl Clock {
    fn now(&self) -> u64 {
        if self.virtual_time.get() { self.ticks.get() } else { (self.epoch.elapsed().as_nanos() * u128::from(TICK_HZ) / 1_000_000_000) as u64 }
    }
}

struct RunState {
    svc: Option<u32>,
    clock: Rc<Clock>,
    dirty_pages: Vec<u64>,
}

type Emu = Unicorn<'static, RunState>;

struct Engine {
    emu: Emu,
    initial: Context,
    views: Vec<Context>,
    active: usize,
    hooked: usize,
}

// All Unicorn handles and their internal Rc state remain behind the engine mutex.
unsafe impl Send for Engine {}

impl Deref for Engine {
    type Target = Emu;
    fn deref(&self) -> &Emu { &self.emu }
}
impl DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut Emu { &mut self.emu }
}

impl Engine {
    fn clear_exclusive(&mut self) -> Result<(), uc_error> {
        // Unicorn contexts include the software monitor; CLREX prevents ABA success after a handoff.
        let pc = self.pc_read()?;
        let result = self.emu_start(CLREX, CLREX + 4, 0, 0);
        self.reg_write(RegisterARM64::PC, pc)?;
        result
    }
}

/// Logical CPU view. All views owned by a `CpuManager` share one serialized engine.
pub struct UnicornCPU {
    emu: Arc<Mutex<Engine>>,
    ram: Arc<GuestRam>,
    pub core_id: u32,
    view: usize,
}

/// The interrupt hook runs under the core's lock, so it only records the SVC and stops;
/// every other exception (BRK, faults) just stops the core.
fn new_emu() -> Result<Emu, uc_error> {
    let mut emu = Unicorn::new_with_data(Arch::ARM64, Mode::LITTLE_ENDIAN, RunState { svc: None, clock: Rc::new(Clock { epoch: Instant::now(), virtual_time: Cell::new(false), block_ticks: Cell::new(192), ticks: Cell::new(0), core: Cell::new(0) }), dirty_pages: Vec::new() })?;
    emu.add_intr_hook(|uc, intno| {
        if intno == EXCP_SWI {
            // QEMU raises the SVC with PC already past it (translate-a64.c, gen_exception_insn with pc_next).
            let mut insn = [0; 4];
            let pc = uc.pc_read().unwrap_or(0);
            if uc.mem_read(pc.wrapping_sub(4), &mut insn).is_ok() {
                uc.get_data_mut().svc = Some((u32::from_le_bytes(insn) >> 5) & 0xffff);
            }
        }
        let _ = uc.emu_stop();
    })?;
    let mut hook = 0;
    let data = Rc::as_ptr(&emu.get_data().clock).cast_mut().cast();
    // 2.1.5's Rust proxy returns bool, but uc_cb_insn_sys_t returns uint32_t.
    // The clock's Rc owns this stable allocation until uc_close removes the hook.
    unsafe { unicorn_engine::uc_hook_add(emu.get_handle(), &raw mut hook, HookType::INSN.0.cast_signed(),
        mrs as *const () as _, data, 1u64, 0u64, Arm64Insn::UC_ARM64_INS_MRS as i32) }.and(Ok(()))?;
    emu.mem_map(CLREX, 0x1000, Prot::READ | Prot::EXEC)?;
    emu.mem_write(CLREX, &0xd503_3f5fu32.to_le_bytes())?;
    Ok(emu)
}

unsafe extern "C" fn mrs(uc: *mut unicorn_engine::uc_engine, reg: RegisterARM64,
    cp: *const unicorn_engine::RegisterARM64CP, data: *mut std::ffi::c_void) -> u32 {
    // Unicorn calls synchronously with valid register metadata and our live clock.
    let (cp, data) = unsafe { (&*cp, &*data.cast::<Clock>()) };
    let value = match (cp.op0, cp.op1, cp.crn, cp.crm, cp.op2) {
        (3, 3, 14, 0, 0) => TICK_HZ,
        (3, 3, 14, 0, 1 | 2) => data.now(),
        (3, 0, 0, 0, 5) => 0x8000_0000 | u64::from(data.core.get()),
        _ => return 0,
    };
    // The selected register is a 64-bit X register, including XZR.
    unsafe { unicorn_engine::uc_reg_write(uc, reg as i32, std::ptr::from_ref(&value).cast()) };
    1
}

impl UnicornCPU {
    /// Create a new Unicorn instance with 8MB of memory (Legacy/Test mode)
    pub fn new() -> Option<Self> {
        Self::new_with_shared_mem(0, GuestRam::new(8 * 1024 * 1024))
    }

    /// Create a new Unicorn instance mapping `ram` at guest address 0
    pub fn new_with_shared_mem(core_id: u32, ram: Arc<GuestRam>) -> Option<Self> {
        let mut emu = new_emu()
            .map_err(|e| {
                eprintln!("Failed to create Unicorn instance for core {}: {:?}", core_id, e);
                e
            })
            .ok()?;

        // Safety: the core holds `ram`, so the mapping outlives the Unicorn instance.
        unsafe {
            emu.mem_map_ptr(0x0, ram.size() as u64, Prot::ALL, ram.base().cast())
                .map_err(|e| {
                    eprintln!("Failed to map shared memory for core {}: {:?}", core_id, e);
                    e
                })
                .ok()?;
        }

        // Initialize stack pointer to end of memory, offset by core ID to avoid collision
        // Give each core 1MB of stack space at the top of memory
        let stack_top = ram.size() as u64 - (core_id as u64 * 0x100000);
        let _ = emu.reg_write(RegisterARM64::SP, stack_top);

        let initial = emu.context_init().ok()?;
        let views = vec![emu.context_init().ok()?];
        Some(Self { emu: Arc::new(Mutex::new(Engine { emu, initial, views, active: 0, hooked: 0 })), ram, core_id, view: 0 })
    }

    /// Locks the core and takes the RAM turn, first hooking writes to the ranges the GPU mapped since the last lock.
    fn lock(&self) -> (MutexGuard<'_, Engine>, MutexGuard<'_, ()>) {
        let mut emu = self.emu.lock().unwrap();
        let turn = self.ram.turn();
        if emu.active != self.view {
            emu.clear_exclusive().expect("CLREX");
            let Engine { emu: uc, views, active, .. } = &mut *emu;
            uc.context_save(&mut views[*active]).expect("save core");
            *active = self.view;
            uc.context_restore(&views[*active]).expect("restore core");
            uc.get_data().clock.core.set(self.core_id);
        }
        let watched = self.ram.watched();
        // One hook per GPU mapping: Unicorn tests every guest store against each hook's range, and a hit only sets page bits.
        for range in &watched[emu.hooked..] {
            let mark = |uc: &mut Unicorn<RunState>, _, pa, len: usize, _| {
                let dirty = &mut uc.get_data_mut().dirty_pages;
                for page in pages(pa, len as u64) {
                    if dirty.last() != Some(&page) { dirty.push(page); }
                }
                true
            };
            emu.add_mem_hook(HookType::MEM_WRITE, range.start, range.end - 1, mark).expect("write hook");
        }
        emu.hooked = watched.len();
        drop(watched);
        (emu, turn)
    }

    pub(super) fn logical_view(&self, core_id: u32) -> Self {
        let mut engine = self.emu.lock().unwrap();
        let mut context = engine.context_init().expect("core context");
        context.reg_write(RegisterARM64::SP, self.ram.size() as u64 - u64::from(core_id) * 0x100000).expect("core stack");
        engine.views.push(context);
        Self { emu: Arc::clone(&self.emu), ram: Arc::clone(&self.ram), core_id, view: core_id as usize }
    }

    #[cfg(test)]
    pub(crate) fn benchmark_cache(&self, start: u64, end: u64) {
        let mut emu = self.lock().0;
        emu.ctl_remove_cache(start, end).unwrap();
    }

    pub(crate) fn set_block_ticks(&self, ticks: u64) {
        self.lock().0.get_data().clock.block_ticks.set(ticks);
    }

    pub(crate) fn set_virtual_time(&self, enabled: bool) {
        self.lock().0.get_data().clock.virtual_time.set(enabled);
    }

    pub fn ticks(&self) -> u64 { self.lock().0.get_data().clock.now() }

    pub(crate) fn idle_until(&self, deadline: u64) {
        let mut emu = self.lock().0;
        let data = emu.get_data_mut();
        if data.clock.virtual_time.get() {
            data.clock.ticks.set(data.clock.ticks.get().max(deadline));
        } else {
            let ticks = deadline.saturating_sub(data.clock.now());
            drop(emu);
            std::thread::sleep(Duration::from_nanos((u128::from(ticks) * 1_000_000_000 / u128::from(TICK_HZ)) as u64));
        }
    }

    pub(crate) fn run_slice(&self, deadline: u64, core: usize) -> Result<bool, uc_error> {
        let (mut emu, _turn) = self.lock();
        let data = emu.get_data_mut();
        data.svc = None;
        data.clock.core.set(core as u32);
        let virtual_time = data.clock.virtual_time.get();
        let remaining = deadline.saturating_sub(data.clock.now());
        if remaining == 0 { return Ok(true); }
        let block_ticks = data.clock.block_ticks.get().max(1);
        let count = if virtual_time { remaining.div_ceil(block_ticks).saturating_mul(INSTRUCTIONS_PER_BLOCK).min(usize::MAX as u64) as usize } else { 0 };
        let timeout = if virtual_time { 0 } else { (u128::from(remaining) * 1_000_000).div_ceil(u128::from(TICK_HZ)).min(u128::from(u64::MAX)) as u64 };
        let pc = emu.pc_read()?;
        let result = emu.emu_start(pc, u64::MAX, timeout, count);
        self.flush_dirty(&mut emu);
        result?;
        let sliced = if virtual_time { emu.get_data().svc.is_none() } else { emu.query(Query::TIMEOUT)? != 0 };
        if virtual_time {
            let elapsed = if sliced { remaining } else { block_ticks };
            let clock = &emu.get_data().clock;
            clock.ticks.set(clock.ticks.get().saturating_add(elapsed));
        }
        Ok(sliced)
    }

    fn flush_dirty(&self, emu: &mut Engine) {
        let dirty = &mut emu.get_data_mut().dirty_pages;
        dirty.sort_unstable();
        dirty.dedup();
        for page in dirty.drain(..) { self.ram.mark_cpu_written(page * PAGE, PAGE); }
    }

    pub(crate) fn context(&self) -> Result<Context, uc_error> { self.lock().0.context_init() }

    pub(crate) fn save(&self, context: &mut Context) -> Result<(), uc_error> {
        let (mut emu, _turn) = self.lock();
        emu.clear_exclusive()?;
        emu.context_save(context)
    }

    pub(crate) fn restore(&self, context: &Context) -> Result<(), uc_error> {
        self.lock().0.context_restore(context)
    }

    pub(crate) fn thread_context(&self, entry: u64, arg: u64, stack: u64, tls: u64, handle: u32) -> Result<Context, uc_error> {
        let (emu, _turn) = self.lock();
        let previous = emu.context_init()?;
        emu.context_restore(&emu.initial)?;
        let mut context = emu.context_init()?;
        emu.context_restore(&previous)?;
        for (reg, value) in [(RegisterARM64::PC, entry), (RegisterARM64::SP, stack), (RegisterARM64::X0, arg), (RegisterARM64::X1, u64::from(handle)), (RegisterARM64::TPIDRRO_EL0, tls)] {
            context.reg_write(reg, value)?;
        }
        Ok(context)
    }

    pub(crate) fn tls(&self) -> u64 { self.lock().0.reg_read(RegisterARM64::TPIDRRO_EL0).unwrap_or(0) }

    /// Run the core until halt or breakpoint
    pub fn run(&self) -> u64 {
        let (mut emu, _turn) = self.lock();
        let pc = emu.reg_read(RegisterARM64::PC).unwrap_or(0);

        // Run until we hit a BRK instruction or error
        let result = emu.emu_start(pc, 0xFFFF_FFFF_FFFF_FFFF, 0, 0);
        self.flush_dirty(&mut emu);
        match result {
            Ok(_) => 1, // Success - normal completion
            Err(e) => {
                // BRK instruction causes an error, which is expected
                if format!("{e:?}").contains("EXCEPTION") {
                    1 // Success - terminated by BRK
                } else {
                    eprintln!("Emulation error: {e:?}");
                    0 // Failure - actual emulation error
                }
            }
        }
    }

    /// Execute a single step
    pub fn step(&self) -> u64 {
        let (mut emu, _turn) = self.lock();
        let pc = emu.reg_read(RegisterARM64::PC).unwrap_or(0);

        let result = emu.emu_start(pc, pc + 4, 0, 1);
        self.flush_dirty(&mut emu);
        match result {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }

    /// Halt execution
    pub fn halt(&self) {
        let mut emu = self.lock().0;
        let _ = emu.emu_stop();
    }

    /// Read register Xn (0-30)
    pub fn get_x(&self, reg_index: u32) -> u64 {
        let emu = self.lock().0;
        if reg_index > 30 {
            return 0;
        }

        let reg = match reg_index {
            0 => RegisterARM64::X0,
            1 => RegisterARM64::X1,
            2 => RegisterARM64::X2,
            3 => RegisterARM64::X3,
            4 => RegisterARM64::X4,
            5 => RegisterARM64::X5,
            6 => RegisterARM64::X6,
            7 => RegisterARM64::X7,
            8 => RegisterARM64::X8,
            9 => RegisterARM64::X9,
            10 => RegisterARM64::X10,
            11 => RegisterARM64::X11,
            12 => RegisterARM64::X12,
            13 => RegisterARM64::X13,
            14 => RegisterARM64::X14,
            15 => RegisterARM64::X15,
            16 => RegisterARM64::X16,
            17 => RegisterARM64::X17,
            18 => RegisterARM64::X18,
            19 => RegisterARM64::X19,
            20 => RegisterARM64::X20,
            21 => RegisterARM64::X21,
            22 => RegisterARM64::X22,
            23 => RegisterARM64::X23,
            24 => RegisterARM64::X24,
            25 => RegisterARM64::X25,
            26 => RegisterARM64::X26,
            27 => RegisterARM64::X27,
            28 => RegisterARM64::X28,
            29 => RegisterARM64::X29,
            30 => RegisterARM64::X30,
            _ => return 0,
        };

        emu.reg_read(reg).unwrap_or(0)
    }

    /// Write register Xn
    pub fn set_x(&self, reg_index: u32, value: u64) {
        let mut emu = self.lock().0;
        if reg_index > 30 {
            return;
        }

        let reg = match reg_index {
            0 => RegisterARM64::X0,
            1 => RegisterARM64::X1,
            2 => RegisterARM64::X2,
            3 => RegisterARM64::X3,
            4 => RegisterARM64::X4,
            5 => RegisterARM64::X5,
            6 => RegisterARM64::X6,
            7 => RegisterARM64::X7,
            8 => RegisterARM64::X8,
            9 => RegisterARM64::X9,
            10 => RegisterARM64::X10,
            11 => RegisterARM64::X11,
            12 => RegisterARM64::X12,
            13 => RegisterARM64::X13,
            14 => RegisterARM64::X14,
            15 => RegisterARM64::X15,
            16 => RegisterARM64::X16,
            17 => RegisterARM64::X17,
            18 => RegisterARM64::X18,
            19 => RegisterARM64::X19,
            20 => RegisterARM64::X20,
            21 => RegisterARM64::X21,
            22 => RegisterARM64::X22,
            23 => RegisterARM64::X23,
            24 => RegisterARM64::X24,
            25 => RegisterARM64::X25,
            26 => RegisterARM64::X26,
            27 => RegisterARM64::X27,
            28 => RegisterARM64::X28,
            29 => RegisterARM64::X29,
            30 => RegisterARM64::X30,
            _ => return,
        };

        let _ = emu.reg_write(reg, value);
    }

    /// Read SP
    pub fn get_sp(&self) -> u64 {
        let emu = self.lock().0;
        emu.reg_read(RegisterARM64::SP).unwrap_or(0)
    }

    /// Write SP
    pub fn set_sp(&self, value: u64) {
        let mut emu = self.lock().0;
        let _ = emu.reg_write(RegisterARM64::SP, value);
    }

    /// Read PC
    pub fn get_pc(&self) -> u64 {
        let emu = self.lock().0;
        emu.reg_read(RegisterARM64::PC).unwrap_or(0)
    }

    /// Write PC
    pub fn set_pc(&self, value: u64) {
        let mut emu = self.lock().0;
        let _ = emu.reg_write(RegisterARM64::PC, value);
    }

    /// Take the immediate of the SVC that stopped the last run
    pub fn svc_pending(&self) -> Option<u32> {
        self.lock().0.get_data_mut().svc.take()
    }

    /// Write TPIDRRO_EL0, the thread-local region pointer user code reads
    pub fn set_tls(&self, value: u64) {
        let _ = self.lock().0.reg_write(RegisterARM64::TPIDRRO_EL0, value);
    }

    pub fn read_mem(&self, vaddr: u64, buf: &mut [u8]) -> Result<(), uc_error> {
        if vaddr.checked_add(buf.len() as u64).is_none_or(|end| end > self.ram.size() as u64) { return Err(uc_error::READ_UNMAPPED); }
        self.lock().0.mem_read(vaddr, buf)
    }

    pub fn write_mem(&self, vaddr: u64, bytes: &[u8]) -> Result<(), uc_error> {
        if vaddr.checked_add(bytes.len() as u64).is_none_or(|end| end > self.ram.size() as u64) { return Err(uc_error::WRITE_UNMAPPED); }
        let (mut emu, _turn) = self.lock();
        emu.mem_write(vaddr, bytes)?;
        self.ram.mark_cpu_written(vaddr, bytes.len() as u64);
        Ok(())
    }

    /// Write a 32-bit value to emulated memory
    pub fn write_u32(&self, vaddr: u64, value: u32) {
        let _ = self.write_mem(vaddr, &value.to_le_bytes());
    }

    /// Read a 32-bit value from emulated memory
    pub fn read_u32(&self, vaddr: u64) -> u32 {
        let mut bytes = [0u8; 4];
        if self.read_mem(vaddr, &mut bytes).is_ok() {
            u32::from_le_bytes(bytes)
        } else {
            0
        }
    }

    /// Write a 64-bit value to emulated memory
    pub fn write_u64(&self, vaddr: u64, value: u64) {
        let _ = self.write_mem(vaddr, &value.to_le_bytes());
    }

    /// Read a 64-bit value from emulated memory
    pub fn read_u64(&self, vaddr: u64) -> u64 {
        let mut bytes = [0u8; 8];
        if self.read_mem(vaddr, &mut bytes).is_ok() {
            u64::from_le_bytes(bytes)
        } else {
            0
        }
    }
}

impl Cpu for UnicornCPU {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn run(&self) -> u64 { UnicornCPU::run(self) }
    fn step(&self) -> u64 { UnicornCPU::step(self) }
    fn halt(&self) { UnicornCPU::halt(self) }
    fn get_x(&self, index: u32) -> u64 { UnicornCPU::get_x(self, index) }
    fn set_x(&self, index: u32, value: u64) { UnicornCPU::set_x(self, index, value) }
    fn get_sp(&self) -> u64 { UnicornCPU::get_sp(self) }
    fn set_sp(&self, value: u64) { UnicornCPU::set_sp(self, value) }
    fn get_pc(&self) -> u64 { UnicornCPU::get_pc(self) }
    fn set_pc(&self, value: u64) { UnicornCPU::set_pc(self, value) }
    fn svc_pending(&self) -> Option<u32> { UnicornCPU::svc_pending(self) }
    fn set_tls(&self, value: u64) { UnicornCPU::set_tls(self, value) }
    fn tls(&self) -> u64 { UnicornCPU::tls(self) }
    fn read_mem(&self, address: u64, bytes: &mut [u8]) -> Result<(), CpuError> { UnicornCPU::read_mem(self, address, bytes).map_err(|_| CpuError) }
    fn write_mem(&self, address: u64, bytes: &[u8]) -> Result<(), CpuError> { UnicornCPU::write_mem(self, address, bytes).map_err(|_| CpuError) }
    fn ticks(&self) -> u64 { UnicornCPU::ticks(self) }
    fn idle_until(&self, deadline: u64) { UnicornCPU::idle_until(self, deadline) }
    fn run_slice(&self, deadline: u64, core: usize) -> Result<bool, CpuError> { UnicornCPU::run_slice(self, deadline, core).map_err(|_| CpuError) }
    fn context(&self) -> Result<CpuContext, CpuError> { UnicornCPU::context(self).map(CpuContext::Unicorn).map_err(|_| CpuError) }
    fn save(&self, context: &mut CpuContext) -> Result<(), CpuError> {
        let CpuContext::Unicorn(context) = context else { return Err(CpuError) };
        UnicornCPU::save(self, context).map_err(|_| CpuError)
    }
    fn restore(&self, context: &CpuContext) -> Result<(), CpuError> {
        let CpuContext::Unicorn(context) = context else { return Err(CpuError) };
        UnicornCPU::restore(self, context).map_err(|_| CpuError)
    }
    fn thread_context(&self, entry: u64, arg: u64, stack: u64, tls: u64, handle: u32) -> Result<CpuContext, CpuError> {
        UnicornCPU::thread_context(self, entry, arg, stack, tls, handle).map(CpuContext::Unicorn).map_err(|_| CpuError)
    }
    fn set_virtual_time(&self, enabled: bool) { UnicornCPU::set_virtual_time(self, enabled) }
    fn set_block_ticks(&self, ticks: u64) { UnicornCPU::set_block_ticks(self, ticks) }
}
