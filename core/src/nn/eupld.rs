use crate::sys;
use crate::nn::ServiceTrait;
pub struct State {

}
impl State {
    pub fn new(_state: &mut sys::State) -> Self {
        Self{}
    }
}
impl ServiceTrait for State {
     fn run(state: &mut sys::State) {
        state.services.eupld = Some(State::new(state));
    }
}
