use super::{
    compute_class::{CLASS_ID as COMPUTE_CLASS_ID, ComputeClass, DispatchSnapshot},
    threed::{CLASS_ID as THREED_CLASS_ID, DrawSnapshot, ThreeD, ThreeDEvent},
};

const SET_OBJECT: usize = 0x00;
const SEMAPHORE_A: usize = 0x10;
const SEMAPHORE_B: usize = 0x14;
const SEMAPHORE_C: usize = 0x18;
const SEMAPHORE_D: usize = 0x1c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpFifoEntry {
    pub address: u64,
    pub length: u32,
    pub level: bool,
    pub sync: bool,
}

impl GpFifoEntry {
    pub fn decode(bytes: [u8; 8]) -> Self {
        let lo = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let hi = u32::from_le_bytes(bytes[4..].try_into().unwrap());
        Self {
            address: (lo & !3) as u64 | ((hi & 0xff) as u64) << 32,
            length: (hi >> 10) & 0x1f_ffff,
            level: hi & (1 << 9) != 0,
            sync: hi >> 31 != 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Draw(DrawSnapshot),
    Dispatch(DispatchSnapshot),
    Report { addr: u64, payload: u32 },
    SemaphoreAcquire { addr: u64, payload: u32 },
    SemaphoreRelease { addr: u64, payload: u32 },
    Clear { value: u32 },
}

#[derive(Clone, Copy)]
enum Mode {
    Increment,
    NonIncrement,
    OneIncrement,
}

#[derive(Clone, Copy)]
struct Packet {
    method: u16,
    subchannel: usize,
    remaining: u32,
    consumed: u32,
    mode: Mode,
}

#[derive(Default)]
pub struct Pusher {
    packet: Option<Packet>,
    classes: [Option<u16>; 8],
    semaphore: [u32; 3],
    compute: ComputeClass,
    threed: ThreeD,
}

impl Pusher {
    pub fn threed(&self) -> &ThreeD {
        &self.threed
    }

    pub fn threed_mut(&mut self) -> &mut ThreeD {
        &mut self.threed
    }

    pub fn submit<'a, I, F, W>(
        &mut self,
        entries: I,
        mut fetch: F,
        mut write_memory: W,
    ) -> Result<Vec<Event>, String>
    where
        I: IntoIterator<Item = [u8; 8]>,
        F: FnMut(u64, usize) -> Result<&'a [u8], String>,
        W: FnMut(u64, &[u8]) -> Result<(), String>,
    {
        let mut events = Vec::new();
        for encoded in entries {
            let entry = GpFifoEntry::decode(encoded);
            let bytes = fetch(entry.address, entry.length as usize * 4)?;
            if bytes.len() != entry.length as usize * 4 {
                return Err(format!(
                    "GPFIFO fetch returned {} bytes, expected {}",
                    bytes.len(),
                    entry.length * 4
                ));
            }
            let mut words = bytes.chunks_exact(4);
            while let Some(bytes) = words.next() {
                let word = u32::from_le_bytes(bytes.try_into().unwrap());
                if self.packet.is_some() {
                    self.data(word, &mut fetch, &mut write_memory, &mut events)?;
                } else if !self.header(word, &mut fetch, &mut write_memory, &mut events)? {
                    break;
                }
            }
        }
        Ok(events)
    }

    fn header<'a>(
        &mut self,
        word: u32,
        fetch: &mut impl FnMut(u64, usize) -> Result<&'a [u8], String>,
        write_memory: &mut impl FnMut(u64, &[u8]) -> Result<(), String>,
        events: &mut Vec<Event>,
    ) -> Result<bool, String> {
        let sec = word >> 29;
        let method = (word & 0xfff) as u16;
        let subchannel = ((word >> 13) & 7) as usize;
        let mode = match sec {
            0 if (word >> 16) & 3 == 0 => Mode::Increment,
            0 => {
                return Err(format!(
                    "unknown GPFIFO group-0 tertiary opcode {}",
                    (word >> 16) & 3
                ))
            }
            1 => Mode::Increment,
            2 if (word >> 16) & 3 == 0 => Mode::NonIncrement,
            2 => {
                return Err(format!(
                    "unknown GPFIFO group-2 tertiary opcode {}",
                    (word >> 16) & 3
                ))
            }
            3 => Mode::NonIncrement,
            4 => {
                self.dispatch(
                    method,
                    subchannel,
                    (word >> 16) & 0x1fff,
                    fetch,
                    write_memory,
                    events,
                )?;
                return Ok(true);
            }
            5 => Mode::OneIncrement,
            6 => return Err("unknown GPFIFO secondary opcode 6".into()),
            7 => return Ok(false),
            _ => unreachable!(),
        };
        let count = if matches!(sec, 0 | 2) {
            (word >> 18) & 0x7ff
        } else {
            (word >> 16) & 0x1fff
        };
        if count != 0 {
            self.packet = Some(Packet {
                method,
                subchannel,
                remaining: count,
                consumed: 0,
                mode,
            });
        }
        Ok(true)
    }

