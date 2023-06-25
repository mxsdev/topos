use std::{default, ops::DerefMut};

use cosmic_text::FontSystem;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use taffy::style::{Dimension, LengthPercentage};

use crate::{
    element::{
        Element, ElementId, ElementList, ElementRef, ElementWeakref, GenericElement, HasElementId,
        SizeConstraint,
    },
    input::input_state::InputState,
    util::{FromMinSize, IntoGeom, IntoTaffy, Pos2, Rect, Size2, Vec2},
};

use super::{ctx::SceneContext, scene::SceneResources};

pub type LayoutPassResult = Size2;

// TODO: switch fully to taffy??
// custom_derive! {
//     #[derive(EnumFromInner)]
//     pub enum LayoutPassResult {
//         Size(Size2),
//         LayoutEngine(taffy::node::Node),
//     }
// }

#[derive(Copy, Clone, Debug, Default)]
pub struct LayoutRect<F = f32> {
    pub left: F,
    pub right: F,
    pub top: F,
    pub bottom: F,
}

impl<F: Copy + Default> LayoutRect<F> {
    pub fn x(val: F) -> Self {
        Self::default().splat_x(val)
    }

    pub fn y(val: F) -> Self {
        Self::default().splat_y(val)
    }

    pub fn same(val: F) -> Self {
        Self::default().splat(val)
    }

    pub fn splat_x(mut self, val: F) -> Self {
        self.left = val;
        self.right = val;
        self
    }

    pub fn splat_y(mut self, val: F) -> Self {
        self.top = val;
        self.bottom = val;
        self
    }

    pub fn splat(self, val: F) -> Self {
        self.splat_x(val).splat_y(val)
    }
}

// layout placers

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    Center,
    #[default]
    Start,
    End,

    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    Center,
    #[default]
    Start,
    End,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum LayoutDisplay {
    #[default]
    Flex,
    Grid,
    None,
}

pub struct FlexBox {}

impl FlexBox {
    pub fn default() -> CSSLayout {
        CSSLayout::flex_box()
    }
}

#[derive(Default)]
pub struct CSSLayout {
    pub display: LayoutDisplay,

    pub pos: Pos2,
    pub size: Option<Size2>,
    pub direction: FlexDirection,
    pub gap: f32,

    pub justify_content: JustifyContent,
    pub align_items: AlignItems,

    pub padding: LayoutRect,
}

impl CSSLayout {
    pub fn flex_box() -> Self {
        Self {
            display: LayoutDisplay::Flex,
            ..Default::default()
        }
    }

    pub fn size(mut self, size: Size2) -> Self {
        self.size = size.into();
        self
    }

    pub fn from_rect(rect: Rect) -> Self {
        Self {
            size: rect.size().into(),
            pos: rect.min,
            ..Default::default()
        }
    }

    pub fn direction(mut self, direction: impl Into<FlexDirection>) -> Self {
        self.direction = direction.into();
        self
    }

    pub fn justify_content(mut self, align: impl Into<JustifyContent>) -> Self {
        self.justify_content = align.into();
        self
    }

    pub fn align_items(mut self, align: impl Into<AlignItems>) -> Self {
        self.align_items = align.into();
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    fn inner_size(&self) -> Option<Size2> {
        let mut res = self.size?;

        res.width = res
            .width
            .min(res.width - (self.padding.left + self.padding.right));

        res.height = res
            .height
            .min(res.height - (self.padding.top + self.padding.bottom));

        res.into()
    }

    pub fn padding(mut self, padding: LayoutRect) -> Self {
        self.padding = padding;
        self
    }

    pub fn place_children<'a>(
        self,
        constraints: SizeConstraint,
        layout_pass: &mut LayoutPass,
        elements: impl ElementList<'a>,
    ) -> Size2 {
        let inner_size = self.inner_size();

        let components = elements
            .element_list()
            .map(|el| {
                let size = layout_pass
                    .layout_child(el, inner_size.map(|x| x.into()).unwrap_or(constraints));

                (
                    el,
                    layout_pass
                        .engine()
                        .new_leaf(taffy::style::Style {
                            size: taffy::geometry::Size::<Dimension> {
                                width: Dimension::Points(size.width),
                                height: Dimension::Points(size.height),
                            },
                            ..Default::default()
                        })
                        .unwrap(),
                )
            })
            .collect_vec();

        let root = layout_pass
            .engine()
            .new_with_children(
                taffy::style::Style {
                    display: match self.display {
                        LayoutDisplay::Flex => taffy::style::Display::Flex,
                        LayoutDisplay::Grid => taffy::style::Display::Grid,
                        LayoutDisplay::None => taffy::style::Display::None,
                    },

                    flex_direction: match self.direction {
                        FlexDirection::Row => taffy::style::FlexDirection::Row,
                        FlexDirection::Column => taffy::style::FlexDirection::Column,
                    },

                    align_items: match self.align_items {
                        AlignItems::Center => taffy::style::AlignItems::Center,
                        AlignItems::Start => taffy::style::AlignItems::Start,
                        AlignItems::End => taffy::style::AlignItems::End,
                    }
                    .into(),

                    justify_content: match self.justify_content {
                        JustifyContent::Center => taffy::style::JustifyContent::Center,
                        JustifyContent::Start => taffy::style::JustifyContent::Start,
                        JustifyContent::End => taffy::style::JustifyContent::End,
                        JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
                        JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround,
                        JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly,
                    }
                    .into(),

                    size: self
                        .size
                        .map(|size| taffy::geometry::Size::<Dimension> {
                            width: Dimension::Points(size.width),
                            height: Dimension::Points(size.height),
                        })
                        .unwrap_or(taffy::style::Style::DEFAULT.size),

                    gap: taffy::geometry::Size {
                        height: LengthPercentage::Points(self.gap),
                        width: LengthPercentage::Points(self.gap),
                    },

                    ..Default::default()
                },
                &components.iter().map(|x| x.1).collect_vec(),
            )
            .unwrap();

        layout_pass
            .engine()
            .compute_layout(root, constraints.max.into_taffy())
            .unwrap();

        for (el, node) in components {
            let pos = layout_pass.engine().layout(node).unwrap().location;
            layout_pass.place_child(el, pos.into_geom())
        }

        layout_pass.engine().layout(root).unwrap().size.into_geom()
    }
}

