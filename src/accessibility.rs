use std::ops::{Deref, DerefMut};

use crate::math::{Pos, Rect, RoundedRect};

pub type AccessNode = accesskit::Node;
pub type AccessNodeId = accesskit::NodeId;
pub type AccessRole = accesskit::Role;

pub struct AccessNodeBuilder {
    inner: accesskit::NodeBuilder,
}

impl Deref for AccessNodeBuilder {
    type Target = accesskit::NodeBuilder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AccessNodeBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AccessNodeBuilder {
    pub fn new(role: AccessRole) -> Self {
        Self {
            inner: accesskit::NodeBuilder::new(role),
        }
    }

    pub fn build(self) -> AccessNode {
        self.inner
            .build(&mut accesskit::NodeClassSet::lock_global())
    }
}

pub type AccessPoint = accesskit::Point;
pub type AccessRect = accesskit::Rect;

pub trait AsAccessPoint {
    fn as_access_point(&self) -> AccessPoint;
}

impl AsAccessPoint for Pos {
    fn as_access_point(&self) -> AccessPoint {
        AccessPoint::new(self.x as f64, self.y as f64)
    }
}

pub trait AsAccessRect {
    fn as_access_rect(&self) -> AccessRect;
}

impl AsAccessRect for Rect {
    fn as_access_rect(&self) -> AccessRect {
        AccessRect::from_points(self.min.as_access_point(), self.max.as_access_point())
    }
}

impl AsAccessRect for RoundedRect {
    fn as_access_rect(&self) -> AccessRect {
        AccessRect::from_points(self.min().as_access_point(), self.max().as_access_point())
    }
}
