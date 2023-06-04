use std::{cell::RefCell, ops::Deref, rc::Rc};

use euclid::{default, Translation2D};

use crate::{
    element::{Element, SizeConstraint},
    shape::PaintShape,
    util::{Pos2, Size2, Translate2DMut},
};

use super::layout::ElementPlacement;

pub(super) struct SceneContextInternal {
    placement: ElementPlacement,
}

impl SceneContextInternal {
    pub fn new(placement: ElementPlacement) -> Self {
        Self { placement }
    }

    pub fn set_placement(&mut self, placement: ElementPlacement) {
        self.placement = placement
    }
}

impl Default for SceneContextInternal {
    fn default() -> Self {
        Self {
            placement: Default::default(),
        }
    }
}

pub struct SceneContext {
    internal: Rc<RefCell<SceneContextInternal>>,
    shapes: Vec<PaintShape>,
}

pub struct ChildUI {
    shapes: Vec<PaintShape>,
    pub size: Size2,
}

impl Clone for SceneContext {
    fn clone(&self) -> Self {
        Self {
            internal: self.internal.clone(),
            shapes: Default::default(),
        }
    }
}

impl SceneContext {
    pub(super) fn new(internal: Rc<RefCell<SceneContextInternal>>) -> Self {
        Self {
            internal,
            shapes: Default::default(),
        }
    }

    pub(super) fn drain(self) -> impl Iterator<Item = PaintShape> {
        self.shapes.into_iter()
    }

    pub fn add_shape(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into())
    }

    pub fn render_child(&mut self, element: &mut impl Element, constraint: SizeConstraint) {
        let mut ctx = self.clone();

        let placement = self.internal.borrow().placement.get(&element.id());

        if let Some(pos) = placement {
            let size = element.ui(&mut ctx, *pos);
            self.shapes.extend(ctx.shapes.into_iter());
        }

        // ChildUI { shapes, size }
    }

    // pub fn place_child(&mut self, pos: Pos2, child: ChildUI) {
    //     let ChildUI { shapes, size } = child;

    //     self.shapes.extend(shapes.into_iter().map(|mut z| {
    //         z.translate_mut(pos.x, pos.y);
    //         z
    //     }));
    // }
}

// impl Deref for SceneContext {
//     type Target = SceneContextInternal;

//     fn deref(&self) -> &Self::Target {
//         self.scene
//     }
// }
