use crate::shape::PaintShape;

#[derive(Default)]
pub struct PaintPass {
    shapes: Vec<PaintShape>,
}

impl PaintPass {
    pub fn add(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into());
    }

    pub fn drain(self) -> impl Iterator<Item = PaintShape> {
        self.shapes.into_iter()
    }
}

#[derive(Default)]
pub struct DepthIterator {
    curr: f32,
}

impl DepthIterator {
    pub(super) fn new() {
        Default::default()
    }

    pub fn next(&mut self) -> f32 {
        let mut next = self.curr.next_up();
        std::mem::swap(&mut self.curr, &mut next);
        return next;
    }
}
