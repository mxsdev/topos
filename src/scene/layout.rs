use ordered_hash_map::OrderedHashMap;
use rustc_hash::FxHashMap;

use crate::{
    element::{Element, ElementId, ElementRef, SizeConstraint},
    util::{Pos2, Size2, Vec2},
};

pub type ElementPlacement = FxHashMap<ElementId, Pos2>;

pub struct LayoutPass {
    // pub(super) elements: Vec<ElementRef<'a>>,
    // result: FxHashMap<ElementRef<'a>, u32>,
    id: ElementId,
    placement: Option<Vec2>,
    children: FxHashMap<ElementId, LayoutPass>,
}

// type ElementRef<'a> = &'a mut dyn Element;

// pub struct LayoutHandle<'a> {
//     element_ref: ElementRef<'a>,
//     pub size: Size2,
//     // id: usize,
// }

struct LayoutNode {}

impl LayoutPass {
    pub(super) fn create(child: &ElementRef<dyn Element>) -> Self {
        Self {
            id: child.id(),
            placement: Default::default(),
            children: Default::default(),
        }
    }

    pub fn layout_child(&mut self, child: &mut impl Element, constraints: SizeConstraint) -> Size2 {
        let mut child_node = LayoutPass::create(child);

        let size = child.layout(constraints, &mut child_node);

        size
    }

    pub fn place_child(&mut self, element: &impl Element, pos: Pos2) {
        if let Some(child) = self.children.get_mut(&element.id()) {
            child.placement = Some(pos.to_vector());
        }
    }

    fn populate_placement(&self, mut pos: Pos2, memo: &mut ElementPlacement) {
        if let Some(placement) = self.placement {
            pos += placement;
        }

        for child in self.children.values() {
            child.populate_placement(pos, memo);
        }

        memo.insert(self.id, pos);
    }

    pub(super) fn finish(self) -> ElementPlacement {
        let mut memo = Default::default();

        self.populate_placement(Pos2::zero(), &mut memo);

        memo
    }
}
