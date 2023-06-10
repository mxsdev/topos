use std::borrow::BorrowMut;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::time::UNIX_EPOCH;

use uuid::Uuid;

use crate::refbox::{self, coerce_ref, RefBox};

use crate::scene::scene::SceneResources;
use crate::{
    scene::{ctx::SceneContext, layout::LayoutPass, update::UpdatePass, PaintPass},
    util::{LogicalUnit, Pos2, Size2},
};

use super::{
    boundary::{Boundary, EmptyBoundary},
    ElementEvent,
};

// pub type ElementRef<El: Element> = RefBox<El>;

#[derive(Clone, Copy)]
pub struct SizeConstraint<F = f32> {
    pub min: Size2<F>,
    pub max: Size2<F>,
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
}

pub trait RootConstructor: Element {
    fn new(resources: &SceneResources) -> Self;
}

pub trait Element {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2;
    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2);

    // fn id(&self) -> ElementId
    // where
    //     Self: Sized,
    // {
    //     let (id, _) = (self as *const dyn Element).to_raw_parts();
    //     id as ElementId
    // }

    // fn ui(&mut self, ctx: &mut SceneContext, constraint: SizeConstraint) -> Size2;

    // fn update(&mut self, event: &ElementEvent, update: &mut UpdatePass) -> bool;
    // fn paint(&mut self, painter: &mut PaintPass);

    // fn update_hover(&mut self, mouse_pos: &Pos2) -> bool {
    //     false
    // }

    // fn takes_focus(&self) -> bool {
    //     true
    // }

    // fn boundary(&self) -> &impl Boundary {
    //     &EmptyBoundary
    // }

    // fn on_hover_enter(&mut self) {}
    // fn on_hover_exit(&mut self) {}
}

pub struct ElementRef<T: Element> {
    element: T,
    id: ElementId,
}

impl<T: Element> From<T> for ElementRef<T> {
    fn from(value: T) -> Self {
        ElementRef {
            element: value,
            id: ElementId::new(),
        }
    }
}

impl<T: Element> ElementRef<T> {
    pub fn get(&mut self) -> &mut T {
        &mut self.element
    }

    pub fn id(&self) -> ElementId {
        self.id
    }
}

// pub type ElementRef<T: Element + ?Sized> = T;

// pub struct ElementRef<T: Element + ?Sized> {
//     element: RefBox<T>,
// }

// impl<T: Element + ?Sized> ElementRef<T> {
//     pub fn new(element: T) -> Self
//     where
//         T: Sized,
//     {
//         Self {
//             element: RefBox::new(element),
//         }
//     }

//     pub fn get(&mut self) -> refbox::Borrow<T> {
//         self.element.try_borrow_mut().unwrap()
//     }

//     pub fn get_weak_dyn(&mut self) -> ElementWeakref<dyn Element>
//     where
//         T: Sized + 'static,
//     {
//         ElementWeakref {
//             reference: coerce_ref!(self.element.create_ref() => dyn Element),
//         }
//     }

//     // pub fn get_weak(&mut self) -> ElementWeakref<T> {
//     //     ElementWeakref {
//     //         reference: self.element.create_ref(),
//     //     }
//     // }

//     pub fn id(&self) -> ElementId {
//         let (id, _) = (self as *const Self).to_raw_parts();
//         id as ElementId
//     }
// }

// pub struct ElementWeakref<T: Element + ?Sized> {
//     reference: refbox::Ref<T>,
// }

// impl<T: Element + ?Sized> ElementWeakref<T> {
//     pub fn try_get(&mut self) -> Option<refbox::Borrow<T>> {
//         self.reference.try_borrow_mut().ok()
//     }
// }

// impl<T: Element> From<T> for ElementRef<T> {
//     fn from(value: T) -> Self {
//         ElementRef::new(value)
//     }
// }
