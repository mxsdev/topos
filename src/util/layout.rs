use std::{
    borrow::BorrowMut,
    cell::{RefCell, RefMut},
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use super::taffy::*;
use itertools::Itertools;
use refbox::RefBox;

use crate::math::{Rect, Size};

pub type MeasureFunc = TaffyMeasureFunc;

pub trait Measurable: Send + Sync {
    fn measure(
        &self,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> Size<f32>;
}

pub struct MeasurableFunc<T: Measurable> {
    pub func: T,
}

impl<T: Measurable> MeasurableFunc<T> {
    pub fn new(inner: T) -> Self {
        Self { func: inner }
    }
}

impl<T: Measurable> TaffyMeasurable for MeasurableFunc<T> {
    fn measure(
        &self,
        known_dimensions: TaffySize<Option<f32>>,
        available_space: TaffySize<TaffyAvailableSpace>,
    ) -> TaffySize<f32> {
        self.func
            .measure(
                known_dimensions.into(),
                available_space.map(Into::into).into(),
            )
            .into()
    }
}

pub fn measure_func_boxed<T: Measurable + 'static>(func: T) -> MeasureFunc {
    MeasureFunc::Boxed(Box::new(MeasurableFunc::new(func)))
}

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
pub struct LengthPercentage(TaffyLengthPercentage);

impl Into<LengthPercentage> for f32 {
    fn into(self) -> LengthPercentage {
        LengthPercentage(TaffyLengthPercentage::Length(self))
    }
}

impl Into<LengthPercentage> for Percent {
    fn into(self) -> LengthPercentage {
        LengthPercentage(TaffyLengthPercentage::Percent(self.0))
    }
}

impl Into<TaffyLengthPercentage> for LengthPercentage {
    fn into(self) -> TaffyLengthPercentage {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Dimension(TaffyDimension);

impl Into<Dimension> for f32 {
    fn into(self) -> Dimension {
        Dimension(TaffyDimension::Length(self))
    }
}

impl Into<Dimension> for Percent {
    fn into(self) -> Dimension {
        Dimension(TaffyDimension::Percent(self.0))
    }
}

impl Into<Dimension> for Auto {
    fn into(self) -> Dimension {
        Dimension(TaffyDimension::Auto)
    }
}

impl Into<TaffyDimension> for Dimension {
    fn into(self) -> TaffyDimension {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AvailableSpace {
    /// The amount of space available is the specified number of pixels
    Definite(f32),
    /// The amount of space available is indefinite and the node should be laid out under a min-content constraint
    MinContent,
    /// The amount of space available is indefinite and the node should be laid out under a max-content constraint
    MaxContent,
}

impl Into<TaffyAvailableSpace> for AvailableSpace {
    fn into(self) -> TaffyAvailableSpace {
        match self {
            AvailableSpace::Definite(val) => TaffyAvailableSpace::Definite(val),
            AvailableSpace::MinContent => TaffyAvailableSpace::MinContent,
            AvailableSpace::MaxContent => TaffyAvailableSpace::MaxContent,
        }
    }
}

impl From<TaffyAvailableSpace> for AvailableSpace {
    fn from(space: TaffyAvailableSpace) -> Self {
        match space {
            TaffyAvailableSpace::Definite(val) => AvailableSpace::Definite(val),
            TaffyAvailableSpace::MinContent => AvailableSpace::MinContent,
            TaffyAvailableSpace::MaxContent => AvailableSpace::MaxContent,
        }
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
        self.style.size = taffy::geometry::Size::<TaffyDimension> {
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
        self.style.gap.width = TaffyLengthPercentage::Length(hor);
        self
    }

    pub fn gap_y(mut self, vert: f32) -> Self {
        self.style.gap.height = TaffyLengthPercentage::Length(vert);
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
}

impl Into<taffy::style::Style> for CSSLayoutBuilder {
    fn into(self) -> taffy::style::Style {
        self.style
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LayoutNodeInternal {
    inner: taffy::tree::NodeId,
    engine: refbox::Ref<taffy::Taffy>,
}

impl Hash for LayoutNodeInternal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Into::<u64>::into(self.inner).hash(state)
    }
}

impl PartialOrd for LayoutNodeInternal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Into::<u64>::into(self.inner).partial_cmp(&Into::<u64>::into(other.inner))
    }
}

impl Ord for LayoutNodeInternal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Into::<u64>::into(self.inner).cmp(&Into::<u64>::into(other.inner))
    }
}

impl LayoutNodeInternal {
    #[inline]
    pub const fn new(inner: taffy::tree::NodeId, engine: refbox::Ref<taffy::Taffy>) -> Self {
        Self { inner, engine }
    }
}

impl Drop for LayoutNodeInternal {
    fn drop(&mut self) {
        let mut engine = self.engine.try_borrow_mut().unwrap();
        engine.remove(self.inner).unwrap();
    }
}

impl Into<TaffyNodeId> for LayoutNodeInternal {
    #[inline]
    fn into(self) -> TaffyNodeId {
        self.inner
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayoutNode {
    node: Rc<LayoutNodeInternal>,
}

impl LayoutNode {
    pub fn new(inner: TaffyNodeId, engine: refbox::Ref<taffy::Taffy>) -> Self {
        Self {
            node: Rc::new(LayoutNodeInternal::new(inner, engine)),
        }
    }

    pub(crate) fn inner(&self) -> TaffyNodeId {
        self.node.inner
    }
}

#[derive(Default)]
pub struct LayoutEngine {
    inner: RefBox<taffy::Taffy>,
}

impl LayoutEngine {
    fn get_inner_mut(&mut self) -> impl DerefMut<Target = taffy::Taffy> + '_ {
        self.inner.try_borrow_mut().unwrap()
    }

    pub fn set_children<'a>(
        &mut self,
        parent: &LayoutNode,
        children: impl Iterator<Item = &'a LayoutNode>,
    ) -> Result<(), TaffyError> {
        self.get_inner_mut()
            .set_children(parent.inner(), &children.map(|c| c.inner()).collect_vec())
    }

    pub fn compute_layout(
        &mut self,
        node: &LayoutNode,
        size: impl Into<taffy::geometry::Size<taffy::prelude::AvailableSpace>>,
    ) -> Result<(), TaffyError> {
        self.get_inner_mut()
            .compute_layout(node.inner(), size.into())
    }

    pub fn layout(&mut self, node: &LayoutNode) -> Result<Rect, TaffyError> {
        self.get_inner_mut().layout(node.inner()).map(Into::into)
    }

    pub fn new_leaf(
        &mut self,
        style: impl Into<taffy::style::Style>,
    ) -> Result<LayoutNode, TaffyError> {
        let inner_ref = self.inner.create_ref();

        self.get_inner_mut()
            .new_leaf(style.into())
            .map(|node| LayoutNode::new(node, inner_ref))
    }

    pub fn new_leaf_with_measure(
        &mut self,
        style: impl Into<taffy::style::Style>,
        measure: impl Into<TaffyMeasureFunc>,
    ) -> Result<LayoutNode, taffy::TaffyError> {
        let inner_ref = self.inner.create_ref();

        self.get_inner_mut()
            .new_leaf_with_measure(style.into(), measure.into())
            .map(|node| LayoutNode::new(node, inner_ref))
    }

    pub fn disable_rounding(&mut self) {
        self.get_inner_mut().disable_rounding()
    }

    pub fn enable_rounding(&mut self) {
        self.get_inner_mut().enable_rounding()
    }
}
