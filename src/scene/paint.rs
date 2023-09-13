use crate::shape::{ComputedPaintShape, PaintShape};

#[derive(Default)]
pub struct PaintPass {
    shapes: Vec<ComputedPaintShape>,
}

impl PaintPass {
    pub fn add(&mut self, shape: impl Into<ComputedPaintShape>) {
        self.shapes.push(shape.into());
    }

    pub fn drain(self) -> impl Iterator<Item = ComputedPaintShape> {
        self.shapes.into_iter()
    }
}
