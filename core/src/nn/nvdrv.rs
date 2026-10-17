//! nvdrv: the device files a process opens by path and drives with ioctls, after switchbrew NV services and Ryujinx
//! Services/Nv. nvmap handles name guest buffers, an address space maps them at GPU virtual addresses in the engine's
//! page map, ctrl keeps the syncpoints and a channel hands its GPFIFO entries to the engine. Submits run to
//! completion, so every fence a channel returns is already reached.
use crate::cpu::UnicornCPU;
use crate::gpu::compute_class::CLASS_ID as COMPUTE;
use crate::gpu::engine::Gpu;
use crate::gpu::memory::PAGE;
use crate::kernel::{HandleTable, Object};
use crate::nn::hipc::{Buffer, Request, Response, Service};
use scene::threed::CLASS_ID as THREED;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// switchbrew NV services NvError.
#[derive(Debug, Clone, Copy)]
enum NvError {
    NotImplemented = 1,
    NotSupported = 2,
    NotInitialized = 3,
    BadParameter = 4,
    Timeout = 5,
    BadValue = 0xb,
    DeviceNotFound = 0x3_0011,
}

// yuzu's nvhost_as_gpu split: switchbrew has no Switch 2 GET_VA_REGIONS capture, and GA10F reports no big page size.
const SMALL_PAGES: u64 = PAGE << 10;
const BIG_PAGES: u64 = 1 << 34;
const VA_END: u64 = 1 << 37;

struct NvmapObject {
    size: u32,
    align: u32,
    kind: u8,
    address: u64,
    refs: u32,
}

#[derive(Default)]
struct AddressSpace {
    /// [start, end) by start: AllocSpace reservations and the mappings MapBufferEx placed itself.
    used: BTreeMap<u64, u64>,
    /// Length by VA, and whether MapBufferEx placed the mapping rather than the guest.
    mappings: HashMap<u64, (u64, bool)>,
}

impl AddressSpace {
    fn reserve(&mut self, start: u64, size: u64) -> Option<u64> {
        let end = start.checked_add(size).filter(|&end| end > start && end <= VA_END && start.is_multiple_of(PAGE))?;
        self.used.range(..end).next_back().is_none_or(|(_, &used)| used <= start).then(|| self.used.insert(start, end)).map(|_| start)
    }

    /// The lowest free `align`ed range of `size` bytes in the big page region.
    fn allocate(&mut self, size: u64, align: u64) -> Option<u64> {
        let mut start = BIG_PAGES;
        for (&used, &end) in &self.used {
            if start.saturating_add(size) <= used {
                break;
            }
            start = start.max(end.next_multiple_of(align));
        }
        self.reserve(start, size)
    }

    fn reserved(&self, va: u64, len: u64) -> bool {
        self.used.range(..=va).next_back().is_some_and(|(_, &end)| va.saturating_add(len) <= end)
    }
}

#[derive(Default)]
struct Channel {
    bound: bool,
    syncpoint: Option<usize>,
}

enum Device {
    Nvmap,
    AddressSpace(AddressSpace),
    Ctrl,
    CtrlGpu,
    Channel(Channel),
}

pub struct Nvdrv {
    gpu: Gpu<'static>,
    fds: HandleTable<Device>,
    nvmap: HandleTable<NvmapObject>,
    syncpoints: Vec<u32>,
}

fn get32(args: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(args[at..at + 4].try_into().unwrap())
}

fn get64(args: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(args[at..at + 8].try_into().unwrap())
}

