use std::borrow::BorrowMut;

use refbox::RefBox;

use crate::{
    scene::{ctx::SceneContext, layout::LayoutPass, update::UpdatePass, PaintPass},
    util::{LogicalUnit, Pos2, Size2},
};

use super::{
    boundary::{Boundary, EmptyBoundary},
    ElementEvent,
};

// pub type ElementRef<El: Element> = RefBox<El>;

pub struct SizeConstraint<F = f32> {
    pub min: Size2<F>,
    pub max: Size2<F>,
}

pub type ElementId = usize;

pub trait Element: Send {
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

pub struct ElementRef<T: Element + ?Sized> {
    element: RefBox<T>,
}

impl<T: Element> ElementRef<T> {
    pub fn get(&mut self) -> refbox::Borrow<T> {
        self.element.try_borrow_mut().unwrap()
    }

    pub fn get_weak(&mut self) -> ElementWeakref<T> {
        ElementWeakref {
            reference: self.element.create_ref(),
        }
    }
}

pub struct ElementWeakref<T: Element + ?Sized> {
    reference: refbox::Ref<T>,
}

impl<T: Element> ElementWeakref<T> {
    pub fn try_get(&mut self) -> Option<refbox::Borrow<T>> {
        self.reference.try_borrow_mut().ok()
    }
}
