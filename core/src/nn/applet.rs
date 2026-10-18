//! Applet Manager application proxy services, switchbrew Applet_Manager_services.

use crate::kernel::Object;
use crate::nn::ServiceTrait;
use crate::nn::hipc::{Request, Response, Service};
use crate::sys;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const NO_MESSAGE: u32 = 0x680;

pub struct State;

impl ServiceTrait for State {
    fn run(state: &mut sys::State) {
        state.services.applet_oe = Some(Self);
        state.services.applet_ae = Some(Self);
    }
}

fn session(service: impl Service + 'static) -> Rc<RefCell<dyn Service>> {
    Rc::new(RefCell::new(service))
}

impl Service for State {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 | 100 | 200 | 201 | 300 | 350 => Response { objects: vec![session(ApplicationProxy)], ..Response::default() },
            _ => Response::not_implemented("appletOE", cmd),
        }
    }
}

struct ApplicationProxy;

impl Service for ApplicationProxy {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        let service = match cmd {
            0 => session(CommonStateGetter(true)),
            1 => session(SelfController),
            2 => session(WindowController),
            3 => session(Empty("IAudioController")),
            4 => session(Empty("IDisplayController")),
            11 => session(Empty("ILibraryAppletCreator")),
            20 => session(ApplicationFunctions),
            1000 => session(Empty("IDebugFunctions")),
            _ => return Response::not_implemented("IApplicationProxy", cmd),
        };
        Response { objects: vec![service], ..Response::default() }
    }
}

struct CommonStateGetter(bool);

impl Service for CommonStateGetter {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            0 => Response { copy_handles: vec![Object::Event(Rc::new(Cell::new(true)))], ..Response::default() },
            1 if self.0 => {
                self.0 = false;
                Response { data: 15u32.to_le_bytes().into(), ..Response::default() }
            }
            1 => Response::error(NO_MESSAGE),
            5 | 9 => Response { data: vec![u8::from(cmd == 9)], ..Response::default() },
            6 => Response { data: 0u32.to_le_bytes().into(), ..Response::default() },
            60 => Response { data: [1920u32.to_le_bytes(), 1080u32.to_le_bytes()].concat(), ..Response::default() },
            _ => Response::not_implemented("ICommonStateGetter", cmd),
        }
    }
}

struct SelfController;

impl Service for SelfController {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            9 => Response { copy_handles: vec![Object::Event(Rc::new(Cell::new(true)))], ..Response::default() },
            10 | 11 | 12 | 13 | 14 | 16 => Response::default(),
            40 => Response { data: crate::nn::vi::LAYER_ID.to_le_bytes().into(), ..Response::default() },
            _ => Response::not_implemented("ISelfController", cmd),
        }
    }
}

struct WindowController;

impl Service for WindowController {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            1 => Response { data: crate::nn::hid::ARUID.to_le_bytes().into(), ..Response::default() },
            10 => Response::default(),
            _ => Response::not_implemented("IWindowController", cmd),
        }
    }
}

struct ApplicationFunctions;

impl Service for ApplicationFunctions {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            20 => Response { data: 0u64.to_le_bytes().into(), ..Response::default() },
            21 => Response { data: u64::from_le_bytes(*b"en-US\0\0\0").to_le_bytes().into(), ..Response::default() },
            23 => Response { data: b"1.0.0\0\0\0\0\0\0\0\0\0\0\0".to_vec(), ..Response::default() },
            26 => Response { data: vec![0; 16], ..Response::default() },
            40 => Response { data: vec![1], ..Response::default() },
            66 | 67 => Response::default(),
            _ => Response::not_implemented("IApplicationFunctions", cmd),
        }
    }
}

struct Empty(&'static str);

impl Service for Empty {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        Response::not_implemented(self.0, cmd)
    }
}
