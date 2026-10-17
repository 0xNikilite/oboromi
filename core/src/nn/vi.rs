//! vi:u, vi:s and vi:m and the display services libnx nx/source/services/vi.c opens through them, with Ryujinx
//! Services/Vi's command ids: one "Default" display and one layer, whose native window parcel names the binder's
//! producer the way Ryujinx IApplicationDisplayService.OpenLayer writes it.

use crate::kernel::Object;
use crate::nn::binder::{BINDER_ID, HosBinderDriver, NvmapResolve};
use crate::nn::hipc::{Request, Response, Service};
use std::cell::RefCell;
use std::rc::Rc;

/// vi result InvalidValue, module 114 description 7.
pub const INVALID_VALUE: u32 = 114 | 7 << 9;
const LAYER_ID: u64 = 1;

#[derive(Clone, Copy)]
enum Interface {
    /// A root service and its GetDisplayService command id.
    Root(u32),
    Application,
    System,
    Manager,
}

struct Display {
    interface: Interface,
    binder: Rc<RefCell<HosBinderDriver>>,
}

/// The root services by name, all sharing one producer.
pub fn services(nvmap: Rc<dyn NvmapResolve>) -> impl Iterator<Item = (String, Rc<RefCell<dyn Service>>)> {
    let binder = Rc::new(RefCell::new(HosBinderDriver::new(nvmap)));
    ["vi:u", "vi:s", "vi:m"].into_iter().zip(0..).map(move |(name, cmd)| {
        let root: Rc<RefCell<dyn Service>> = Rc::new(RefCell::new(Display { interface: Interface::Root(cmd), binder: binder.clone() }));
        (name.into(), root)
    })
}

impl Display {
    fn session(&self, interface: Interface) -> Response {
        let display = Display { interface, binder: self.binder.clone() };
        Response { move_handles: vec![Object::Session(Rc::new(RefCell::new(display)))], ..Response::default() }
    }

    /// Writes the native window parcel to the request's out buffer and returns `prefix` and the parcel's size.
    fn layer(request: &Request, prefix: &[u64]) -> Response {
        let header = [0x20, 0x10, 4, 0x30, 2, 0, BINDER_ID, 0, 0, 0];
        let parcel: Vec<u8> = header.iter().flat_map(|w: &u32| w.to_le_bytes()).chain(*b"dispdrv\0\0\0\0\0").collect();
        match request.b.first().filter(|b| b.size >= parcel.len() as u64) {
            Some(b) if request.mem.write_mem(b.addr, &parcel).is_ok() => {
                Response { data: prefix.iter().chain(&[parcel.len() as u64]).flat_map(|v| v.to_le_bytes()).collect(), ..Response::default() }
            }
            _ => Response::error(INVALID_VALUE),
        }
    }
}

impl Service for Display {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        use Interface::*;
        match (self.interface, cmd) {
            (Root(id), _) if cmd == id => self.session(Application),
            (Application, 100 | 103) => {
                Response { move_handles: vec![Object::Session(self.binder.clone())], ..Response::default() }
            }
            (Application, 101) => self.session(System),
            (Application, 102) => self.session(Manager),
            (Application, 1010) if request.args.split(|&b| b == 0).next() == Some(b"Default") => {
                Response { data: 0u64.to_le_bytes().into(), ..Response::default() }
            }
            (Application, 1010) => Response::error(INVALID_VALUE),
            (Application, 1020 | 2021 | 2031 | 2101) | (Manager, 2011 | 6000) | (System, 2205 | 2207) => Response::default(),
            (Application, 2020) => Self::layer(request, &[]),
            (Application, 2030) | (Manager, 2012) => Self::layer(request, &[LAYER_ID]),
            (Manager, 2010) => Response { data: LAYER_ID.to_le_bytes().into(), ..Response::default() },
            (Application, 5202) => {
                let vsync = self.binder.borrow().vsync.clone();
                Response { copy_handles: vec![Object::Event(vsync)], ..Response::default() }
            }
            _ => Response::unknown_command(cmd),
        }
    }
}
