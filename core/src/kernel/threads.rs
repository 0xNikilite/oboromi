//! Serialized Horizon threads; waits own their objects until one wake or deadline wins.
// SVC arguments deliberately narrow X registers to Horizon's signed and unsigned W-register types.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
use super::{Error, HandleTable, Kernel, MAIN_STACK, Object, TLS};
use crate::cpu::UnicornCPU;
use crate::cpu::{cpu_manager::CORE_COUNT, unicorn_interface::TICK_HZ};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use unicorn_engine::Context;

const CURRENT_THREAD: u32 = 0xffff_8000;
const LISTENERS: u32 = 0x4000_0000;
const ALL_CORES: u64 = (1 << CORE_COUNT) - 1;

#[derive(Clone)]
enum SyncObject {
    Event(Rc<Cell<bool>>),
    Thread(usize),
}

enum Wait {
    Sleep,
    Sync(Vec<SyncObject>),
    Mutex { owner: usize, addr: u64, tag: u32 },
    Cond { key: u64, addr: u64, tag: u32 },
    Address(u64),
}

enum State {
    Dormant,
    Ready,
    Running,
    Waiting(Wait, Option<u64>),
    Exited,
}

struct Thread {
    context: Context,
    state: State,
    priority: u8,
    effective: u8,
    preferred: i32,
    affinity: u64,
    core: usize,
    cancelled: bool,
    result: Option<(u32, Option<usize>)>,
    order: u64,
    uid: u64,
}

struct Core {
    current: Option<usize>,
    slice_end: u64,
    ready: [VecDeque<usize>; 64],
}

pub struct Timer {
    pub event: Rc<Cell<bool>>,
    pub deadline: u64,
    pub period: u64,
}

pub struct Scheduler {
    threads: Vec<Thread>,
    cores: [Core; CORE_COUNT],
    current: usize,
    next_core: usize,
    order: u64,
    next_uid: u64,
    rotate: bool,
    yield_any: bool,
    pub virtual_time: bool,
    pub timers: Vec<Timer>,
    pub switches: u64,
    #[cfg(test)]
    pub trace: Vec<(usize, u32, u64)>,
    #[cfg(test)]
    pub seed: u64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            threads: vec![],
            cores: std::array::from_fn(|_| Core { current: None, slice_end: 0, ready: std::array::from_fn(|_| VecDeque::new()) }),
            current: 0,
            next_core: 0,
            order: 0,
            next_uid: 1,
            rotate: false,
            yield_any: false,
            virtual_time: false,
            timers: vec![],
            switches: 0,
            #[cfg(test)]
            trace: vec![],
            #[cfg(test)]
            seed: 0,
        }
    }
}

fn ticks(ns: u64) -> u64 {
    (u128::from(ns) * u128::from(TICK_HZ)).div_ceil(1_000_000_000) as u64
}
fn deadline(cpu: &UnicornCPU, ns: i64) -> Option<u64> {
    (ns > 0).then(|| cpu.ticks().saturating_add(ticks(ns as u64)))
}
fn word(cpu: &UnicornCPU, addr: u64) -> Result<u32, Error> {
    if addr & 3 != 0 {
        return Err(Error::InvalidAddress);
    }
    let mut bytes = [0; 4];
    cpu.read_mem(addr, &mut bytes).map_err(|_| Error::InvalidCurrentMemory)?;
    Ok(u32::from_le_bytes(bytes))
}
fn put_word(cpu: &UnicornCPU, addr: u64, value: u32) -> Result<(), Error> {
    cpu.write_mem(addr, &value.to_le_bytes()).map_err(|_| Error::InvalidCurrentMemory)
}

impl Scheduler {
    fn add(&mut self, id: usize, context: Context, priority: u8, core: usize) {
        let thread = Thread {
            context,
            state: State::Dormant,
            priority,
            effective: priority,
            preferred: core as i32,
            affinity: 1 << core,
            core,
            cancelled: false,
            result: None,
            order: 0,
            uid: self.next_uid,
        };
        self.next_uid += 1;
        if id == self.threads.len() {
            self.threads.push(thread);
        } else {
            self.threads[id] = thread;
        }
    }

