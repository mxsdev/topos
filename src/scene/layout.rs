use std::{
    borrow::Cow,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use cosmic_text::FontSystem;
use itertools::Itertools;

use crate::{
    element::{Element, ElementRef, ElementWeakref},
    input::input_state::InputState,
    math::{Pos, Rect, Size, WindowScaleFactor},
};

use super::{ctx::SceneContext, scene::SceneResources};

pub use crate::util::layout::*;

pub type LayoutPassResult = crate::util::layout::LayoutNode;

// scene layout

pub type ElementTree = ElementTreeNode;

pub struct ElementTreeNode {
    element: ElementWeakref<dyn Element>,
    rect: Rect,
    children: Vec<ElementTreeNode>,
    layout_node: LayoutPassResult,
}

impl ElementTreeNode {
    pub(super) fn do_input_pass(&mut self, input: &mut InputState) -> bool {
        input.set_current_element(self.element.id().into());
        let mut focus_within = input.is_focused();

        if let Some(mut element) = self.element.try_get() {
            for child in self.children.iter_mut().rev() {
                focus_within |= child.do_input_pass(input);
            }

            input.set_focused_within(focus_within);
            element.input(input, self.rect);
        }

        focus_within
    }

    pub(super) fn do_ui_pass(&mut self, ctx: &mut SceneContext) {
        let element_id = self.element.id();

        if let Some(mut element) = self.element.try_get() {
            element.ui(ctx, self.rect);

            let mut children_access_nodes = Vec::new();

            for child in self.children.iter_mut() {
                child.do_ui_pass(ctx);
                children_access_nodes.push(child.element.id().as_access_id())
            }

            element.ui_post(ctx, self.rect);

            let mut access_node_builder = element.node();
            access_node_builder.set_children(children_access_nodes);

            let access_node = access_node_builder.build();
            ctx.output
                .accesskit_update()
                .nodes
                .push((element_id.as_access_id(), access_node));
        }
    }

    pub(super) fn do_layout_post_pass(&mut self, resources: &mut SceneResources) {
        if let Some(mut element) = self.element.try_get() {
            element.layout_post(resources, self.rect);

            for child in self.children.iter_mut() {
                child.do_layout_post_pass(resources);
            }
        }
    }
}

pub type LayoutEngine = crate::util::layout::LayoutEngine;

pub type LayoutPass<'a, 'b> = LayoutPassGeneric<&'a mut SceneResources<'b>, ()>;
type LayoutNode = LayoutPassGeneric<(), LayoutPassResult>;

pub struct LayoutPassGeneric<Resources, Result> {
    element: ElementWeakref<dyn Element>,
    children: Vec<LayoutNode>,
    resources: Resources,
    result: Result,
}

impl<'a, 'b: 'a> LayoutPass<'a, 'b> {
    pub(super) fn new(
        root: &mut ElementRef<impl Element + 'static>,
        scene_resources: &'a mut SceneResources<'b>,
    ) -> Self {
        Self {
            children: Default::default(),
            element: root.get_weak_dyn(),
            resources: scene_resources,
            result: Default::default(),
        }
    }

    fn finish(self, result: LayoutPassResult) -> (LayoutNode, &'a mut SceneResources<'b>) {
        self.resources
            .layout_engine()
            .set_children(&result, self.children.iter().map(|x| &x.result))
            .unwrap();

        (
            LayoutNode {
                element: self.element,
                children: self.children,
                resources: (),
                result,
            },
            self.resources,
        )
    }

    pub fn engine(&mut self) -> &mut LayoutEngine {
        self.resources.layout_engine()
    }

    pub fn layout_child(&mut self, child: &mut ElementRef<impl Element + 'static>) {
        let idx = self.children.len();
        let id = child.id();

        let (child_node, _) = {
            let mut child_node = LayoutPass::new(child, self.resources);
            let size = child.get().layout(&mut child_node);

            child_node.finish(size)
        };

        // let (child_node, _) = child_node.finish(size);

        self.children.push(child_node);
    }

    pub(super) fn do_layout_pass(
        mut self,
        screen_size: Size,
        root: &mut ElementRef<impl Element>,
    ) -> ElementTree {
        let root_layout_node = root.get().layout(&mut self);
        let (node, resources) = self.finish(root_layout_node);

        let layout_engine = resources.layout_engine();

        layout_engine
            .compute_layout(&node.result, screen_size)
            .unwrap();

        let mut tree = node.finish_rec(layout_engine, Pos::zero());
        tree.do_layout_post_pass(resources);

        tree
    }

    pub fn scale_factor(&self) -> WindowScaleFactor {
        self.resources.scale_factor()
    }

    pub fn font_system(&mut self) -> impl DerefMut<Target = FontSystem> + '_ {
        self.resources.font_system()
    }

    pub fn font_system_ref(&mut self) -> Arc<Mutex<FontSystem>> {
        self.resources.font_system_ref()
    }

    pub fn scene_resources(&self) -> &SceneResources {
        &self.resources
    }
}

impl LayoutNode {
    fn finish_rec(self, layout_engine: &mut LayoutEngine, parent_pos: Pos) -> ElementTreeNode {
        let result_rect: Rect = layout_engine.layout(&self.result).unwrap().into();

        let mut scene_layout = ElementTreeNode {
            children: Default::default(),
            element: self.element,
            rect: result_rect.translate(parent_pos.to_vector()),
            layout_node: self.result,
        };

        for child in self.children.into_iter() {
            scene_layout
                .children
                .push(child.finish_rec(layout_engine, scene_layout.rect.min));
        }

        scene_layout
    }
}
