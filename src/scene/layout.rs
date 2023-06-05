use ordered_hash_map::OrderedHashMap;
use rustc_hash::FxHashMap;

use crate::{
    element::{Element, ElementId, ElementRef, SizeConstraint},
    util::{Pos2, Size2, Vec2},
};

pub type ElementPlacement = FxHashMap<ElementId, Pos2>;
// pub type ElementPlacement = Vec<(ElementWeakref<dyn Element>, Pos2)>;

pub struct LayoutPass {
    // pub(super) elements: Vec<ElementRef<'a>>,
    // result: FxHashMap<ElementRef<'a>, u32>,
    id: ElementId,
    // element: ElementWeakref<dyn Element>,
    placement: Option<Vec2>,
    children: OrderedHashMap<ElementId, LayoutPass>,
}

// type ElementRef<'a> = &'a mut dyn Element;

// pub struct LayoutHandle<'a> {
//     element_ref: ElementRef<'a>,
//     pub size: Size2,
//     // id: usize,
// }

struct LayoutNode {}

impl LayoutPass {
    pub(super) fn create(child: &mut ElementRef<impl Element + 'static>) -> Self {
        Self {
            // element: child.get_weak_dyn(),
            id: child.id(),
            placement: Default::default(),
            children: Default::default(),
        }
    }

    pub fn layout_child(
        &mut self,
        child: &mut ElementRef<impl Element + 'static>,
        constraints: SizeConstraint,
    ) -> Size2 {
        let mut child_node = LayoutPass::create(child);

        let size = child.layout(constraints, &mut child_node);

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

        // memo.push((self.element, pos));
        memo.insert(self.id, pos);
    }

    pub(super) fn finish(self) -> ElementPlacement {
        let mut memo = Default::default();

        self.populate_placement(Pos2::zero(), &mut memo);

        memo
    }
}
