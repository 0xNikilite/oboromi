use crate::nn::ServiceTrait;
use crate::sys;
pub struct State {}
impl State {
    pub fn new(_state: &mut sys::State) -> Self {
        Self {}
    }
}
impl ServiceTrait for State {
    fn run(state: &mut sys::State) {
        state.services.pcv = Some(State::new(state));
    }
}
