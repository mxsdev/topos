use std::num::NonZeroU128;

use uuid::Uuid;

use crate::accessibility::{AccessNodeBuilder, AccessNodeId};
use crate::debug::HashU64;
use crate::input::input_state::InputState;
use crate::math::CoordinateTransform;
use crate::refbox::{self, coerce_weak, RefBox};

use crate::scene::layout::LayoutPassResult;
use crate::scene::scene::SceneResources;
use crate::shape::ClipRect;
use crate::util::LogicalUnit;
use crate::{
    math::{Rect, Size},
    scene::{ctx::SceneContext, layout::LayoutPass},
};

#[derive(Clone, Copy)]
pub struct SizeConstraint<F = f32> {
    pub min: Size<F>,
    pub max: Size<F>,
}

impl<F: crate::num::Zero> Into<SizeConstraint<F>> for Size<F, LogicalUnit> {
    fn into(self) -> SizeConstraint<F> {
        SizeConstraint::max(self)
    }
}

impl<F> SizeConstraint<F> {
    pub fn max(size: Size<F>) -> Self
    where
        F: crate::num::Zero,
    {
        Self {
            min: Size::zero(),
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
        // TODO: more performant to just use first 64 bits of the UUID?
        accesskit::NodeId(self.inner.as_u128().hash_u64())
    }
}

pub trait RootConstructor: Element {
    fn new(resources: &mut SceneResources) -> Self;
}

pub trait Element {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult;
    fn layout_post(&mut self, _resources: &mut SceneResources, _rect: Rect) {}

    fn input(&mut self, _input: &mut InputState, _rect: Rect) {}
    fn input_with_resources(&mut self, _input: &mut InputState, _resources: &mut SceneResources, _rect: Rect) {}

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect);
    fn ui_post(&mut self, _ctx: &mut SceneContext, _rect: Rect) {}
    fn node(&self) -> AccessNodeBuilder;

    fn coordinate_transform(&self) -> Option<CoordinateTransform> {
        None
    }

    fn clip_rect(&self) -> Option<ClipRect> {
        None
    }
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

    pub fn id(&self) -> ElementId {
        self.id
    }

    pub fn get_weak_dyn(&mut self) -> ElementWeakref<dyn Element>
    where
        T: Sized + 'static,
    {
        ElementWeakref {
            reference: coerce_weak!(self.element.downgrade() => dyn Element),
            id: self.id(),
        }
    }
}

pub struct ElementWeakref<T: Element + ?Sized> {
    reference: refbox::Weak<T>,
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
