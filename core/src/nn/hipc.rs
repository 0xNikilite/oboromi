//! HIPC messages with CMIF payloads following switchbrew IPC_Marshalling.

use crate::cpu::UnicornCPU;
use crate::kernel::Object;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const MESSAGE_SIZE: usize = 0x100;
pub const CLOSE: u16 = 2;
pub const REQUEST: u16 = 4;
pub const CONTROL: u16 = 5;
pub const REQUEST_WITH_CONTEXT: u16 = 6;
pub const CONTROL_WITH_CONTEXT: u16 = 7;
const SFCI: u32 = u32::from_le_bytes(*b"SFCI");
const SFCO: u32 = u32::from_le_bytes(*b"SFCO");

/// sf result descriptions (module 10), Ryujinx.Horizon/Sdk/Sf/SfResult.cs.
#[derive(Debug, Clone, Copy)]
pub enum SfError {
    InvalidHeaderSize = 202,
    InvalidInHeader = 211,
    UnknownCommandId = 221,
}

impl SfError {
    pub fn code(self) -> u32 {
        10 | (self as u32) << 9
    }
}

pub struct Buffer {
    pub addr: u64,
    pub size: u64,
}

pub struct Request<'a> {
    pub mem: &'a UnicornCPU,
    pub send_pid: bool,
    pub copy_handles: Vec<u32>,
    pub move_handles: Vec<u32>,
    pub x: Vec<Buffer>,
    pub a: Vec<Buffer>,
    pub b: Vec<Buffer>,
    pub w: Vec<Buffer>,
    pub c: Vec<Buffer>,
    pub objects: Vec<u32>,
    /// Raw words after the SFCI header, up to the end of the raw data section.
    pub args: Vec<u8>,
}

#[derive(Default)]
pub struct Response {
    pub result: u32,
    pub data: Vec<u8>,
    pub copy_handles: Vec<Object>,
    pub move_handles: Vec<Object>,
    pub objects: Vec<Rc<RefCell<dyn Service>>>,
}

impl Response {
    pub fn error(result: u32) -> Self {
        Self { result, ..Self::default() }
    }

    pub fn unknown_command(cmd: u32) -> Self {
        log::warn!("command {cmd}: {:?}", SfError::UnknownCommandId);
        Self::error(SfError::UnknownCommandId.code())
    }
}

pub trait Service {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response;
    /// Periodic device signals use guest ticks, so waits can shorten CPU slices.
    fn event_period(&self, _event: &Rc<Cell<bool>>) -> Option<u64> { None }
}

pub enum ParsedRequest<'a> {
    Invoke { command: u32, object: Option<u32>, request: Request<'a> },
    Close(u32),
}

pub fn command_type(message: &[u8; MESSAGE_SIZE]) -> u16 {
    u16::from_le_bytes([message[0], message[1]])
}

struct Words<'m> {
    words: &'m [u32],
    at: usize,
}

impl<'m> Words<'m> {
    fn take(&mut self, n: usize) -> Result<&'m [u32], SfError> {
        let taken = self.words.get(self.at..self.at + n).ok_or(SfError::InvalidHeaderSize)?;
        self.at += n;
        Ok(taken)
    }

    fn buffers(&mut self, n: u32, words: usize, decode: fn(&[u32]) -> Buffer) -> Result<Vec<Buffer>, SfError> {
        (0..n).map(|_| self.take(words).map(decode)).collect()
    }
}

fn bits(value: u32, at: u32, count: u32) -> u32 {
    value >> at & ((1 << count) - 1)
}

fn static_buffer(d: &[u32]) -> Buffer {
    let addr = u64::from(d[1]) | u64::from(bits(d[0], 12, 4)) << 32 | u64::from(bits(d[0], 6, 3)) << 36;
    Buffer { addr, size: (d[0] >> 16).into() }
}

fn mapped_buffer(d: &[u32]) -> Buffer {
    let addr = u64::from(d[1]) | u64::from(bits(d[2], 28, 4)) << 32 | u64::from(bits(d[2], 2, 3)) << 36;
    Buffer { addr, size: u64::from(d[0]) | u64::from(bits(d[2], 24, 4)) << 32 }
}