    fn vacant(&self, handles: &HandleTable<Object>) -> usize {
        self.threads
            .iter()
            .enumerate()
            .find_map(|(id, t)| {
                let retained = handles.objects.values().any(|o| matches!(o, Object::Thread(other) if *other == id))
                    || self.threads.iter().any(|t| match &t.state {
                        State::Waiting(Wait::Sync(objects), _) => {
                            objects.iter().any(|o| matches!(o, SyncObject::Thread(other) if *other == id))
                        }
                        State::Waiting(Wait::Mutex { owner, .. }, _) => *owner == id,
                        _ => false,
                    });
                (matches!(t.state, State::Dormant | State::Exited) && !retained).then_some(id)
            })
            .unwrap_or(self.threads.len())
    }

    fn enqueue(&mut self, id: usize, front: bool) {
        let t = &mut self.threads[id];
        t.state = State::Ready;
        let queue = &mut self.cores[t.core].ready[usize::from(t.effective)];
        if front {
            queue.push_front(id);
        } else {
            queue.push_back(id);
        }
    }

    fn dequeue(&mut self, id: usize) {
        let t = &self.threads[id];
        self.cores[t.core].ready[usize::from(t.effective)].retain(|&i| i != id);
    }

    fn wait(&mut self, kind: Wait, deadline: Option<u64>) {
        self.order += 1;
        let t = &mut self.threads[self.current];
        t.order = self.order;
        t.state = State::Waiting(kind, deadline);
    }

    fn wake(&mut self, id: usize, result: u32, index: Option<usize>) {
        if matches!(self.threads[id].state, State::Waiting(..)) {
            self.threads[id].result = Some((result, index));
            self.enqueue(id, false);
        }
    }

