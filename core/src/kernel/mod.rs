//! HLE Horizon kernel: one process with scheduled threads whose SVCs are dispatched from the
//! core's run loop, outside the Unicorn lock. Guest virtual addresses equal offsets into the flat
//! RAM block, so the regions below are bookkeeping over memory that is always mapped.

mod threads;
pub use threads::{Scheduler, Timer};

use crate::cpu::UnicornCPU;
use crate::cpu::cpu_manager::MEMORY_SIZE;
use crate::nn::hipc::{self, Service};
use crate::nn::{hid, sm::Sm};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Region {
    pub base: u64,
    pub size: u64,
}

impl Region {
    fn contains(&self, addr: u64, size: u64) -> bool {
        addr >= self.base && addr.checked_add(size).is_some_and(|end| end <= self.base + self.size)
    }
}

// Our own layout, 2 MiB aligned like Horizon's ASLR regions: switchbrew documents no
// Switch 2 region bases, so these need a capture before they can claim to match one.
pub const CODE: Region = Region { base: 0x0800_0000, size: 0x0800_0000 };
/// TLS region with 0x200-byte thread slots; the first 0x100 bytes hold IPC.
pub const TLS: u64 = 0x1000_0000;
pub const MAIN_STACK: Region = Region { base: 0x1010_0000, size: 0x10_0000 };
pub const STACK: Region = Region { base: 0x2000_0000, size: 0x2000_0000 };
pub const HEAP: Region = Region { base: 0x4000_0000, size: 0x1_0000_0000 };
pub const ALIAS: Region = Region { base: 0x1_8000_0000, size: 0x1_0000_0000 };
pub const ASLR: Region = Region { base: CODE.base, size: MEMORY_SIZE - CODE.base };
const TOTAL_MEMORY: u64 = HEAP.size;
const PAGE: u64 = 0x1000;
/// SetHeapSize granularity; the heap region is its 4 GiB cap (switchbrew SVC#SetHeapSize).
const HEAP_ALIGN: u64 = 0x20_0000;
pub const CURRENT_PROCESS: u32 = 0xFFFF_8001;

/// Kernel result descriptions (module 1), yuzu src/core/hle/kernel/svc_results.h.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    NotImplemented = 33,
    InvalidSize = 101,
    InvalidAddress = 102,
    InvalidCurrentMemory = 106,
    InvalidNewMemoryPermission = 108,
    InvalidMemoryRegion = 110,
    InvalidPriority = 112,
    InvalidCpuCore = 113,
    InvalidHandle = 114,
    InvalidCombination = 116,
    TimedOut = 117,
    Cancelled = 118,
    InvalidState = 125,
    InvalidPointer = 115,
    OutOfRange = 119,
    InvalidEnumValue = 120,
    NotFound = 121,
    LimitReached = 132,
}

impl Error {
    pub fn code(self) -> u32 {
        1 | (self as u32) << 9
    }
}

pub enum Object {
    Thread(usize),
    WritableEvent(Rc<Cell<bool>>),
    Session(Rc<RefCell<dyn Service>>),
    TransferMemory { addr: u64, size: u64, perm: u32 },
    Event(Rc<Cell<bool>>),
    /// Without an MMU a mapping only records the guest address its owner writes to.
    SharedMemory { size: u64, mapped: Rc<Cell<Option<u64>>> },
}

pub struct HandleTable<T> {
    objects: HashMap<u32, T>,
    last: u32,
}

impl<T> Default for HandleTable<T> {
    fn default() -> Self {
        Self { objects: HashMap::new(), last: 0 }
    }
}

impl<T> HandleTable<T> {
    pub fn add(&mut self, object: T) -> u32 {
        self.last += 1;
        self.objects.insert(self.last, object);
        self.last
    }

    pub fn get(&self, handle: u32) -> Option<&T> {
        self.objects.get(&handle)
    }

    pub fn remove(&mut self, handle: u32) -> Option<T> {
        self.objects.remove(&handle)
    }
}

#[derive(Default)]
pub struct Process {
    pub handles: HandleTable<Object>,
    pub heap_size: u64,
    image_size: u64,
    /// Disjoint [start, end) ranges MapPhysicalMemory has mapped in the alias region.
    physical: Vec<(u64, u64)>,
    /// MapMemory (dst, src, size) mappings in the stack region.
    aliases: Vec<(u64, u64, u64)>,
}

