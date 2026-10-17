//! IHOSBinderDriver and the one IGraphicBufferProducer behind it, as libnx nx/source/display/{binder,parcel,
//! buffer_producer,native_window}.c drive them: transaction codes, parcel layouts and slot states follow Ryujinx
//! Services/SurfaceFlinger (IHOSBinderDriver.cs, IGraphicBufferProducer.cs, BufferQueueProducer.cs, Types/).

use crate::cpu::UnicornCPU;
use crate::cpu::cpu_manager::MEMORY_SIZE;
use crate::gpu::texture::block_linear_to_pitch;
use crate::kernel::Object;
use crate::nn::hipc::{Request, Response, Service};
use crate::nn::vi::INVALID_VALUE;
use std::cell::Cell;
use std::rc::Rc;

pub const BINDER_ID: u32 = 1;
/// What Connect and QueueBuffer report, the size of Ryujinx's default layer.
const LAYER_SIZE: [u32; 2] = [1280, 720];
/// Android status codes, Ryujinx SurfaceFlinger/Status.cs.
const OK: i32 = 0;
const WOULD_BLOCK: i32 = -11;
const BAD_VALUE: i32 = -22;
const UNKNOWN_TRANSACTION: i32 = -74;
/// Byte offsets in the flattened GraphicBuffer: the 0x28-byte header, then NvGraphicBuffer and its first surface.
const NVMAP_ID: usize = 0x2c;
const SURFACE: usize = 0x28 + 0x34;
const BLOCK_LINEAR: u32 = 3;

pub trait NvmapResolve {
    fn address(&self, id: u32) -> Option<u64>;
}

#[derive(Default, Clone, Copy, PartialEq)]
enum State {
    #[default]
    Free,
    Dequeued,
    Presented,
}

#[derive(Default)]
struct Slot {
    state: State,
    buffer: Vec<u32>,
}

pub struct HosBinderDriver {
    nvmap: Rc<dyn NvmapResolve>,
    slots: [Slot; 3],
    count: usize,
    pub vsync: Rc<Cell<bool>>,
    buffer_free: Rc<Cell<bool>>,
}

fn word(words: &[u32], at: usize) -> u32 {
    words.get(at).copied().unwrap_or(0)
}

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect()
}

impl HosBinderDriver {
    pub fn new(nvmap: Rc<dyn NvmapResolve>) -> Self {
        Self { nvmap, slots: Default::default(), count: 3, vsync: Rc::default(), buffer_free: Rc::new(Cell::new(true)) }
    }

    /// The reply payload of IGraphicBufferProducer transaction `code`, its status last.
    fn transact(&mut self, mem: &UnicornCPU, code: u32, payload: &[u32]) -> Vec<u32> {
        let args = payload.get(2 + (word(payload, 1) as usize + 1).div_ceil(2)..).unwrap_or_default();
        let slot = word(args, 0) as usize;
        let dequeued = self.slots.get(slot).is_some_and(|s| s.state == State::Dequeued);
        let output = vec![LAYER_SIZE[0], LAYER_SIZE[1], 0, 0];
        let (mut reply, status) = match code {
            1 => match self.slots.get(slot).filter(|s| !s.buffer.is_empty()) {
                Some(s) => ([&[1, 4 * s.buffer.len() as u32, 0], &s.buffer[..]].concat(), OK),
                None => (vec![0], BAD_VALUE),
            },
            2 => {
                let count = match word(args, 0) as usize {
                    0 => self.slots.len(),
                    count => count,
                };
                if (2..=self.slots.len()).contains(&count) && !self.slots.iter().any(|s| s.state == State::Dequeued) {
                    self.count = count;
                    self.buffer_free.set(self.slots[..count].iter().any(|s| s.state == State::Free && !s.buffer.is_empty()));
                    (vec![], OK)
                } else {
                    (vec![], BAD_VALUE)
                }
            }
            3 => match self.slots[..self.count].iter().position(|s| s.state == State::Free && !s.buffer.is_empty()) {
                Some(free) => {
                    self.slots[free].state = State::Dequeued;
                    self.buffer_free.set(self.slots[..self.count].iter().any(|s| s.state == State::Free && !s.buffer.is_empty()));
                    // A non-null NvMultiFence with no fences.
                    ([vec![free as u32, 1, 36, 0], vec![0; 9]].concat(), OK)
                }
                None => (vec![u32::MAX, 0], WOULD_BLOCK),
            },
            7 if dequeued && self.present(mem, slot).is_some() => (output, OK),
            7 => (output, BAD_VALUE),
            8 if dequeued => {
                self.slots[slot].state = State::Free;
                self.buffer_free.set(true);
                (vec![], OK)
            }
            8 => (vec![], BAD_VALUE),
            9 => match LAYER_SIZE.get(word(args, 0) as usize) {
                Some(&value) => (vec![value], OK),
                None => (vec![0], BAD_VALUE),
            },
            10 => (output, OK),
            11 => {
                self.slots.iter_mut().for_each(|s| s.state = State::Free);
                self.buffer_free.set(self.slots[..self.count].iter().any(|s| !s.buffer.is_empty()));
                (vec![], OK)
            }
            14 => match self.slots.get_mut(slot).filter(|_| slot < self.count) {
                Some(s) => {
                    *s = Slot { state: State::Free, buffer: args.get(4..4 + word(args, 2) as usize / 4).unwrap_or_default().to_vec() };
                    self.buffer_free.set(true);
                    (vec![], OK)
                }
                None => (vec![], BAD_VALUE),
            },
            _ => (vec![], UNKNOWN_TRANSACTION),
        };
        reply.push(status as u32);
        reply
    }