    fn priorities(&mut self) {
        let old: Vec<_> = self.threads.iter().map(|t| t.effective).collect();
        for t in &mut self.threads {
            t.effective = t.priority;
        }
        for _ in 0..self.threads.len() {
            let mut changed = false;
            for id in 0..self.threads.len() {
                if let State::Waiting(Wait::Mutex { owner, .. }, _) = self.threads[id].state {
                    let priority = self.threads[id].effective;
                    if self.threads[owner].effective > priority {
                        self.threads[owner].effective = priority;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for (id, before) in old.into_iter().enumerate() {
            let t = &self.threads[id];
            if t.effective != before && matches!(t.state, State::Ready) {
                self.cores[t.core].ready[usize::from(before)].retain(|&i| i != id);
                self.enqueue(id, false);
            }
        }
    }

    fn signaled(&self, objects: &[SyncObject]) -> Option<usize> {
        objects.iter().position(|o| match o {
            SyncObject::Event(e) => e.get(),
            SyncObject::Thread(id) => matches!(self.threads[*id].state, State::Exited),
        })
    }

    fn poll(&mut self, cpu: &UnicornCPU) {
        let now = cpu.ticks();
        for timer in &mut self.timers {
            if timer.deadline <= now {
                timer.event.set(true);
                // Catch up at the pause end, without replaying a burst of missed periods.
                timer.deadline = if timer.period == 0 { u64::MAX } else { now.saturating_add(timer.period) };
            }
        }
        for id in 0..self.threads.len() {
            if let State::Waiting(ref wait, end) = self.threads[id].state {
                let index = if let Wait::Sync(objects) = wait { self.signaled(objects) } else { None };
                if index.is_some() {
                    self.wake(id, 0, index);
                } else if end.is_some_and(|end| end <= now) {
                    let result = if matches!(wait, Wait::Sleep) { 0 } else { Error::TimedOut.code() };
                    self.wake(id, result, None);
                }
            }
        }
        self.priorities();
    }

    fn next_deadline(&self) -> Option<u64> {
        self.threads
            .iter()
            .filter_map(|t| if let State::Waiting(_, end) = t.state { end } else { None })
            .chain(self.timers.iter().filter_map(|t| (t.deadline != u64::MAX).then_some(t.deadline)))
            .min()
    }

    fn select(&mut self) -> Option<(usize, usize)> {
        for offset in 0..CORE_COUNT {
            let core = (self.next_core + offset) % CORE_COUNT;
            if self.cores[core].ready.iter().all(VecDeque::is_empty) {
                let candidate = self
                    .threads
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| matches!(t.state, State::Ready) && t.affinity & (1 << core) != 0)
                    .min_by_key(|(_, t)| (t.effective, t.order))
                    .map(|(id, _)| id);
                if let Some(id) = candidate {
                    self.dequeue(id);
                    self.threads[id].core = core;
                    self.enqueue(id, false);
                }
            }
            let skip = self.yield_any.then_some(self.current);
            let candidate =
                self.cores[core].ready.iter().enumerate().find_map(|(p, q)| q.iter().position(|id| Some(*id) != skip).map(|i| (p, i)));
            if let Some((priority, position)) = candidate {
                let id = self.cores[core].ready[priority].remove(position).unwrap();
                self.next_core = (core + 1) % CORE_COUNT;
                self.yield_any = false;
                return Some((id, core));
            }
        }
        if self.yield_any {
            self.yield_any = false;
            self.select()
        } else {
            None
        }
    }

    fn waiters(&self, matches: impl Fn(&Wait) -> bool, count: i32) -> Vec<usize> {
        let mut ids: Vec<_> = self
            .threads
            .iter()
            .enumerate()
            .filter_map(|(id, t)| match &t.state {
                State::Waiting(w, _) if matches(w) => Some(id),
                _ => None,
            })
            .collect();
        ids.sort_by_key(|&id| (self.threads[id].effective, self.threads[id].order));
        if count > 0 {
            ids.truncate(count as usize);
        }
        ids
    }

    fn unlock(&mut self, cpu: &UnicornCPU, owner: usize, addr: u64) -> Result<(), Error> {
        word(cpu, addr)?;
        let waiters = self.waiters(|w| matches!(w, Wait::Mutex { owner: o, addr: a, .. } if *o == owner && *a == addr), 0);
        let mut value = 0;
        if let Some(&next) = waiters.first() {
            if let State::Waiting(Wait::Mutex { tag, .. }, _) = self.threads[next].state {
                value = tag;
            }
            if waiters.len() > 1 {
                value |= LISTENERS;
            }
            for &id in &waiters[1..] {
                if let State::Waiting(Wait::Mutex { owner, .. }, _) = &mut self.threads[id].state {
                    *owner = next;
                }
            }
            self.wake(next, 0, None);
        }
        put_word(cpu, addr, value)?;
        self.priorities();
        Ok(())
    }
}

impl Kernel {
    /// Enable deterministic block-driven ticks before starting the process.
    #[must_use]
    pub fn with_virtual_time(mut self, enabled: bool) -> Self {
        self.scheduler.virtual_time = enabled;
        self
    }

    pub(super) fn start_threads(&mut self, cpu: &UnicornCPU) -> u32 {
        cpu.set_virtual_time(self.scheduler.virtual_time);
        let id = 0;
        self.scheduler.add(id, cpu.context().expect("main context"), 44, 0);
        let handle = self.process.handles.add(Object::Thread(id));
        cpu.set_x(1, u64::from(handle));
        cpu.save(&mut self.scheduler.threads[id].context).expect("main context");
        self.scheduler.enqueue(id, false);
        handle
    }

    pub(super) fn run_threads(&mut self, cpu: &UnicornCPU) -> bool {
        // Loaders set the entry ABI registers after Kernel::start.
        if self.scheduler.switches == 0 && cpu.save(&mut self.scheduler.threads[0].context).is_err() {
            return false;
        }
        loop {
            self.scheduler.poll(cpu);
            let Some((id, core)) = self.scheduler.select() else {
                if self.scheduler.threads.iter().all(|t| matches!(t.state, State::Exited)) {
                    return true;
                }
                if let Some(end) = self.scheduler.next_deadline() {
                    cpu.idle_until(end);
                    continue;
                }
                return false;
            };
            self.scheduler.current = id;
            self.scheduler.threads[id].state = State::Running;
            let t = &mut self.scheduler.threads[id];
            if cpu.restore(&t.context).is_err() {
                return false;
            }
            if let Some((result, index)) = t.result.take() {
                cpu.set_x(0, u64::from(result));
                if let Some(index) = index {
                    cpu.set_x(1, index as u64);
                }
            }
            self.scheduler.switches += 1;
            self.hid.borrow_mut().tick(cpu);
            let now = cpu.ticks();
            if self.scheduler.cores[core].current != Some(id) || now >= self.scheduler.cores[core].slice_end {
                let quantum = ticks(375_000);
                #[cfg(test)]
                let quantum = if self.scheduler.seed != 0 {
                    self.scheduler.seed ^= self.scheduler.seed << 13;
                    self.scheduler.seed ^= self.scheduler.seed >> 7;
                    self.scheduler.seed ^= self.scheduler.seed << 17;
                    (1 + self.scheduler.seed % 32) * 192
                } else {
                    quantum
                };
                self.scheduler.cores[core].slice_end = now.saturating_add(quantum);
                self.scheduler.cores[core].current = Some(id);
            }
            let end = self.scheduler.cores[core].slice_end.min(self.scheduler.next_deadline().unwrap_or(u64::MAX));
            let sliced = match cpu.run_slice(end, core) {
                Ok(sliced) => sliced,
                Err(error) => {
                    log::error!("thread {id} at {:#x}: {error:?}", cpu.get_pc());
                    return false;
                }
            };
            self.scheduler.poll(cpu);
            self.scheduler.rotate = sliced || cpu.ticks() >= self.scheduler.cores[core].slice_end;
            if let Some(nr) = cpu.svc_pending() {
                #[cfg(test)]
                self.scheduler.trace.push((id, nr, cpu.ticks()));
                let fatal_break = nr == 0x26 && cpu.get_x(0) & 0x8000_0000 == 0;
                let Some(x) = self.svc(cpu, nr, std::array::from_fn(|i| cpu.get_x(i as u32))) else {
                    return true;
                };
                if fatal_break {
                    return false;
                }
                for (i, value) in x.into_iter().enumerate() {
                    cpu.set_x(i as u32, value);
                }
            } else if !sliced {
                return false;
            }
            if cpu.save(&mut self.scheduler.threads[id].context).is_err() {
                return false;
            }
            if self.scheduler.rotate || !matches!(self.scheduler.threads[id].state, State::Running) {
                self.scheduler.cores[core].current = None;
            }
            if matches!(self.scheduler.threads[id].state, State::Running) {
                self.scheduler.enqueue(id, !self.scheduler.rotate);
            }
        }
    }

    fn thread_id(&self, handle: u32) -> Result<usize, Error> {
        if handle == CURRENT_THREAD && !self.scheduler.threads.is_empty() {
            return Ok(self.scheduler.current);
        }
        match self.process.handles.get(handle) {
            Some(Object::Thread(id)) => Ok(*id),
            _ => Err(Error::InvalidHandle),
        }
    }

    fn sync_object(&self, handle: u32) -> Result<SyncObject, Error> {
        match self.process.handles.get(handle) {
            Some(Object::Event(e)) => Ok(SyncObject::Event(e.clone())),
            Some(Object::Thread(id)) => Ok(SyncObject::Thread(*id)),
            _ => Err(Error::InvalidHandle),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn thread_svc(&mut self, cpu: &UnicornCPU, nr: u32, x: &mut [u64; 8]) -> Result<(), Error> {
        let current = self.scheduler.current;
        match nr {
            0x08 => {
                let core = if (x[5] as u32).cast_signed() == -2 { 0 } else { x[5] as i32 };
                if !(0..CORE_COUNT as i32).contains(&core) {
                    return Err(Error::InvalidCpuCore);
                }
                if x[4] >= 64 {
                    return Err(Error::InvalidPriority);
                }
                if x[1] & 3 != 0 || x[3] & 15 != 0 {
                    return Err(Error::InvalidAddress);
                }
                let id = self.scheduler.vacant(&self.process.handles);
                let tls = TLS + id as u64 * 0x200;
                if tls + 0x200 > MAIN_STACK.base {
                    return Err(Error::LimitReached);
                }
                let handle = self.process.handles.add(Object::Thread(id));
                let context = cpu.thread_context(x[1], x[2], x[3], tls, handle).map_err(|_| Error::LimitReached)?;
                cpu.write_mem(tls, &[0; 0x200]).map_err(|_| Error::InvalidCurrentMemory)?;
                self.scheduler.add(id, context, x[4] as u8, core as usize);
                x[1] = u64::from(handle);
            }
            0x09 => {
                let id = self.thread_id(x[0] as u32)?;
                if !matches!(self.scheduler.threads[id].state, State::Dormant) {
                    return Err(Error::InvalidState);
                }
                self.scheduler.enqueue(id, false);
            }
            0x0a => self.scheduler.threads[current].state = State::Exited,
            0x0b => {
                let ns = x[0] as i64;
                if ns > 0 {
                    self.scheduler.wait(Wait::Sleep, deadline(cpu, ns));
                } else if ns >= -2 {
                    self.scheduler.rotate = true;
                    self.scheduler.yield_any = ns == -2;
                    if ns == -1 {
                        self.scheduler.next_core = (self.scheduler.threads[current].core + 1) % CORE_COUNT;
                    }
                }
            }
            0x0c => x[1] = u64::from(self.scheduler.threads[self.thread_id(x[1] as u32)?].effective),
            0x0d => {
                let id = self.thread_id(x[0] as u32)?;
                if x[1] >= 64 {
                    return Err(Error::InvalidPriority);
                }
                self.scheduler.threads[id].priority = x[1] as u8;
                self.scheduler.priorities();
            }
            0x0e => {
                let t = &self.scheduler.threads[self.thread_id(x[2] as u32)?];
                x[1] = t.preferred as u64;
                x[2] = t.affinity;
            }
            0x0f => {
                let id = self.thread_id(x[0] as u32)?;
                let mut preferred = x[1] as i32;
                let mut mask = x[2];
                if preferred == -2 {
                    preferred = 0;
                    mask = 1;
                }
                if preferred == -3 {
                    preferred = self.scheduler.threads[id].preferred;
                }
                if !(-1..CORE_COUNT as i32).contains(&preferred) || mask & !ALL_CORES != 0 {
                    return Err(Error::InvalidCpuCore);
                }
                if mask == 0 || preferred >= 0 && mask & (1 << preferred) == 0 {
                    return Err(Error::InvalidCombination);
                }
                let ready = matches!(self.scheduler.threads[id].state, State::Ready);
                if ready {
                    self.scheduler.dequeue(id);
                }
                let t = &mut self.scheduler.threads[id];
                t.preferred = preferred;
                t.affinity = mask;
                if mask & (1 << t.core) == 0 {
                    t.core = if preferred >= 0 { preferred as usize } else { mask.trailing_zeros() as usize };
                }
                if ready {
                    self.scheduler.enqueue(id, false);
                }
            }
            0x11 | 0x12 | 0x17 => {
                let event = match self.process.handles.get(x[0] as u32) {
                    Some(Object::Event(e)) if nr != 0x11 => e,
                    Some(Object::WritableEvent(e)) if nr != 0x17 => e,
                    _ => return Err(Error::InvalidHandle),
                };
                if nr == 0x17 && !event.get() {
                    return Err(Error::InvalidState);
                }
                event.set(nr == 0x11);
            }
            0x18 => {
                if x[2] > 64 {
                    return Err(Error::OutOfRange);
                }
                let mut objects = Vec::with_capacity(x[2] as usize);
                for index in 0..x[2] {
                    let addr = x[1].checked_add(index * 4).ok_or(Error::InvalidPointer)?;
                    objects.push(self.sync_object(word(cpu, addr)?)?);
                }
                if let Some(index) = self.scheduler.signaled(&objects) {
                    x[1] = index as u64;
                } else if x[3] == 0 {
                    return Err(Error::TimedOut);
                } else if std::mem::take(&mut self.scheduler.threads[current].cancelled) {
                    return Err(Error::Cancelled);
                } else {
                    self.scheduler.wait(Wait::Sync(objects), deadline(cpu, x[3] as i64));
                }
            }
            0x19 => {
                let id = self.thread_id(x[0] as u32)?;
                if matches!(self.scheduler.threads[id].state, State::Waiting(Wait::Sync(_), _)) {
                    self.scheduler.wake(id, Error::Cancelled.code(), None);
                } else {
                    self.scheduler.threads[id].cancelled = true;
                }
            }
            0x1a => {
                if word(cpu, x[1])? == x[0] as u32 | LISTENERS {
                    let owner = self.thread_id(x[0] as u32)?;
                    self.scheduler.wait(Wait::Mutex { owner, addr: x[1], tag: x[2] as u32 }, None);
                    self.scheduler.priorities();
                }
            }
            0x1b => self.scheduler.unlock(cpu, current, x[0])?,
            0x1c => {
                word(cpu, x[1])?;
                self.scheduler.unlock(cpu, current, x[0])?;
                put_word(cpu, x[1], 1)?;
                if x[3] == 0 {
                    return Err(Error::TimedOut);
                }
                self.scheduler.wait(Wait::Cond { key: x[1], addr: x[0], tag: x[2] as u32 }, deadline(cpu, x[3] as i64));
            }
            0x1d => {
                let ids = self.scheduler.waiters(|w| matches!(w, Wait::Cond { key, .. } if *key == x[0]), x[1] as i32);
                for id in ids {
                    let State::Waiting(Wait::Cond { addr, tag, .. }, end) = self.scheduler.threads[id].state else { unreachable!() };
                    match word(cpu, addr).and_then(|value| {
                        put_word(cpu, addr, if value == 0 { tag } else { value | LISTENERS })?;
                        if value == 0 { Ok(None) } else { self.thread_id(value & !LISTENERS).map(Some) }
                    }) {
                        Ok(Some(owner)) => self.scheduler.threads[id].state = State::Waiting(Wait::Mutex { owner, addr, tag }, end),
                        Ok(None) => self.scheduler.wake(id, 0, None),
                        Err(e) => self.scheduler.wake(id, e.code(), None),
                    }
                }
                if self.scheduler.waiters(|w| matches!(w, Wait::Cond { key, .. } if *key == x[0]), 1).is_empty() {
                    put_word(cpu, x[0], 0)?;
                }
                self.scheduler.priorities();
            }
            0x25 => x[1] = self.scheduler.threads[self.thread_id(x[1] as u32)?].uid,
            0x26 => {
                if x[0] & 0x8000_0000 == 0 {
                    for t in &mut self.scheduler.threads {
                        t.state = State::Exited;
                    }
                    for core in &mut self.scheduler.cores {
                        for queue in &mut core.ready {
                            queue.clear();
                        }
                    }
                    log::error!("guest break {:#x} at {:#x}", x[0], cpu.get_pc());
                }
            }
            0x34 => {
                let value = word(cpu, x[0])? as i32;
                let matches = match x[1] {
                    0 | 1 => value < x[2] as i32,
                    2 => value == x[2] as i32,
                    _ => return Err(Error::InvalidEnumValue),
                };
                if x[1] == 1 {
                    put_word(cpu, x[0], value.wrapping_sub(1) as u32)?;
                }
                if !matches {
                    return Err(Error::InvalidState);
                }
                if x[3] == 0 {
                    return Err(Error::TimedOut);
                }
                self.scheduler.wait(Wait::Address(x[0]), deadline(cpu, x[3] as i64));
            }
            0x35 => {
                let value = word(cpu, x[0])?;
                let ids = self.scheduler.waiters(|w| matches!(w, Wait::Address(addr) if *addr == x[0]), 0);
                let count = x[3] as i32;
                let delta: i32 = match x[1] {
                    0 => 0,
                    1 => 1,
                    2 => {
                        if ids.is_empty() {
                            1
                        } else if count <= 0 {
                            -2
                        } else if ids.len() < count as usize {
                            -1
                        } else {
                            0
                        }
                    }
                    _ => return Err(Error::InvalidEnumValue),
                };
                if x[1] != 0 {
                    if value != x[2] as u32 {
                        return Err(Error::InvalidState);
                    }
                    put_word(cpu, x[0], value.wrapping_add_signed(delta))?;
                }
                for id in ids.into_iter().take(if count <= 0 { usize::MAX } else { count as usize }) {
                    self.scheduler.wake(id, 0, None);
                }
            }
            0x45 => {
                let e = Rc::new(Cell::new(false));
                x[1] = u64::from(self.process.handles.add(Object::WritableEvent(e.clone())));
                x[2] = u64::from(self.process.handles.add(Object::Event(e)));
            }
            _ => return Err(Error::NotImplemented),
        }
        Ok(())
    }

    pub(super) fn current_core(&self) -> u64 {
        self.scheduler.threads[self.scheduler.current].core as u64
    }
}
