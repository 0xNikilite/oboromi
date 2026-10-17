//! The service manager behind the `sm:` port: RegisterClient and GetService (switchbrew Services_API).

use crate::kernel::Object;
use crate::nn::hipc::{Request, Response, Service};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// sm result NotRegistered, module 21 description 7.
const NOT_REGISTERED: u32 = 21 | 7 << 9;

pub type Services = HashMap<String, Rc<RefCell<dyn Service>>>;

pub struct Sm(pub Rc<RefCell<Services>>);

impl Service for Sm {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        match cmd {
            0 => Response::default(),
            1 => {
                let name = request.args.get(..8).unwrap_or_default();
                let name = String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or_default());
                match self.0.borrow().get(name.as_ref()) {
                    Some(service) => Response { move_handles: vec![Object::Session(service.clone())], ..Response::default() },
                    None => Response::error(NOT_REGISTERED),
                }
            }
            _ => Response::unknown_command(cmd),
        }
    }
}