    /// Shows the slot's RGBA8 surface, then frees the slot it replaces on screen.
    fn present(&mut self, mem: &UnicornCPU, slot: usize) -> Option<()> {
        let buffer = &self.slots[slot].buffer;
        let field = |at: usize| buffer.get(at / 4).map(|&w| w as usize);
        let (width, height, pitch, gobs) = (field(4)?, field(8)?, field(SURFACE + 0x14)?, field(SURFACE + 0x24)?);
        let bytes_per_pixel = field(SURFACE + 8)? >> 3 & 0x1f;
        if width == 0 || height == 0 || pitch == 0 || bytes_per_pixel != 4 || width * 4 > pitch || gobs > 5 {
            return None;
        }
        let block_linear = field(SURFACE + 0x10)? == BLOCK_LINEAR as usize;
        let rows = if block_linear { height.next_multiple_of(8 << gobs) } else { height };
        let len = pitch.checked_mul(rows).filter(|&len| len as u64 <= MEMORY_SIZE)?;
        let address = self.nvmap.address(field(NVMAP_ID)? as u32)? + field(SURFACE + 0x1c)? as u64;
        let mut surface = vec![0; len];
        mem.read_mem(address, &mut surface).ok()?;
        if block_linear {
            surface = block_linear_to_pitch(&surface, (pitch / 4) as u32, height as u32, 4, gobs as u32);
        }
        let pixels: Vec<u8> = surface.chunks(pitch).take(height).flat_map(|row| &row[..width * 4]).copied().collect();
        crate::display::present(width as u32, height as u32, &pixels);
        for s in &mut self.slots {
            if s.state == State::Presented {
                s.state = State::Free;
                self.buffer_free.set(true);
            }
        }
        self.slots[slot].state = State::Presented;
        self.vsync.set(true);
        Some(())
    }
}

impl Service for HosBinderDriver {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        let args = words(&request.args);
        if word(&args, 0) != BINDER_ID {
            return Response::error(INVALID_VALUE);
        }
        match cmd {
            // TransactParcel takes MapAlias buffers, TransactParcelAuto these or pointer buffers.
            0 | 3 => {
                let (Some(input), Some(output)) = (request.a.first().or(request.x.first()), request.b.first().or(request.c.first())) else {
                    return Response::error(INVALID_VALUE);
                };
                let mut parcel = vec![0; input.size as usize];
                if request.mem.read_mem(input.addr, &mut parcel).is_err() {
                    return Response::error(INVALID_VALUE);
                }
                let parcel = words(&parcel);
                let (size, offset) = (word(&parcel, 0) as usize, word(&parcel, 1) as usize);
                let payload = parcel.get(offset / 4..(offset + size) / 4).unwrap_or_default();
                let reply = self.transact(request.mem, word(&args, 1), payload);
                let len = 4 * reply.len() as u32;
                let bytes: Vec<u8> = [len, 16, 0, 16 + len].iter().chain(&reply).flat_map(|w| w.to_le_bytes()).collect();
                if bytes.len() as u64 > output.size || request.mem.write_mem(output.addr, &bytes).is_err() {
                    return Response::error(INVALID_VALUE);
                }
                Response::default()
            }
            1 => Response::default(),
            2 => Response { copy_handles: vec![Object::Event(self.buffer_free.clone())], ..Response::default() },
            _ => Response::unknown_command(cmd),
        }
    }
}
