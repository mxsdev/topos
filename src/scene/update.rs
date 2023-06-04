#[derive(Default)]
pub struct UpdatePass {
    hover_consumed: bool,
}

impl UpdatePass {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn consume_hover(&mut self) {
        self.hover_consumed = true;
    }

    pub(super) fn hover_consumed(&self) -> bool {
        self.hover_consumed
    }
}
