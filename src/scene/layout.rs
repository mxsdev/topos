use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use cosmic_text::FontSystem;
use itertools::Itertools;

use crate::{
    element::{Element, ElementRef, ElementWeakref},
    input::input_state::InputState,
    util::{Pos, Rect, Size, WindowScaleFactor},
};

use super::{ctx::SceneContext, scene::SceneResources};

pub type LayoutPassResult = taffy::prelude::Node;

#[derive(Copy, Clone, Debug, Default)]
pub struct LayoutRect<F = f32> {
    pub left: F,
    pub right: F,
    pub top: F,
    pub bottom: F,
}

impl<F> Into<taffy::geometry::Rect<F>> for LayoutRect<F> {
    fn into(self) -> taffy::geometry::Rect<F> {
        taffy::geometry::Rect {
            left: self.left,
            top: self.top,
            bottom: self.bottom,
            right: self.right,
        }
    }
}

impl<F> LayoutRect<F> {
    fn map<C, R>(self, f: C) -> LayoutRect<R>
    where
        C: Fn(F) -> R,
    {
        LayoutRect {
            left: f(self.left),
            right: f(self.right),
            top: f(self.top),
            bottom: f(self.bottom),
        }
    }
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
pub struct Row;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Column;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RowReverse;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnReverse;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl Into<taffy::style::FlexDirection> for FlexDirection {
    fn into(self) -> taffy::style::FlexDirection {
        match self {
            FlexDirection::Row => taffy::style::FlexDirection::Row,
            FlexDirection::Column => taffy::style::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::style::FlexDirection::ColumnReverse,
        }
    }
}

impl Into<FlexDirection> for Row {
    fn into(self) -> FlexDirection {
        FlexDirection::Row
    }
}

impl Into<FlexDirection> for Column {
    fn into(self) -> FlexDirection {
        FlexDirection::Column
    }
}

impl Into<FlexDirection> for RowReverse {
    fn into(self) -> FlexDirection {
        FlexDirection::RowReverse
    }
}

impl Into<FlexDirection> for ColumnReverse {
    fn into(self) -> FlexDirection {
        FlexDirection::ColumnReverse
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Center;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Start;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct End;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SpaceBetween;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SpaceAround;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SpaceEvenly;

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

impl Into<JustifyContent> for Center {
    fn into(self) -> JustifyContent {
        JustifyContent::Center
    }
}

impl Into<JustifyContent> for Start {
    fn into(self) -> JustifyContent {
        JustifyContent::Start
    }
}

impl Into<JustifyContent> for End {
    fn into(self) -> JustifyContent {
        JustifyContent::End
    }
}

impl Into<JustifyContent> for SpaceBetween {
    fn into(self) -> JustifyContent {
        JustifyContent::SpaceBetween
    }
}

impl Into<JustifyContent> for SpaceAround {
    fn into(self) -> JustifyContent {
        JustifyContent::SpaceAround
    }
}

impl Into<JustifyContent> for SpaceEvenly {
    fn into(self) -> JustifyContent {
        JustifyContent::SpaceEvenly
    }
}

impl Into<taffy::style::JustifyContent> for JustifyContent {
    fn into(self) -> taffy::style::JustifyContent {
        match self {
            JustifyContent::Center => taffy::style::JustifyContent::Center,
            JustifyContent::Start => taffy::style::JustifyContent::Start,
            JustifyContent::End => taffy::style::JustifyContent::End,
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    Center,
    #[default]
    Start,
    End,
}

impl Into<taffy::style::AlignItems> for AlignItems {
    fn into(self) -> taffy::style::AlignItems {
        match self {
            AlignItems::Center => taffy::style::AlignItems::Center,
            AlignItems::Start => taffy::style::AlignItems::Start,
            AlignItems::End => taffy::style::AlignItems::End,
        }
    }
}

impl Into<AlignItems> for Center {
    fn into(self) -> AlignItems {
        AlignItems::Center
    }
}

impl Into<AlignItems> for Start {
    fn into(self) -> AlignItems {
        AlignItems::Start
    }
}

impl Into<AlignItems> for End {
    fn into(self) -> AlignItems {
        AlignItems::End
    }
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
    pub fn builder() -> CSSLayoutBuilder {
        CSSLayoutBuilder::flex_box()
    }
}

pub struct Manual {}

impl Manual {
    pub fn builder() -> CSSLayoutBuilder {
        CSSLayoutBuilder::none()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Percent(pub f32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Auto;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LengthPercentage(taffy::style::LengthPercentage);

impl Into<LengthPercentage> for f32 {
    fn into(self) -> LengthPercentage {
        LengthPercentage(taffy::style::LengthPercentage::Points(self))
    }
}

impl Into<LengthPercentage> for Percent {
    fn into(self) -> LengthPercentage {
        LengthPercentage(taffy::style::LengthPercentage::Percent(self.0))
    }
}

impl Into<taffy::style::LengthPercentage> for LengthPercentage {
    fn into(self) -> taffy::style::LengthPercentage {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Dimension(taffy::style::Dimension);

impl Into<Dimension> for f32 {
    fn into(self) -> Dimension {
        Dimension(taffy::style::Dimension::Points(self))
    }
}

impl Into<Dimension> for Percent {
    fn into(self) -> Dimension {
        Dimension(taffy::style::Dimension::Percent(self.0))
    }
}

impl Into<Dimension> for Auto {
    fn into(self) -> Dimension {
        Dimension(taffy::style::Dimension::Auto)
    }
}

impl Into<taffy::style::Dimension> for Dimension {
    fn into(self) -> taffy::style::Dimension {
        self.0
    }
}

#[derive(Default)]
pub struct CSSLayoutBuilder {
    style: taffy::style::Style,
}

impl CSSLayoutBuilder {
    pub fn flex_box() -> Self {
        Self {
            style: taffy::style::Style {
                display: taffy::style::Display::Flex,
                ..Default::default()
            },
        }
    }

    pub fn none() -> Self {
        Self {
            style: taffy::style::Style {
                display: taffy::style::Display::None,
                ..Default::default()
            },
        }
    }

    pub fn size(mut self, size: Size<impl Into<Dimension>>) -> Self {
        self.style.size = taffy::geometry::Size::<taffy::style::Dimension> {
            width: size.width.into().into(),
            height: size.height.into().into(),
        };
        self
    }

    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    pub fn flex_basis(mut self, basis: impl Into<Dimension>) -> Self {
        self.style.flex_basis = basis.into().into();
        self
    }

    pub fn width(mut self, width: impl Into<Dimension>) -> Self {
        self.style.size.width = width.into().into();
        self
    }

    pub fn height(mut self, height: impl Into<Dimension>) -> Self {
        self.style.size.height = height.into().into();
        self
    }

    pub fn direction(mut self, direction: impl Into<FlexDirection>) -> Self {
        self.style.flex_direction = direction.into().into();
        self
    }

    pub fn justify_content(mut self, justify_content: impl Into<JustifyContent>) -> Self {
        self.style.justify_content = Some(justify_content.into().into());
        self
    }

    pub fn align_items(mut self, align: impl Into<AlignItems>) -> Self {
        self.style.align_items = Some(align.into().into());
        self
    }

    pub fn gap(self, val: f32) -> Self {
        self.gap_xy(val, val)
    }

    pub fn gap_x(mut self, hor: f32) -> Self {
        self.style.gap.width = taffy::style::LengthPercentage::Points(hor);
        self
    }

    pub fn gap_y(mut self, vert: f32) -> Self {
        self.style.gap.height = taffy::style::LengthPercentage::Points(vert);
        self
    }

    pub fn gap_xy(self, hor: f32, vert: f32) -> Self {
        self.gap_x(hor).gap_y(vert)
    }

    pub fn padding_left(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.style.padding.left = padding.into().into();
        self
    }

    pub fn padding_right(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.style.padding.right = padding.into().into();
        self
    }

    pub fn padding_x(self, padding: impl Into<LengthPercentage>) -> Self {
        let p = padding.into();
        self.padding_left(p).padding_right(p)
    }

    pub fn to_taffy(self) -> taffy::style::Style {
        self.style
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

pub type LayoutEngine = taffy::Taffy;

pub type LayoutPass<'a, 'b> = LayoutPassGeneric<&'a mut SceneResources<'b>, ()>;
type LayoutNode = LayoutPassGeneric<(), LayoutPassResult>;

pub struct LayoutPassGeneric<Resources, Result> {
    element: ElementWeakref<dyn Element>,
    children: Vec<LayoutNode>,
    resources: Resources,
    result: Result,
}

impl<'a, 'b> LayoutPass<'a, 'b> {
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
            .set_children(
                result,
                &self.children.iter().map(|c| c.result).collect_vec(),
            )
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
            .compute_layout(root_layout_node, screen_size.into())
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
        let result_rect: Rect = layout_engine.layout(self.result).unwrap().into();

        let mut scene_layout = ElementTreeNode {
            children: Default::default(),
            element: self.element,
            rect: result_rect.translate(parent_pos.to_vector()),
        };

        for child in self.children.into_iter() {
            scene_layout
                .children
                .push(child.finish_rec(layout_engine, scene_layout.rect.min));
        }

        scene_layout
    }
}
