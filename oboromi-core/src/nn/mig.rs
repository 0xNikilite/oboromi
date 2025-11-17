use crate::sys;
use crate::nn::ServiceTrait;
pub struct State {

}
impl State {
    pub fn new(state: &mut sys::State) -> Self {
        Self{}
    }
}
impl ServiceTrait for State {
     fn run(state: &mut sys::State) {
        state.services.mig = Some(State::new(state));
    }
}
