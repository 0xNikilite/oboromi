//! Time services, switchbrew PSC_services#time.

use crate::kernel::Object;
use crate::nn::ServiceTrait;
use crate::nn::hipc::{Request, Response, Service};
use crate::sys;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct State;

impl ServiceTrait for State {
    fn run(state: &mut sys::State) {
        state.services.time = Some(Self);
    }
}

fn session(service: impl Service + 'static) -> Rc<RefCell<dyn Service>> {
    Rc::new(RefCell::new(service))
}

fn unix_time() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn steady_time() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs() as i64
}

impl Service for State {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        let service = match cmd {
            0 | 1 | 4 => session(SystemClock),
            2 => session(SteadyClock),
            3 => session(TimeZone),
            20 => {
                let shared = Object::SharedMemory { size: 0x1000, mapped: Rc::new(Cell::new(None)) };
                return Response { copy_handles: vec![shared], ..Response::default() };
            }
            _ => return Response::not_implemented("time", cmd),
        };
        Response { objects: vec![service], ..Response::default() }
    }
}

struct SystemClock;

impl Service for SystemClock {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 => Response { data: unix_time().to_le_bytes().into(), ..Response::default() },
            2 => {
                let data = [unix_time().to_le_bytes().as_slice(), &steady_point()].concat();
                Response { data, ..Response::default() }
            }
            _ => Response::not_implemented("ISystemClock", cmd),
        }
    }
}

fn steady_point() -> [u8; 24] {
    let mut point = [0; 24];
    point[..8].copy_from_slice(&steady_time().to_le_bytes());
    point[8..].copy_from_slice(b"oboromi-time\0\0\0\0");
    point
}

struct SteadyClock;

impl Service for SteadyClock {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 => Response { data: steady_point().into(), ..Response::default() },
            _ => Response::not_implemented("ISteadyClock", cmd),
        }
    }
}

struct TimeZone;

impl Service for TimeZone {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        match cmd {
            0 => {
                let mut name = vec![0; 0x24];
                name[..3].copy_from_slice(b"UTC");
                Response { data: name, ..Response::default() }
            }
            101 => {
                let seconds = request.args.get(..8).map_or(0, |b| i64::from_le_bytes(b.try_into().unwrap()));
                Response { data: calendar(seconds), ..Response::default() }
            }
            _ => Response::not_implemented("ITimeZoneService", cmd),
        }
    }
}

fn calendar(seconds: i64) -> Vec<u8> {
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let prior = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334][month as usize - 1] + i64::from(leap && month > 2);
    let mut out = Vec::with_capacity(32);
    out.extend((year as i16).to_le_bytes());
    out.extend([month as u8, day as u8, (day_seconds / 3600) as u8, (day_seconds / 60 % 60) as u8, (day_seconds % 60) as u8, 0]);
    out.extend(((days + 4).rem_euclid(7) as u32).to_le_bytes());
    out.extend(((prior + day - 1) as u32).to_le_bytes());
    out.extend(*b"UTC\0\0\0\0\0");
    out.extend([0; 4]);
    out.extend(0i32.to_le_bytes());
    out
}