    fn data<'a>(
        &mut self,
        value: u32,
        fetch: &mut impl FnMut(u64, usize) -> Result<&'a [u8], String>,
        write_memory: &mut impl FnMut(u64, &[u8]) -> Result<(), String>,
        events: &mut Vec<Event>,
    ) -> Result<(), String> {
        let mut packet = self.packet.unwrap();
        let method = match packet.mode {
            Mode::Increment => packet.method + packet.consumed as u16,
            Mode::NonIncrement => packet.method,
            Mode::OneIncrement => packet.method + u16::from(packet.consumed != 0),
        };
        self.dispatch(method, packet.subchannel, value, fetch, write_memory, events)?;
        packet.remaining -= 1;
        packet.consumed += 1;
        self.packet = (packet.remaining != 0).then_some(packet);
        Ok(())
    }

    fn dispatch<'a>(
        &mut self,
        method: u16,
        subchannel: usize,
        value: u32,
        fetch: &mut impl FnMut(u64, usize) -> Result<&'a [u8], String>,
        write_memory: &mut impl FnMut(u64, &[u8]) -> Result<(), String>,
        events: &mut Vec<Event>,
    ) -> Result<(), String> {
        if method < 0x40 {
            return self.channel(method as usize * 4, subchannel, value, events);
        }
        let class = self.classes[subchannel].ok_or_else(|| {
            format!(
                "unbound subchannel {subchannel} method {:#06x}",
                method as usize * 4
            )
        })?;
        if class == COMPUTE_CLASS_ID {
            if let Some(dispatch) = self.compute.write(method, value, fetch)? {
                events.push(Event::Dispatch(dispatch));
            }
        } else if class == THREED_CLASS_ID {
            if let Some(event) = self.threed.write(method, value, write_memory)? {
                events.push(match event {
                    ThreeDEvent::Draw(draw) => Event::Draw(draw),
                    ThreeDEvent::Report { addr, payload } => Event::Report { addr, payload },
                    ThreeDEvent::Clear { value } => Event::Clear { value },
                });
            }
        } else {
            return Err(format!(
                "unsupported class method: class {class:#06x}, method {:#06x}",
                method as usize * 4
            ));
        }
        Ok(())
    }

    fn channel(
        &mut self,
        method: usize,
        subchannel: usize,
        value: u32,
        events: &mut Vec<Event>,
    ) -> Result<(), String> {
        match method {
            SET_OBJECT => self.classes[subchannel] = Some(value as u16),
            SEMAPHORE_A | SEMAPHORE_B | SEMAPHORE_C => {
                self.semaphore[(method - SEMAPHORE_A) / 4] = value
            }
            SEMAPHORE_D => {
                let addr =
                    ((self.semaphore[0] & 0xff) as u64) << 32 | (self.semaphore[1] & !3) as u64;
                let payload = self.semaphore[2];
                match value & 0x1f {
                    1 => events.push(Event::SemaphoreAcquire { addr, payload }),
                    2 => events.push(Event::SemaphoreRelease { addr, payload }),
                    operation => {
                        return Err(format!(
                            "unsupported channel semaphore operation {operation:#x}"
                        ))
                    }
                }
            }
            _ => return Err(format!("channel method {method:#04x} not implemented")),
        }
        Ok(())
    }
}