pub struct Kernel {
    pub scheduler: Scheduler,
    pub process: Process,
    pub ports: HashMap<String, Rc<RefCell<dyn Service>>>,
    /// OutputDebugString texts, for the caller to drain.
    pub debug_output: Vec<String>,
    pub hid: Rc<RefCell<hid::State>>,
}

impl Default for Kernel {
    fn default() -> Self {
        let hid = Rc::new(RefCell::new(hid::State::default()));
        let sm: Rc<RefCell<dyn Service>> = Rc::new(RefCell::new(Sm(HashMap::from([("hid".into(), hid.clone() as _)]))));
        let ports = HashMap::from([("sm:".into(), sm)]);
        Self { scheduler: Scheduler::default(), process: Process::default(), ports, debug_output: vec![], hid }
    }
}

fn overlap(ranges: &[(u64, u64)], start: u64, end: u64) -> u64 {
    ranges.iter().map(|&(s, e)| e.min(end).saturating_sub(s.max(start))).sum()
}

fn check_pages(addr: u64, size: u64) -> Result<(), Error> {
    if !addr.is_multiple_of(PAGE) {
        return Err(Error::InvalidAddress);
    }
    if size == 0 || !size.is_multiple_of(PAGE) {
        return Err(Error::InvalidSize);
    }
    addr.checked_add(size).map(drop).ok_or(Error::InvalidCurrentMemory)
}

fn copy(cpu: &UnicornCPU, src: u64, dst: u64, size: u64) -> Result<(), Error> {
    let mut bytes = vec![0; size as usize];
    cpu.read_mem(src, &mut bytes).and_then(|()| cpu.write_mem(dst, &bytes)).map_err(|_| Error::InvalidCurrentMemory)
}

fn output_debug_string(cpu: &UnicornCPU, addr: u64, len: u64) -> Result<String, Error> {
    if addr.checked_add(len).is_none_or(|end| end > MEMORY_SIZE) {
        return Err(Error::InvalidPointer);
    }
    let mut text = vec![0; len as usize];
    cpu.read_mem(addr, &mut text).map_err(|_| Error::InvalidPointer)?;
    let text = String::from_utf8_lossy(&text).into_owned();
    log::info!("guest: {text}");
    Ok(text)
}

impl Process {
    fn used(&self) -> u64 {
        self.image_size + MAIN_STACK.size + self.heap_size + overlap(&self.physical, 0, u64::MAX)
    }

    fn reserve(&self, extra: u64) -> Result<(), Error> {
        if self.used() + extra > TOTAL_MEMORY { Err(Error::LimitReached) } else { Ok(()) }
    }

    fn mapped(&self, addr: u64, size: u64) -> bool {
        Region { base: HEAP.base, size: self.heap_size }.contains(addr, size)
            || overlap(&self.physical, addr, addr + size) == size
    }

    fn set_heap_size(&mut self, size: u64) -> Result<u64, Error> {
        if !size.is_multiple_of(HEAP_ALIGN) || size > HEAP.size {
            return Err(Error::InvalidSize);
        }
        self.reserve(size.saturating_sub(self.heap_size))?;
        self.heap_size = size;
        Ok(HEAP.base)
    }

    fn map_physical_memory(&mut self, addr: u64, size: u64) -> Result<(), Error> {
        check_pages(addr, size)?;
        if !ALIAS.contains(addr, size) {
            return Err(Error::InvalidMemoryRegion);
        }
        self.reserve(size - overlap(&self.physical, addr, addr + size))?;
        self.physical.push((addr, addr + size));
        self.physical.sort_unstable();
        self.physical.dedup_by(|next, prev| next.0 <= prev.1 && { prev.1 = prev.1.max(next.1); true });
        Ok(())
    }

    /// Without an MMU the alias is a move: Horizon locks the source while it is mapped, so the
    /// guest cannot see the difference.
    fn map_memory(&mut self, cpu: &UnicornCPU, dst: u64, src: u64, size: u64) -> Result<(), Error> {
        check_pages(dst, size)?;
        check_pages(src, size)?;
        if !self.mapped(src, size) {
            return Err(Error::InvalidCurrentMemory);
        }
        if !STACK.contains(dst, size) {
            return Err(Error::InvalidMemoryRegion);
        }
        let hits = |a: u64, b: u64, n: u64| a < b + size && b < a + n;
        if self.aliases.iter().any(|&(d, s, n)| hits(d, dst, n) || hits(s, src, n)) {
            return Err(Error::InvalidCurrentMemory);
        }
        copy(cpu, src, dst, size)?;
        self.aliases.push((dst, src, size));
        Ok(())
    }

