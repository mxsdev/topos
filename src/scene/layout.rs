use std::ops::DerefMut;

use cosmic_text::FontSystem;
use rustc_hash::FxHashMap;

use crate::{
    element::{Element, ElementId, ElementRef, ElementWeakref, SizeConstraint},
    input::input_state::InputState,
    util::{Pos2, Size2, Vec2},
};

use super::{ctx::SceneContext, scene::SceneResources};

pub struct SceneLayout {
    elements: Vec<(ElementWeakref<dyn Element>, Pos2)>,
}

impl SceneLayout {
    pub(super) fn do_input_pass(&mut self, input: &mut InputState) {
        for (element, pos) in self.elements.iter_mut().rev() {
            if let Some(mut element) = element.try_get() {
                element.input(input, *pos)
            }
        }
    }

    pub(super) fn do_ui_pass(&mut self, ctx: &mut SceneContext) {
        for (element, pos) in self.elements.iter_mut() {
            if let Some(mut element) = element.try_get() {
                element.ui(ctx, *pos)
            }
        }
    }
}

pub struct LayoutPass {
    element: ElementWeakref<dyn Element>,
    placement: Option<Vec2>,

    children: Vec<LayoutPass>,
    children_map: FxHashMap<ElementId, usize>,

    scene_resources: SceneResources,
}

impl LayoutPass {
    pub(super) fn new(
        root: &mut ElementRef<impl Element + 'static>,
        scene_resources: SceneResources,
    ) -> Self {
        Self {
            placement: Default::default(),
            children: Default::default(),
            children_map: Default::default(),
            scene_resources,
            element: root.get_weak_dyn(),
        }
    }

    fn create(&self, child: &mut ElementRef<impl Element + 'static>) -> Self {
        Self::new(child, self.scene_resources.clone())
    }

    pub fn layout_and_place_child(
        &mut self,
        child: &mut ElementRef<impl Element + 'static>,
        constraints: SizeConstraint,
        pos: Pos2,
    ) -> Size2 {
        let (size, idx) = self.layout_child_inner(child, constraints);
        self.place_child_inner(child, pos, idx);

        size
    }

    fn layout_child_inner(
        &mut self,
        child: &mut ElementRef<impl Element + 'static>,
        constraints: SizeConstraint,
    ) -> (Size2, usize) {
        let mut child_node = self.create(child);

        let size = child.get().layout(constraints, &mut child_node);

        let idx = self.children.len();

        self.children.push(child_node);
        self.children_map.insert(child.id(), idx);

        (size, idx)
    }

    pub fn layout_child(
        &mut self,
        child: &mut ElementRef<impl Element + 'static>,
        constraints: SizeConstraint,
    ) -> Size2 {
        self.layout_child_inner(child, constraints).0
    }

    fn place_child_inner(&mut self, element: &ElementRef<impl Element>, pos: Pos2, idx: usize) {
        self.children[idx].placement = Some(pos.to_vector());
    }

    pub fn place_child(&mut self, element: &ElementRef<impl Element>, pos: Pos2) {
        if let Some(idx) = self.children_map.get(&element.id()) {
            self.place_child_inner(element, pos, *idx)
        }
    }

    pub(super) fn do_layout_pass(
        mut self,
        screen_size: Size2,
        root: &mut ElementRef<impl Element>,
    ) -> SceneLayout {
        let default_constraints = SizeConstraint {
            min: Size2::zero(),
            max: screen_size,
        };

        root.get().layout(default_constraints, &mut self);

        self.finish()
    }

    fn finish_rec(self, mut pos: Pos2, scene_layout: &mut SceneLayout) {
        if let Some(placement) = self.placement {
            pos += placement;
        }

        scene_layout.elements.push((self.element, pos));

        for child in self.children.into_iter() {
            child.finish_rec(pos, scene_layout);
        }
    }

    pub fn finish(self) -> SceneLayout {
        let mut layout = SceneLayout {
            elements: Default::default(),
        };

        self.finish_rec(Pos2::default(), &mut layout);

        layout
    }

    pub fn scale_factor(&self) -> f32 {
        self.scene_resources.scale_factor()
    }

    pub fn font_system(&mut self) -> impl DerefMut<Target = FontSystem> + '_ {
        self.scene_resources.font_system()
    }

    pub fn scene_resources(&self) -> &SceneResources {
        &self.scene_resources
    }
}