// scene layout

pub type ElementTree = ElementTreeNode;

pub struct ElementTreeNode {
    element: ElementWeakref<dyn Element>,
    rect: Rect,
    children: Vec<ElementTreeNode>,
}

impl ElementTreeNode {
    pub(super) fn do_input_pass(&mut self, input: &mut InputState) {
        if let Some(mut element) = self.element.try_get() {
            for child in self.children.iter_mut().rev() {
                child.do_input_pass(input);
            }

            element.input(input, self.rect);
        }
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
}

pub type LayoutEngine = taffy::Taffy;

pub type LayoutPass<'a> = LayoutPassGeneric<&'a mut LayoutEngine, &'a mut SceneResources, ()>;
type LayoutNode = LayoutPassGeneric<(), (), Size2>;

pub struct LayoutPassGeneric<Engine, Resources, S> {
    element: ElementWeakref<dyn Element>,

    children: Vec<LayoutNode>,
    children_map: FxHashMap<ElementId, usize>,

    layout_engine: Engine,
    resources: Resources,

    placement: Option<Vec2>,

    size: S,
}

impl<'a> LayoutPass<'a> {
    pub(super) fn new(
        root: &mut (impl GenericElement + ?Sized),
        scene_resources: &'a mut SceneResources,
        engine: &'a mut LayoutEngine,
    ) -> Self {
        Self {
            children: Default::default(),
            children_map: Default::default(),
            element: root.get_weak_dyn(),
            resources: scene_resources,
            layout_engine: engine,
            placement: Default::default(),
            size: Default::default(),
        }
    }

    fn finish(self, size: Size2) -> LayoutNode {
        LayoutNode {
            element: self.element,
            children: self.children,
            children_map: self.children_map,
            placement: self.placement,

            layout_engine: (),
            resources: (),

            size,
        }
    }

    pub fn layout_and_place_child(
        &mut self,
        child: &mut (impl GenericElement + ?Sized),
        constraints: impl Into<SizeConstraint>,
        pos: Pos2,
    ) -> Size2 {
        let (size, idx) = self.layout_child_inner(child, constraints.into());
        self.place_child_inner(pos, idx);

        size
    }

    pub fn engine(&mut self) -> &mut LayoutEngine {
        self.layout_engine
    }

    fn layout_child_inner<'b>(
        &'b mut self,
        child: &mut (impl GenericElement + ?Sized),
        constraints: SizeConstraint,
    ) -> (Size2, usize) {
        let idx = self.children.len();
        let id = child.id();

        let mut child_node = LayoutPass::<'b>::new(child, self.resources, self.layout_engine);

        let size = child.layout(constraints, &mut child_node);

        let child_node = child_node.finish(size);

        self.children.push(child_node);
        self.children_map.insert(id, idx);

        (size, idx)
    }

    pub fn layout_child(
        &mut self,
        child: &mut (impl GenericElement + ?Sized),
        constraints: SizeConstraint,
    ) -> Size2 {
        self.layout_child_inner(child, constraints).0
    }

    fn place_child_inner(&mut self, pos: Pos2, idx: usize) {
        self.children[idx].placement = Some(pos.to_vector());
    }

    pub fn place_child(&mut self, element: &(impl HasElementId + ?Sized), pos: Pos2) {
        let idx = self.children_map.get(&element.id()).unwrap();
        self.place_child_inner(pos, *idx)
    }

    pub(super) fn do_layout_pass(
        mut self,
        screen_size: Size2,
        root: &mut ElementRef<impl Element>,
    ) -> ElementTree {
        let default_constraints = SizeConstraint {
            min: Size2::zero(),
            max: screen_size,
        };

        let size = root.get().layout(default_constraints, &mut self);
        let node = self.finish(size);

        node.finish_rec(Pos2::zero())
    }

    pub fn scale_factor(&self) -> f64 {
        self.resources.scale_factor()
    }

    pub fn font_system(&mut self) -> impl DerefMut<Target = FontSystem> + '_ {
        self.resources.font_system()
    }

    pub fn scene_resources(&self) -> &SceneResources {
        &self.resources
    }
}

impl LayoutNode {
    fn finish_rec(self, mut pos: Pos2) -> ElementTreeNode {
        if let Some(placement) = self.placement {
            pos += placement;
        }

        let mut scene_layout = ElementTreeNode {
            children: Default::default(),
            element: self.element,
            rect: Rect::from_min_size(pos, self.size),
        };

        for child in self.children.into_iter() {
            scene_layout.children.push(child.finish_rec(pos));
        }

        scene_layout
    }
}
