use std::{hash::Hash, ops::DerefMut, rc::Rc, sync::{Arc, Mutex}};

use super::{taffy::*, text::{FontSystemRef, TextBoxSizeCacheKey, TextCacheBuffer}};
use derive_more::From;
use itertools::Itertools;
use refbox::RefBox;

use crate::math::{Rect, Size};


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
        LengthPercentage(TaffyLengthPercentage::length(self))
    }
}

impl Into<LengthPercentage> for Percent {
    fn into(self) -> LengthPercentage {
        LengthPercentage(TaffyLengthPercentage::percent(self.0))
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
        Dimension(TaffyDimension::length(self))
    }
}

impl Into<Dimension> for Percent {
    fn into(self) -> Dimension {
        Dimension(TaffyDimension::percent(self.0))
    }
}

impl Into<Dimension> for Auto {
    fn into(self) -> Dimension {
        Dimension(TaffyDimension::auto())
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

    pub fn max_size(mut self, size: Size<impl Into<Dimension>>) -> Self {
        self.style.max_size = taffy::geometry::Size::<TaffyDimension> {
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

    pub fn max_width(mut self, width: impl Into<Dimension>) -> Self {
        self.style.max_size.width = width.into().into();
        self
    }

    pub fn max_height(mut self, height: impl Into<Dimension>) -> Self {
        self.style.max_size.height = height.into().into();
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
        self.style.gap.width = TaffyLengthPercentage::length(hor);
        self
    }

    pub fn gap_y(mut self, vert: f32) -> Self {
        self.style.gap.height = TaffyLengthPercentage::length(vert);
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


#[derive(From)]
pub enum TaffyNodeContext {
    Text(Arc<Mutex<TextCacheBuffer>>),
}

pub type TaffyEngine = taffy::TaffyTree<TaffyNodeContext>;

#[derive(Debug, PartialEq, Eq)]
struct LayoutNodeInternal {
    inner: taffy::tree::NodeId,
    engine: refbox::Weak<TaffyEngine>,
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
    pub const fn new(inner: taffy::tree::NodeId, engine: refbox::Weak<TaffyEngine>) -> Self {
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
    pub fn new(inner: TaffyNodeId, engine: refbox::Weak<TaffyEngine>) -> Self {
        Self {
            node: Rc::new(LayoutNodeInternal::new(inner, engine)),
        }
    }

    pub fn set_style(&mut self, style: impl Into<LayoutStyle>) {
        self.node.engine.try_borrow_mut().unwrap().set_style(self.inner(), style.into());
    }

    pub(crate) fn inner(&self) -> TaffyNodeId {
        self.node.inner
    }
}

pub struct LayoutEngine {
    inner: RefBox<TaffyEngine>,
    font_system: FontSystemRef,
}

impl LayoutEngine {
    pub fn new(font_system: FontSystemRef) -> Self {
        Self {
            inner: RefBox::new(TaffyEngine::new()),
            font_system,
        }
    }
    
    fn get_inner_mut(&mut self) -> impl DerefMut<Target = TaffyEngine> + '_ {
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
        self.inner.try_borrow_mut().unwrap()
            .compute_layout_with_measure(node.inner(), size.into(), |known_dimensions, available_space, _node_id, node_context, _font_metrics| { 
                match node_context {
                    None => {
                        taffy::Size {
                            width: known_dimensions.width.unwrap_or_default(),
                            height: known_dimensions.height.unwrap_or_default(),
                        }
                    }

                    Some(TaffyNodeContext::Text(buffer))  => {
                        let taffy::Size {
                            width: available_width,
                            height: available_height,
                            ..
                        } = available_space;
                
                        let taffy::Size { width, height, .. } = known_dimensions;
                
                        let tbox_width = width.unwrap_or(match available_width {
                            taffy::AvailableSpace::Definite(max_width) => max_width,
                            taffy::AvailableSpace::MinContent => 0.,
                            taffy::AvailableSpace::MaxContent => f32::INFINITY,
                        });
                
                        let tbox_height = height.unwrap_or(match available_height {
                            taffy::AvailableSpace::Definite(max_height) => max_height,
                            taffy::AvailableSpace::MinContent => 0.,
                            taffy::AvailableSpace::MaxContent => f32::INFINITY,
                        });
                
                        let mut buffer = buffer.lock().unwrap();

                        buffer.buffer.set_size(
                            &mut self.font_system.lock().unwrap(),
                            Some(tbox_width),
                            Some(tbox_height),
                        );

                        let result = buffer.buffer.computed_size();

                        result.into()
                    }
                }
            })
    }

    pub fn layout(&mut self, node: &LayoutNode) -> Result<Rect, TaffyError> {
        self.get_inner_mut().layout(node.inner()).map(Into::into)
    }

    pub fn new_leaf(
        &mut self,
        style: impl Into<LayoutStyle>,
    ) -> Result<LayoutNode, TaffyError> {
        let inner_ref = self.inner.downgrade();

        self.get_inner_mut()
            .new_leaf(style.into())
            .map(|node| LayoutNode::new(node, inner_ref))
    }

    pub fn new_leaf_with_context(
        &mut self,
        style: impl Into<LayoutStyle>,
        context: impl Into<TaffyNodeContext>,
    ) -> Result<LayoutNode, TaffyError> {
        let inner_ref = self.inner.downgrade();

        self.get_inner_mut()
            .new_leaf_with_context(style.into(), context.into())
            .map(|node| LayoutNode::new(node, inner_ref))
    }

    pub fn disable_rounding(&mut self) {
        self.get_inner_mut().disable_rounding()
    }

    pub fn enable_rounding(&mut self) {
        self.get_inner_mut().enable_rounding()
    }
}

pub type LayoutStyle = taffy::style::Style;