fn set32(args: &mut [u8], at: usize, value: u32) {
    args[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn set64(args: &mut [u8], at: usize, value: u64) {
    args[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn fields(fields: &[(&[u64], usize)]) -> Vec<u8> {
    fields.iter().flat_map(|&(values, width)| values.iter().flat_map(move |v| v.to_le_bytes().into_iter().take(width))).collect()
}

/// The GA10F characteristics switchbrew lists for Switch 2, packed into 0xD0 bytes.
fn characteristics() -> Vec<u8> {
    let classes = [0x902d, THREED.into(), COMPUTE.into(), 0xc76f, 0xa140, 0xc7b5];
    fields(&[
        (&[0x170, 0xe, 0xa1, 1], 4),
        (&[0x10_0000, 0], 8),
        (&[6, 0x20, 0, 0x1_0000, 0x15, 0, 1, 0x808, 0x806, 0x60, 0x28, 0], 4),
        (&[0x93_5faf_1edc_0155], 8),
        (&classes, 4),
        (&[0x1f, 0xf, 0x2b, 0x21, 0xd, 0xffff, 0xffff, 0xffff], 2),
        (&[0, 1, 1, 1, 4, 0, 1, 0, 0], 4),
        (&[0x67_6131_3066], 8),
        (&[0, 2, 0x40, 3, 7, 1, 1, 0, 0], 4),
    ])
}

/// HipcAutoSelect buffers arrive as a mapped descriptor or, when that one is empty, a pointer one.
fn pick<'r>(mapped: &'r [Buffer], pointer: &'r [Buffer], i: usize) -> Option<&'r Buffer> {
    mapped.get(i).filter(|b| b.size != 0).or(pointer.get(i))
}

fn read(mem: &UnicornCPU, buffer: Option<&Buffer>, len: usize) -> Result<Vec<u8>, NvError> {
    let mut bytes = vec![0; len];
    if let Some(b) = buffer.filter(|b| b.size != 0) {
        let n = len.min(b.size as usize);
        mem.read_mem(b.addr, &mut bytes[..n]).map_err(|_| NvError::BadParameter)?;
    }
    Ok(bytes)
}

fn write(mem: &UnicornCPU, buffer: Option<&Buffer>, bytes: &[u8]) -> Result<(), NvError> {
    match buffer.filter(|b| b.size != 0) {
        Some(b) => mem.write_mem(b.addr, &bytes[..bytes.len().min(b.size as usize)]).map_err(|_| NvError::BadParameter),
        None => Ok(()),
    }
}

impl Nvdrv {
    pub fn new(gpu: Gpu<'static>) -> Self {
        Self { gpu, fds: HandleTable::default(), nvmap: HandleTable::default(), syncpoints: Vec::new() }
    }

    fn open(&mut self, path: &[u8]) -> Result<u32, NvError> {
        let device = match path.split(|&b| b == 0).next().unwrap_or_default() {
            b"/dev/nvmap" => Device::Nvmap,
            b"/dev/nvhost-as-gpu" => Device::AddressSpace(AddressSpace::default()),
            b"/dev/nvhost-ctrl" => Device::Ctrl,
            b"/dev/nvhost-ctrl-gpu" => Device::CtrlGpu,
            b"/dev/nvhost-gpu" => Device::Channel(Channel::default()),
            _ => return Err(NvError::DeviceNotFound),
        };
        Ok(self.fds.add(device))
    }

    /// Ioctl, Ioctl2 and Ioctl3: the argument struct is in and out through the first buffers, Ioctl2 adds a second
    /// input and Ioctl3 a second output.
    fn ioctls(&mut self, mem: &UnicornCPU, [fd, code]: [u32; 2], inputs: [Option<&Buffer>; 2], outputs: [Option<&Buffer>; 2]) -> Result<(), NvError> {
        let mut args = read(mem, inputs[0], (code >> 16 & 0x3fff) as usize)?;
        let extra_len = inputs[1].or(outputs[1]).map_or(0, |b| b.size as usize);
        let mut extra = read(mem, inputs[1], extra_len)?;
        let result = self.ioctl(fd, code, &mut args, &mut extra);
        write(mem, outputs[0], &args)?;
        write(mem, outputs[1], &extra)?;
        result
    }

    fn ioctl(&mut self, fd: u32, code: u32, args: &mut [u8], extra: &mut [u8]) -> Result<(), NvError> {
        use NvError::*;
        match (self.fds.get_mut(fd).ok_or(BadParameter)?, code) {
            (Device::Nvmap, 0xc008_0101) => {
                let size = get32(args, 0);
                if size == 0 {
                    return Err(BadValue);
                }
                set32(args, 4, self.nvmap.add(NvmapObject { size, align: 0, kind: 0, address: 0, refs: 1 }));
            }
            (Device::Nvmap, 0xc008_0103) => {
                self.nvmap.get_mut(get32(args, 0)).ok_or(BadValue)?.refs += 1;
                set32(args, 4, get32(args, 0));
            }
            (Device::Nvmap, 0xc020_0104) => {
                let object = self.nvmap.get_mut(get32(args, 0)).ok_or(BadValue)?;
                let (align, address) = (get32(args, 12).max(0x1000), get64(args, 24));
                if !align.is_power_of_two() || address == 0 {
                    return Err(BadValue);
                }
                (object.align, object.kind, object.address) = (align, args[16], address);
                set32(args, 12, align);
            }
            (Device::Nvmap, 0xc018_0105) => {
                let handle = get32(args, 0);
                let object = self.nvmap.get_mut(handle).ok_or(BadValue)?;
                object.refs -= 1;
                let (address, size, freed) = (object.address, object.size, object.refs == 0);
                if freed {
                    self.nvmap.remove(handle);
                }
                set64(args, 8, if freed { address } else { 0 });
                set32(args, 16, size);
            }
            (Device::Nvmap, 0xc00c_0109) => {
                let object = self.nvmap.get_mut(get32(args, 0)).ok_or(BadValue)?;
                let value = match get32(args, 4) {
                    1 => object.size,
                    2 => object.align,
                    4 => 0x4000_0000,
                    5 => object.kind.into(),
                    _ => return Err(BadValue),
                };
                set32(args, 8, value);
            }
            (Device::Nvmap, 0xc008_010e) => {
                self.nvmap.get_mut(get32(args, 4)).ok_or(BadValue)?;
                set32(args, 0, get32(args, 4));
            }
            (Device::AddressSpace(_), 0x4004_4101) => match self.fds.get_mut(get32(args, 0)) {
                Some(Device::Channel(channel)) => channel.bound = true,
                _ => return Err(BadParameter),
            },
            (Device::AddressSpace(space), 0xc018_4102) => {
                let (page_size, size) = (get32(args, 4), u64::from(get32(args, 0)) * u64::from(get32(args, 4)));
                let align = get64(args, 16).max(page_size.into()).max(PAGE);
                let offset = match get32(args, 8) & 1 {
                    0 => Some(align).filter(|a| a.is_power_of_two()).and_then(|align| space.allocate(size, align)),
                    _ => space.reserve(get64(args, 16), size),
                };
                set64(args, 16, offset.ok_or(BadValue)?);
            }
            (Device::AddressSpace(space), 0xc008_4105) => {
                let va = get64(args, 0);
                let (len, placed) = space.mappings.remove(&va).ok_or(BadValue)?;
                if placed {
                    space.used.remove(&va);
                }
                self.gpu.space.unmap(va, len);
            }
            (Device::AddressSpace(space), 0xc028_4106) => {
                let object = self.nvmap.get_mut(get32(args, 8)).filter(|o| o.address != 0).ok_or(BadValue)?;
                let offset = get64(args, 16);
                let size = match get64(args, 24) {
                    0 => u64::from(object.size).checked_sub(offset).ok_or(BadValue)?,
                    size => size,
                };
                let fixed = get32(args, 0) & 1 != 0;
                let (pa, len) = (object.address.checked_add(offset).ok_or(BadValue)?, size.checked_next_multiple_of(PAGE).ok_or(BadValue)?);
                let inside = offset.checked_add(size).is_some_and(|end| end <= object.size.into());
                if !inside || !pa.is_multiple_of(PAGE) || pa.checked_add(len).is_none_or(|end| end > self.gpu.ram.size() as u64) {
                    return Err(BadValue);
                }
                let va = match fixed {
                    true => Some(get64(args, 32)).filter(|&va| va.is_multiple_of(PAGE) && space.reserved(va, len)),
                    false => space.allocate(len, PAGE),
                };
                let va = va.ok_or(BadValue)?;
                self.gpu.map(va, pa, len);
                space.mappings.insert(va, (len, !fixed));
                set64(args, 32, va);
            }
            (Device::AddressSpace(_), 0xc040_4108) => {
                set32(args, 8, 48);
                args[16..].copy_from_slice(&fields(&[(&[SMALL_PAGES], 8), (&[0x1000, 0], 4), (&[(BIG_PAGES - SMALL_PAGES) >> 12, BIG_PAGES], 8), (&[PAGE, 0], 4), (&[(VA_END - BIG_PAGES) / PAGE], 8)]));
            }
            (Device::AddressSpace(_), 0x4028_4109) => {}
            (Device::Ctrl, 0xc008_0014) => set32(args, 4, *self.syncpoints.get(get32(args, 0) as usize).ok_or(BadValue)?),
            (Device::Ctrl, 0x4004_0015) => *self.syncpoints.get_mut(get32(args, 0) as usize).ok_or(BadValue)? += 1,
            (Device::Ctrl, 0xc00c_0016 | 0xc010_001d) => {
                let value = *self.syncpoints.get(get32(args, 0) as usize).ok_or(BadValue)?;
                if (value.wrapping_sub(get32(args, 4)) as i32) < 0 {
                    return Err(Timeout);
                }
                if code == 0xc010_001d {
                    set32(args, 12, value);
                }
            }
            (Device::CtrlGpu, 0x8004_4701) => set32(args, 0, 1),
            // GM20B's, from yuzu nvhost_ctrl_gpu: GA10F's zcull layout is not documented.
            (Device::CtrlGpu, 0x8028_4702) => args.copy_from_slice(&fields(&[(&[0x20, 0x20, 0x400, 0x800, 0x20, 0x20, 0xc0, 0x20, 0x40, 0x10], 4)])),
            (Device::CtrlGpu, 0xc0e0_4705) => {
                let characteristics = characteristics();
                set64(args, 0, characteristics.len() as u64);
                extra.get_mut(..characteristics.len()).ok_or(BadParameter)?.copy_from_slice(&characteristics);
            }
            (Device::CtrlGpu, 0xc018_4706) => {
                if get32(args, 0) != 0 {
                    set64(args, 16, 0x3f);
                }
            }
            (Device::Channel(_), 0x4004_4801 | 0x4004_4803 | 0xc010_480b | 0xc018_480c | 0x4004_480d | 0x4008_4714) => {}
            (Device::Channel(channel), 0xc020_481a) => {
                channel.syncpoint = Some(self.syncpoints.len());
                set32(args, 12, self.syncpoints.len() as u32);
                set32(args, 16, 0);
                self.syncpoints.push(0);
            }
            (Device::Channel(channel), 0xc010_4809) => {
                if !channel.bound {
                    return Err(NotInitialized);
                }
                if ![THREED, COMPUTE].map(u32::from).contains(&get32(args, 0)) {
                    return Err(BadValue);
                }
            }
            (Device::Channel(channel), 0xc018_481b) => {
                let id = channel.syncpoint.ok_or(NotInitialized)?;
                let entries = extra.get(..8 * get32(args, 8) as usize).ok_or(BadParameter)?;
                let entries: Vec<[u8; 8]> = entries.chunks_exact(8).map(|e| e.try_into().unwrap()).collect();
                self.gpu.submit(&entries).map_err(|e| {
                    log::warn!("nvhost-gpu submit: {e}");
                    NotSupported
                })?;
                self.syncpoints[id] += 1;
                set32(args, 16, id as u32);
                set32(args, 20, self.syncpoints[id]);
            }
            _ => {
                log::warn!("ioctl {code:#x} on nvdrv fd {fd}: {NotImplemented:?}");
                return Err(NotImplemented);
            }
        }
        Ok(())
    }
}

impl Service for Nvdrv {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        let words = [0, 1].map(|i| request.args.get(4 * i..4 * i + 4).map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap())));
        let input = |i| pick(&request.a, &request.x, i);
        let output = |i| pick(&request.b, &request.c, i);
        let (mut fd, mut copy_handles) = (0, Vec::new());
        let result = match cmd {
            0 => read(request.mem, input(0), input(0).map_or(0, |b| b.size as usize)).and_then(|path| self.open(&path)).map(|opened| fd = opened),
            1 => self.ioctls(request.mem, words, [input(0), None], [output(0), None]),
            2 => self.fds.remove(words[0]).map(drop).ok_or(NvError::BadParameter),
            // Initialize's transfer memory is the driver's own heap, which nothing here allocates from.
            3 | 8 => Ok(()),
            4 => match self.fds.get(words[0]) {
                Some(Device::Channel(_)) if (1..=3).contains(&words[1]) => {
                    copy_handles.push(Object::Event(Rc::default()));
                    Ok(())
                }
                _ => Err(NvError::NotSupported),
            },
            11 => self.ioctls(request.mem, words, [input(0), input(1)], [output(0), None]),
            12 => self.ioctls(request.mem, words, [input(0), None], [output(0), output(1)]),
            _ => return Response::unknown_command(cmd),
        };
        let error = result.err().map_or(0, |e| e as u32);
        let data = if cmd == 0 { vec![fd, error] } else { vec![error] };
        Response { data: data.iter().flat_map(|w| w.to_le_bytes()).collect(), copy_handles, ..Response::default() }
    }
}

impl crate::nn::binder::NvmapResolve for RefCell<Nvdrv> {
    fn address(&self, id: u32) -> Option<u64> {
        self.borrow().nvmap.get(id).filter(|object| object.address != 0).map(|object| object.address)
    }
}
