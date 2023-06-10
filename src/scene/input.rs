use crate::input::input_state::InputState;

pub trait HasInput {
    fn input(&mut self) -> &mut InputState;
}

impl HasInput for InputState {
    fn input(&mut self) -> &mut InputState {
        self
    }
}