fn receive_buffer(d: &[u32]) -> Buffer {
    Buffer { addr: u64::from(d[0]) | u64::from(d[1] & 0xffff) << 32, size: (d[1] >> 16).into() }
}

impl<'a> Request<'a> {
    /// Decodes a CMIF request into its command id and arguments.
    pub fn parse(message: &[u8; MESSAGE_SIZE], mem: &'a UnicornCPU, domain: bool) -> Result<ParsedRequest<'a>, SfError> {
        let words: Vec<u32> = message.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        let (h0, h1) = (words[0], words[1]);
        let mut r = Words { words: &words, at: 2 };
        let (mut send_pid, mut copy_handles, mut move_handles) = (false, vec![], vec![]);
        if h1 >> 31 != 0 {
            let d = r.take(1)?[0];
            send_pid = d & 1 != 0;
            if send_pid {
                r.take(2)?;
            }
            copy_handles = r.take(bits(d, 1, 4) as usize)?.to_vec();
            move_handles = r.take(bits(d, 5, 4) as usize)?.to_vec();
        }
        let x = r.buffers(bits(h0, 16, 4), 2, static_buffer)?;
        let a = r.buffers(bits(h0, 20, 4), 3, mapped_buffer)?;
        let b = r.buffers(bits(h0, 24, 4), 3, mapped_buffer)?;
        let w = r.buffers(bits(h0, 28, 4), 3, mapped_buffer)?;
        let start = r.at;
        let raw = r.take(bits(h1, 0, 10) as usize)?;
        let c_count = match bits(h1, 10, 4) {
            0 | 1 => 0,
            2 => 1,
            n => n - 2,
        };
        let c = r.buffers(c_count, 2, receive_buffer)?;
        let payload = raw.get(start.next_multiple_of(4) - start..).ok_or(SfError::InvalidHeaderSize)?;
        let (cmif, object, objects) = if domain {
            let header = payload.get(..4).ok_or(SfError::InvalidHeaderSize)?;
            let kind = header[0] & 0xff;
            let count = bits(header[0], 8, 8) as usize;
            let size = bits(header[0], 16, 16).div_ceil(4) as usize;
            if kind == 2 { return Ok(ParsedRequest::Close(header[1])); }
            if kind != 1 || size < 4 { return Err(SfError::InvalidInHeader); }
            let cmif = payload.get(4..4 + size).ok_or(SfError::InvalidHeaderSize)?;
            let objects = payload.get(4 + size..4 + size + count).ok_or(SfError::InvalidHeaderSize)?.to_vec();
            (cmif, Some(header[1]), objects)
        } else {
            (payload, None, vec![])
        };
        if cmif.len() < 4 { return Err(SfError::InvalidHeaderSize); }
        if cmif[0] != SFCI {
            return Err(SfError::InvalidInHeader);
        }
        let args = cmif[4..].iter().flat_map(|w| w.to_le_bytes()).collect();
        Ok(ParsedRequest::Invoke { command: cmif[2], object, request: Self { mem, send_pid, copy_handles, move_handles, x, a, b, w, c, objects, args } })
    }
}

pub fn encode_reply(result: u32, data: &[u8], copy: &[u32], moved: &[u32], objects: Option<&[u32]>) -> Vec<u8> {
    let handles = !copy.is_empty() || !moved.is_empty();
    let mut words = vec![0, 0];
    if handles {
        words.push((copy.len() << 1 | moved.len() << 5) as u32);
        words.extend(copy.iter().chain(moved));
    }
    let start = words.len();
    words.resize(start.next_multiple_of(4), 0);
    if let Some(objects) = objects {
        words.extend([objects.len() as u32, 0, 0, 0]);
    }
    words.extend([SFCO, 0, result, 0]);
    words.extend(data.chunks(4).map(|c| {
        let mut word = [0; 4];
        word[..c.len()].copy_from_slice(c);
        u32::from_le_bytes(word)
    }));
    words.extend(objects.into_iter().flatten());
    words[1] = (words.len() - start) as u32 | u32::from(handles) << 31;
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    assert!(bytes.len() <= MESSAGE_SIZE, "reply overflows the IPC buffer");
    bytes
}
