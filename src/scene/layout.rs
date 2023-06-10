use std::ops::DerefMut;

use cosmic_text::FontSystem;
use ordered_hash_map::OrderedHashMap;
use rustc_hash::FxHashMap;

use crate::{
    element::{Element, ElementId, ElementRef, SizeConstraint},
    util::{Pos2, Size2, Vec2},
};

use super::scene::SceneResources;

pub type ElementPlacement = FxHashMap<ElementId, Pos2>;
// pub type ElementPlacement = Vec<(ElementWeakref<dyn Element>, Pos2)>;

pub struct LayoutPass {
    // pub(super) elements: Vec<ElementRef<'a>>,
    // result: FxHashMap<ElementRef<'a>, u32>,
    id: ElementId,
    // element: ElementWeakref<dyn Element>,
    placement: Option<Vec2>,
    children: OrderedHashMap<ElementId, LayoutPass>,

    scene_resources: SceneResources,
}

// type ElementRef<'a> = &'a mut dyn Element;

// pub struct LayoutHandle<'a> {
//     element_ref: ElementRef<'a>,
//     pub size: Size2,
//     // id: usize,
// }

struct LayoutNode {}

impl LayoutPass {
    pub(super) fn new(
        root: &mut ElementRef<impl Element + 'static>,
        scene_resources: SceneResources,
    ) -> Self {
        Self {
            id: root.id(),
            placement: Default::default(),
            children: Default::default(),
            scene_resources,
        }
    }

    fn create(&self, child: &mut ElementRef<impl Element + 'static>) -> Self {
        Self::new(child, self.scene_resources.clone())
    }

    pub fn layout_child(
        &mut self,
        child: &mut ElementRef<impl Element + 'static>,
        constraints: SizeConstraint,
    ) -> Size2 {
        let mut child_node = self.create(child);

        let size = child.get().layout(constraints, &mut child_node);

        self.children.insert(child.id(), child_node);

        size
    }

    pub fn place_child(&mut self, element: &ElementRef<impl Element>, pos: Pos2) {
        if let Some(child) = self.children.get_mut(&element.id()) {
            child.placement = Some(pos.to_vector());
        }
    }

    fn populate_placement(self, mut pos: Pos2, memo: &mut ElementPlacement) {
        if let Some(placement) = self.placement {
            pos += placement;
        }

        for child in self.children.into_values() {
            child.populate_placement(pos, memo);
        }

        memo.insert(self.id, pos);
    }

    pub(super) fn finish(self) -> ElementPlacement {
        let mut memo = Default::default();

        self.populate_placement(Pos2::zero(), &mut memo);

        memo
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
