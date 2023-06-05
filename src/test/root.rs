use crate::{
    element::{Element, SizeConstraint},
    scene::{ctx::SceneContext, layout::LayoutPass},
    util::{Pos2, Size2},
};

use super::TestRect;

pub struct TestRoot {
    rects: Vec<TestRect>,
}

impl TestRoot {
    pub fn new() -> Self {
        Self {
            rects: vec![
                TestRect::new(Pos2::new(20., 20.)),
                TestRect::new(Pos2::new(40., 40.)),
                TestRect::new(Pos2::new(60., 60.)),
            ],
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        for rect in self.rects.iter_mut() {
            layout_pass.layout_child(rect, constraints);
            layout_pass.place_child(rect, Pos2::zero());
        }

        constraints.max
    }

    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        let mut send_to_front = None::<usize>;

        for (i, rect) in self.rects.iter_mut().enumerate() {
            ctx.render_child(rect);

            if rect.clicked {
                send_to_front = Some(i);
            }
        }

        if let Some(idx) = send_to_front {
            let rect = self.rects.remove(idx);
            self.rects.insert(0, rect);
        }
    }
}
