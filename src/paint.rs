use crate::shape::PaintShape;

#[derive(Default)]
pub struct ScenePainter {
    shapes: Vec<PaintShape>,
}

impl ScenePainter {
    pub fn add(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into());
    }

    pub fn drain(self) -> impl Iterator<Item = PaintShape> {
        self.shapes.into_iter()
    }
}
