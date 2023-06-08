use std::{
    cell::{RefCell, RefMut},
    ops::Deref,
    rc::Rc,
};

use euclid::{default, Translation2D};

use crate::{
    element::{Element, SizeConstraint},
    input::input_state::InputState,
    shape::PaintShape,
    util::{Pos2, Size2, Translate2DMut},
};

use super::{layout::ElementPlacement, scene};

// #[derive(Default)]
// pub(super) struct SceneContextInternal {
//     input: InputState,
// }

// impl SceneContextInternal {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn set_input(&mut self, input: InputState) {
//         self.input = input;
//     }

//     // pub fn set_placement(&mut self, placement: ElementPlacement) {
//     //     self.placement = placement
//     // }
// }

pub struct SceneContext {
    // internal: Rc<RefCell<SceneContextInternal>>,
    input: Rc<RefCell<InputState>>,
    scene_layout: Rc<RefCell<ElementPlacement>>,
    shapes: Vec<PaintShape>,
    scale_factor: f32,
}

pub struct ChildUI {
    shapes: Vec<PaintShape>,
    pub size: Size2,
}

impl Clone for SceneContext {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            scene_layout: self.scene_layout.clone(),
            shapes: Default::default(),
            scale_factor: self.scale_factor,
        }
    }
}

impl SceneContext {
    fn new_inner(
        input: Rc<RefCell<InputState>>,
        scene_layout: Rc<RefCell<ElementPlacement>>,
        scale_factor: f32,
    ) -> Self {
        Self {
            input,
            scene_layout,
            shapes: Default::default(),
            scale_factor,
        }
    }

    pub(super) fn new(
        input: InputState,
        scene_layout: ElementPlacement,
        scale_factor: f32,
    ) -> Self {
        Self::new_inner(
            Rc::new(RefCell::new(input)),
            Rc::new(RefCell::new(scene_layout)),
            scale_factor,
        )
    }

    pub(super) fn drain(self) -> (Vec<PaintShape>, InputState) {
        (self.shapes, self.input.take())
    }

    pub fn add_shape(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into())
    }

    pub fn add_buffer(&mut self, buffer: &cosmic_text::Buffer) {}

    pub fn input(&mut self) -> RefMut<InputState> {
        self.input.borrow_mut()
    }

    pub fn render_child(&mut self, element: &mut impl Element) {
        let mut ctx = self.clone();

        let scene_layout = self.scene_layout.borrow();
        let placement = scene_layout.get(&element.id());

        if let Some(pos) = placement {
            element.ui(&mut ctx, *pos);
            self.shapes.extend(ctx.shapes.into_iter());
        }
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
