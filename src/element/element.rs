use std::num::NonZeroU128;
use std::sync::Arc;

use uuid::Uuid;

use crate::accessibility::{AccessNode, AccessNodeBuilder, AccessNodeId};
use crate::input::input_state::InputState;
use crate::refbox::{self, coerce_ref, RefBox};

use crate::scene::scene::SceneResources;
use crate::util::LogicalUnit;
use crate::{
    scene::{ctx::SceneContext, layout::LayoutPass},
    util::{Pos2, Size2},
};

#[derive(Clone, Copy)]
pub struct SizeConstraint<F = f32> {
    pub min: Size2<F>,
    pub max: Size2<F>,
}

impl<F: euclid::num::Zero> Into<SizeConstraint<F>> for euclid::Size2D<F, LogicalUnit> {
    fn into(self) -> SizeConstraint<F> {
        SizeConstraint::max(self)
    }
}

impl<F> SizeConstraint<F> {
    pub fn max(size: Size2<F>) -> Self
    where
        F: euclid::num::Zero,
    {
        Self {
            min: Size2::zero(),
            max: size,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct ElementId {
    inner: Uuid,
}

impl ElementId {
    pub fn new() -> Self {
        Self {
            inner: Uuid::new_v4(),
        }
    }

    pub fn as_access_id(&self) -> AccessNodeId {
        accesskit::NodeId(NonZeroU128::new(self.inner.as_u128()).unwrap())
    }
}

pub trait RootConstructor: Element {
    fn new(resources: &SceneResources) -> Self;
}

pub trait Element {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2;
    fn input(&mut self, input: &mut InputState, pos: Pos2) {}
    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2);
    fn ui_post(&mut self, ctx: &mut SceneContext, pos: Pos2) {}
    fn node(&self) -> AccessNodeBuilder;
}

pub struct ElementRef<T: Element + ?Sized> {
    element: RefBox<T>,
    id: ElementId,
}

impl<T: Element + ?Sized> ElementRef<T> {
    pub fn new(element: T) -> Self
    where
        T: Sized,
    {
        Self {
            element: RefBox::new(element),
            id: ElementId::new(),
        }
    }

    pub fn get(&mut self) -> refbox::Borrow<T> {
        self.element.try_borrow_mut().unwrap()
    }

    pub fn get_weak_dyn(&mut self) -> ElementWeakref<dyn Element>
    where
        T: Sized + 'static,
    {
        ElementWeakref {
            reference: coerce_ref!(self.element.create_ref() => dyn Element),
            id: self.id(),
        }
    }

    pub fn id(&self) -> ElementId {
        self.id
    }
}

pub struct ElementWeakref<T: Element + ?Sized> {
    reference: refbox::Ref<T>,
    id: ElementId,
}

impl<T: Element + ?Sized> ElementWeakref<T> {
    pub fn try_get(&mut self) -> Option<refbox::Borrow<T>> {
        self.reference.try_borrow_mut().ok()
    }

    pub fn id(&self) -> ElementId {
        self.id
    }
}

impl<T: Element> From<T> for ElementRef<T> {
    fn from(value: T) -> Self {
        ElementRef::new(value)
    }
}