    fn unmap_memory(&mut self, cpu: &UnicornCPU, dst: u64, src: u64, size: u64) -> Result<(), Error> {
        let index = self.aliases.iter().position(|&a| a == (dst, src, size)).ok_or(Error::InvalidCurrentMemory)?;
        copy(cpu, dst, src, size)?;
        self.aliases.remove(index);
        Ok(())
    }

    fn shared_memory(&self, handle: u32, size: u64) -> Result<&Rc<Cell<Option<u64>>>, Error> {
        match self.handles.get(handle) {
            Some(Object::SharedMemory { size: object_size, mapped }) if size == *object_size => Ok(mapped),
            Some(Object::SharedMemory { .. }) => Err(Error::InvalidSize),
            _ => Err(Error::InvalidHandle),
        }
    }

    fn map_shared_memory(&self, handle: u32, addr: u64, size: u64, perm: u64) -> Result<(), Error> {
        check_pages(addr, size)?;
        if perm != 1 {
            return Err(Error::InvalidNewMemoryPermission);
        }
        self.shared_memory(handle, size)?.set(Some(addr));
        Ok(())
    }

    fn unmap_shared_memory(&self, handle: u32, addr: u64, size: u64) -> Result<(), Error> {
        let mapped = self.shared_memory(handle, size)?;
        if mapped.get() != Some(addr) {
            return Err(Error::InvalidMemoryRegion);
        }
        mapped.set(None);
        Ok(())
    }

    fn create_transfer_memory(&mut self, addr: u64, size: u64, perm: u32) -> Result<u32, Error> {
        check_pages(addr, size)?;
        if !matches!(perm, 0 | 1 | 3) {
            return Err(Error::InvalidNewMemoryPermission);
        }
        if !self.mapped(addr, size) {
            return Err(Error::InvalidCurrentMemory);
        }
        Ok(self.handles.add(Object::TransferMemory { addr, size, perm }))
    }

    /// Info ids, yuzu src/core/hle/kernel/svc_types.h InfoType.
    fn info(&self, id: u64, handle: u32, sub: u64) -> Result<u64, Error> {
        let value = match id {
            2 => ALIAS.base,
            3 => ALIAS.size,
            4 => HEAP.base,
            5 => HEAP.size,
            6 => TOTAL_MEMORY,
            7 => self.used(),
            12 => ASLR.base,
            13 => ASLR.size,
            14 => STACK.base,
            15 => STACK.size,
            // The Switch 2 ids 29-33 (remote region, ProcessPageSize) need a capture for their values.
            _ => return Err(Error::InvalidEnumValue),
        };
        if sub != 0 {
            return Err(Error::InvalidEnumValue);
        }
        if handle != CURRENT_PROCESS {
            return Err(Error::InvalidHandle);
        }
        Ok(value)
    }
}

impl Kernel {
    /// Points the thread at an image of `image_size` bytes loaded at `entry`.
    pub fn start(&mut self, cpu: &UnicornCPU, entry: u64, image_size: u64) {
        self.process.image_size = image_size.next_multiple_of(PAGE);
        cpu.set_pc(entry);
        cpu.set_sp(MAIN_STACK.base + MAIN_STACK.size);
        cpu.set_tls(TLS);
        self.start_threads(cpu);
    }

    /// Runs ready threads and deadlines until process exit or an unhandled stop.
    pub fn run(&mut self, cpu: &UnicornCPU) -> bool { self.run_threads(cpu) }

