//! The hid service on the Switch 1 Npad ABI: commands from switchbrew HID_services and Ryujinx
//! Services/Hid/IHidServer.cs, shared memory from switchbrew HID_services#SharedMemoryFormat and
//! Ryujinx HidDevices/NpadDevices.cs. Switchbrew documents no Switch 2 Npad revision, so the host
//! pad is a wired FullKey controller on Npad No1.

use crate::cpu::UnicornCPU;
use crate::input::{self, InputState};
use crate::kernel::Object;
use crate::nn::hipc::{Request, Response, Service};
use crate::nn::ServiceTrait;
use crate::sys;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SHARED_MEMORY_SIZE: u64 = 0x40000;
const NPAD: u64 = 0x9A00;
const NPAD_SIZE: u64 = 0x5000;
const NPADS: u64 = 10;
/// FullKey, Handheld, JoyDual, JoyLeft, JoyRight, Palma and SystemExt.
const LIFOS: [u64; 7] = [0x28, 0x378, 0x6C8, 0xA18, 0xD68, 0x10B8, 0x1408];
const RECORDS: u64 = 17;
const RECORD_SIZE: u64 = 0x30;
const FULL_KEY: u32 = 1;
const CONNECTED_WIRED: u32 = 0b11;
pub const ARUID: u64 = 0;

pub struct State {
    pub input: fn() -> InputState,
    shared: Rc<Cell<Option<u64>>>,
    style_set: u32,
    ids: Vec<u32>,
    sampling: u64,
}

impl Default for State {
    fn default() -> Self {
        Self { input: || input::state().0, shared: Rc::default(), style_set: 0, ids: vec![], sampling: 0 }
    }
}

impl ServiceTrait for State {
    fn run(state: &mut sys::State) {
        state.services.hid = Some(State::default());
    }
}

impl State {
    pub fn tick(&mut self, cpu: &UnicornCPU) {
        let input = (self.input)();
        self.write(cpu, input);
    }

    /// Appends one record to every Npad lifo, the input to No1's FullKey and zero states elsewhere.
    fn write(&mut self, cpu: &UnicornCPU, input: InputState) {
        let Some(base) = self.shared.get() else { return };
        let connected = input.connected && self.style_set & FULL_KEY != 0 && self.ids.contains(&0);
        self.sampling += 1;
        let tail = self.sampling % RECORDS;
        let stick = |v: f32| ((v * 0x7fff as f32) as i32).clamp(-0x7fff, 0x7fff);
        for npad in 0..NPADS {
            let entry = base + NPAD + npad * NPAD_SIZE;
            let pad = npad == 0 && connected;
            cpu.write_u32(entry, if pad { FULL_KEY } else { 0 });
            for (i, lifo) in LIFOS.iter().map(|offset| entry + offset).enumerate() {
                let (state, attributes) = if pad && i == 0 { (input, CONNECTED_WIRED) } else { (InputState::default(), 0) };
                let mut record = [self.sampling, self.sampling, state.buttons].map(u64::to_le_bytes).concat();
                record.extend(state.left.into_iter().chain(state.right).flat_map(|v| stick(v).to_le_bytes()));
                record.extend([attributes, 0].map(u32::to_le_bytes).concat());
                let _ = cpu.write_mem(lifo + 0x20 + tail * RECORD_SIZE, &record);
                cpu.write_u64(lifo + 8, RECORDS);
                cpu.write_u64(lifo + 0x10, tail);
                cpu.write_u64(lifo + 0x18, self.sampling.min(RECORDS - 1));
            }
        }
    }
}

fn u32_arg(request: &Request) -> u32 {
    request.args.get(..4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn session(service: impl Service + 'static) -> Rc<RefCell<dyn Service>> {
    Rc::new(RefCell::new(service))
}

impl Service for State {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        let ok = Response::default();
        match cmd {
            0 => Response { objects: vec![session(AppletResource(self.shared.clone()))], ..ok },
            100 => {
                self.style_set = u32_arg(request);
                ok
            }
            101 => Response { data: self.style_set.to_le_bytes().into(), ..ok },
            102 => {
                let Some(ids) = request.x.first() else { return Response::unknown_command(cmd) };
                let mut bytes = vec![0; ids.size as usize];
                let _ = request.mem.read_mem(ids.addr, &mut bytes);
                self.ids = bytes.chunks_exact(4).map(|id| u32::from_le_bytes(id.try_into().unwrap())).collect();
                ok
            }
            103 | 109 => {
                (0..RECORDS).for_each(|_| self.write(request.mem, InputState::default()));
                ok
            }
            11 | 21 | 31 | 66 | 67 | 91 | 120 | 201 | 204 | 206 => ok,
            106 => Response { copy_handles: vec![Object::Event(Rc::new(Cell::new(true)))], ..ok },
            // LinearResonantActuator, no position.
            200 => Response { data: vec![1, 0, 0, 0, 0, 0, 0, 0], ..ok },
            203 => Response { objects: vec![session(ActiveVibrationDeviceList)], ..ok },
            205 => Response { data: vec![1], ..ok },
            _ => Response::unknown_command(cmd),
        }
    }
}

struct AppletResource(Rc<Cell<Option<u64>>>);

impl Service for AppletResource {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 => {
                let shared = Object::SharedMemory { size: SHARED_MEMORY_SIZE, mapped: self.0.clone() };
                Response { copy_handles: vec![shared], ..Response::default() }
            }
            _ => Response::unknown_command(cmd),
        }
    }
}

struct ActiveVibrationDeviceList;

impl Service for ActiveVibrationDeviceList {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 => Response::default(),
            _ => Response::unknown_command(cmd),
        }
    }
}
