//! audren:u and IAudioRenderer (switchbrew Audio_services, libnx audren.c).
use crate::audio::{self, Error, GuestMemory, renderer::{AudioRendererParameter, Renderer, work_buffer_size}};
use crate::{cpu::Cpu, kernel::Object, nn::hipc::{Request, Response, Service}};
use std::{cell::{Cell, RefCell}, rc::Rc};

pub struct State;
impl crate::nn::ServiceTrait for State { fn run(state: &mut crate::sys::State) { state.services.audren = Some(State); } }
pub struct Manager;
fn word(bytes: &[u8], at: usize) -> u32 { u32::from_le_bytes(bytes[at..at+4].try_into().unwrap()) }
impl Service for Manager {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        if !matches!(cmd, 0 | 1) { return Response::unknown_command(cmd); }
        let result = (|| {
            let b = request.args.get(..0x34).ok_or(Error::InvalidUpdateInfo)?;
            let parameter = AudioRendererParameter { sample_rate: word(b,0), sample_count: word(b,4), mix_buffer_count: word(b,8), sub_mix_count: word(b,12), voice_count: word(b,16), sink_count: word(b,20), effect_count: word(b,24), performance_frame_count: word(b,28), splitter_count: word(b,36), splitter_destination_count: word(b,40), revision: word(b,48) };
            let required = work_buffer_size(&parameter)?;
            if cmd == 1 { return Ok(Response { data: required.to_le_bytes().into(), ..Default::default() }); }
            let size = request.args.get(0x38..0x40).map(|b| u64::from_le_bytes(b.try_into().unwrap())).ok_or(Error::InvalidUpdateInfo)?;
            let &(address, available, perm) = request.transfers.first().ok_or(Error::InsufficientBuffer)?;
            if size < required || size > available || perm != 0 || !request.send_pid || request.copy_handles.get(1) != Some(&crate::kernel::CURRENT_PROCESS) { return Err(Error::InsufficientBuffer); }
            let mut work = vec![0; size as usize];
            request.mem.read_mem(address, &mut work).map_err(|_| Error::InsufficientBuffer)?;
            let renderer = Renderer::open(parameter, &mut work)?;
            request.mem.write_mem(address, &work).map_err(|_| Error::InsufficientBuffer)?;
            Ok(Response { objects: vec![Rc::new(RefCell::new(AudioRenderer { renderer, parameter, next: u64::MAX, event: Rc::new(Cell::new(false)) }))], ..Default::default() })
        })();
        result.unwrap_or_else(|e: Error| Response::error(e.result()))
    }
}
struct Memory<'a>(&'a dyn Cpu);
impl GuestMemory for Memory<'_> {
    fn read(&self, va: u64, out: &mut [u8]) { if self.0.read_mem(va, out).is_err() { out.fill(0); } }
    fn write(&mut self, va: u64, data: &[u8]) { let _ = self.0.write_mem(va, data); }
}
struct AudioRenderer { renderer: Renderer, parameter: AudioRendererParameter, next: u64, event: Rc<Cell<bool>> }
impl Service for AudioRenderer {
    fn deadline(&self) -> Option<u64> { Some(self.next) }
    fn advance(&mut self, cpu: &dyn Cpu) {
        while cpu.ticks() >= self.next {
            audio::play(&[self.renderer.step(&mut Memory(cpu))]);
            self.event.set(true);
            self.next += crate::cpu::unicorn_interface::TICK_HZ / 200;
        }
    }
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        match cmd {
            0..=3 => Response { data: match cmd { 0 => self.parameter.sample_rate, 1 => self.parameter.sample_count, 2 => self.parameter.mix_buffer_count, _ => u32::from(self.next == u64::MAX) }.to_le_bytes().into(), ..Default::default() },
            4 => {
                let result = (|| {
                    let input = request.a.first().ok_or(Error::InvalidUpdateInfo)?;
                    let output = request.b.first().ok_or(Error::InsufficientBuffer)?;
                    if request.b.len() != 2 { return Err(Error::InsufficientBuffer); }
                    let mut bytes = vec![0; input.size as usize];
                    request.mem.read_mem(input.addr, &mut bytes).map_err(|_| Error::InvalidUpdateInfo)?;
                    let mut reply = vec![0; output.size as usize];
                    self.renderer.request_update(&bytes, &mut reply)?;
                    request.mem.write_mem(output.addr, &reply).map_err(|_| Error::InsufficientBuffer)?;
                    Ok(Response::default())
                })();
                result.unwrap_or_else(|e: Error| Response::error(e.result()))
            }
            5 => { self.renderer.start(); if self.next == u64::MAX { self.next = request.mem.ticks() + crate::cpu::unicorn_interface::TICK_HZ / 200; } Response::default() }
            6 => { self.renderer.stop(); self.next = u64::MAX; self.event.set(false); Response::default() }
            7 => Response { copy_handles: vec![Object::Event(self.event.clone())], ..Default::default() },
            _ => Response::unknown_command(cmd),
        }
    }
}