    /// Returns x0-x7 after the call, or None once the process has exited.
    /// SVC ids and register use: yuzu src/core/hle/kernel/svc.h SvcId and svc.cpp SvcWrap_*64.
    pub fn svc(&mut self, cpu: &UnicornCPU, nr: u32, mut x: [u64; 8]) -> Option<[u64; 8]> {
        if nr == 0x10 { x[0] = self.current_core(); return Some(x); }
        if nr == 0x1e { x[0] = cpu.ticks(); return Some(x); }
        let process = &mut self.process;
        let result = match nr {
            0x08..=0x12 | 0x17..=0x1d | 0x25 | 0x26 | 0x34 | 0x35 | 0x45 => self.thread_svc(cpu, nr, &mut x),
            0x01 => process.set_heap_size(x[1]).map(|base| x[1] = base),
            0x04 => process.map_memory(cpu, x[0], x[1], x[2]),
            0x05 => process.unmap_memory(cpu, x[0], x[1], x[2]),
            0x07 => return None,
            0x13 => process.map_shared_memory(x[0] as u32, x[1], x[2], x[3]),
            0x14 => process.unmap_shared_memory(x[0] as u32, x[1], x[2]),
            0x15 => process.create_transfer_memory(x[1], x[2], x[3] as u32).map(|h| x[1] = h.into()),
            0x16 => process.handles.remove(x[0] as u32).map(drop).ok_or(Error::InvalidHandle),
            0x1f => self.connect_to_named_port(cpu, x[1]).map(|h| x[1] = h.into()),
            0x21 => self.send_sync_request(cpu, x[0] as u32),
            0x27 => output_debug_string(cpu, x[0], x[1]).map(|text| self.debug_output.push(text)),
            0x29 => process.info(x[1], x[2] as u32, x[3]).map(|value| x[1] = value),
            0x2c => process.map_physical_memory(x[0], x[1]),
            _ => Err(Error::NotImplemented),
        };
        x[0] = result.map_or_else(
            |e| {
                log::warn!("svc {nr:#x}: {e:?}");
                e.code().into()
            },
            |()| 0,
        );
        Some(x)
    }

    fn connect_to_named_port(&mut self, cpu: &UnicornCPU, addr: u64) -> Result<u32, Error> {
        let mut name = [0; 12];
        cpu.read_mem(addr, &mut name).map_err(|_| Error::InvalidPointer)?;
        let len = name.iter().position(|&b| b == 0).ok_or(Error::OutOfRange)?;
        let port = std::str::from_utf8(&name[..len]).ok().and_then(|name| self.ports.get(name));
        let service = port.ok_or(Error::NotFound)?.clone();
        Ok(self.process.handles.add(Object::Session(service)))
    }

    fn send_sync_request(&mut self, cpu: &UnicornCPU, handle: u32) -> Result<(), Error> {
        let Some(Object::Session(service)) = self.process.handles.get(handle) else {
            return Err(Error::InvalidHandle);
        };
        let service = service.clone();
        // switchbrew SVC#SendSyncRequest [S2]: every error closes the session (kernel or sysmodule is unknown).
        self.exchange(cpu, &service).inspect_err(|_| drop(self.process.handles.remove(handle)))
    }

    fn exchange(&mut self, cpu: &UnicornCPU, service: &RefCell<dyn Service>) -> Result<(), Error> {
        let mut message = [0; hipc::MESSAGE_SIZE];
        cpu.read_mem(if cpu.tls() == 0 { TLS } else { cpu.tls() }, &mut message).map_err(|_| Error::InvalidPointer)?;
        match hipc::command_type(&message) {
            hipc::CLOSE => return Ok(()),
            hipc::REQUEST | hipc::REQUEST_WITH_CONTEXT => {}
            other => {
                log::warn!("HIPC command type {other}: {:?}", Error::NotImplemented);
                return Err(Error::NotImplemented);
            }
        }
        let reply = match hipc::Request::parse(&message, cpu) {
            Ok((cmd, mut request)) => service.borrow_mut().dispatch(cmd, &mut request),
            Err(e) => {
                log::warn!("CMIF request: {e:?}");
                hipc::Response::error(e.code())
            }
        };
        let mut add = |objects: Vec<Object>| -> Vec<u32> {
            objects.into_iter().map(|o| self.process.handles.add(o)).collect()
        };
        let (copy, moved) = (add(reply.copy_handles), add(reply.move_handles));
        let bytes = hipc::encode_reply(reply.result, &reply.data, &copy, &moved);
        cpu.write_mem(if cpu.tls() == 0 { TLS } else { cpu.tls() }, &bytes).map_err(|_| Error::InvalidPointer)
    }
}
