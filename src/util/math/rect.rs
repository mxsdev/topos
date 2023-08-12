use std::borrow::Borrow;
use std::ops::*;

use crate::num::{One, Zero};
use num_traits::Float;

use super::{pos, vector, Pos, ScaleFactor, Sides, Size, Vector};
use crate::util::{markers::*, max, min, taffy::*};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect<T = f32, U = LogicalUnit> {
    pub min: Pos<T, U>,
    pub max: Pos<T, U>,
}

impl<T: Copy, U> Copy for Rect<T, U> {}

impl<T: Clone, U> Clone for Rect<T, U> {
    fn clone(&self) -> Self {
        Self::new(self.min.clone(), self.max.clone())
    }
}

impl<T, U> Rect<T, U> {
    /// Constructor.
    #[inline]
    pub const fn new(min: Pos<T, U>, max: Pos<T, U>) -> Self {
        Rect { min, max }
    }

    /// Constructor.
    #[inline]
    pub fn from_min_size(min: Pos<T, U>, size: Size<T, U>) -> Self
    where
        T: Copy + Add<T, Output = T>,
    {
        Rect {
            min,
            max: min + size,
        }
    }
}

impl<T, U> Rect<T, U>
where
    T: PartialOrd,
{
    /// Returns true if the box has a negative area.
    ///
    /// The common interpretation for a negative box is to consider it empty. It can be obtained
    /// by calculating the intersection of two boxes that do not intersect.
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.max.x < self.min.x || self.max.y < self.min.y
    }

    /// Returns true if the size is zero, negative or NaN.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !(self.max.x > self.min.x && self.max.y > self.min.y)
    }

    /// Returns `true` if the two boxes intersect.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Returns `true` if this box contains the point. Points are considered
    /// in the box if they are on the front, left or top faces, but outside if they
    /// are on the back, right or bottom faces.
    #[inline]
    pub fn contains(&self, p: Pos<T, U>) -> bool {
        self.min.x <= p.x && p.x < self.max.x && self.min.y <= p.y && p.y < self.max.y
    }

    /// Returns `true` if this box contains the interior of the other box. Always
    /// returns `true` if other is empty, and always returns `false` if other is
    /// nonempty but this box is empty.
    #[inline]
    pub fn contains_box(&self, other: &Self) -> bool {
        other.is_empty()
            || (self.min.x <= other.min.x
                && other.max.x <= self.max.x
                && self.min.y <= other.min.y
                && other.max.y <= self.max.y)
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + PartialOrd,
{
    #[inline]
    pub fn to_non_empty(&self) -> Option<Self> {
        if self.is_empty() {
            return None;
        }

        Some(*self)
    }

    /// Computes the intersection of two boxes, returning `None` if the boxes do not intersect.
    #[inline]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let b = self.intersection_unchecked(other);

        if b.is_empty() {
            return None;
        }

        Some(b)
    }

    /// Computes the intersection of two boxes without check whether they do intersect.
    ///
    /// The result is a negative box if the boxes do not intersect.
    /// This can be useful for computing the intersection of more than two boxes, as
    /// it is possible to chain multiple intersection_unchecked calls and check for
    /// empty/negative result at the end.
    #[inline]
    pub fn intersection_unchecked(&self, other: &Self) -> Self {
        Rect {
            min: pos(max(self.min.x, other.min.x), max(self.min.y, other.min.y)),
            max: pos(min(self.max.x, other.max.x), min(self.max.y, other.max.y)),
        }
    }

    /// Computes the union of two boxes.
    ///
    /// If either of the boxes is empty, the other one is returned.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        if other.is_empty() {
            return *self;
        }
        if self.is_empty() {
            return *other;
        }

        Rect {
            min: pos(min(self.min.x, other.min.x), min(self.min.y, other.min.y)),
            max: pos(max(self.max.x, other.max.x), max(self.max.y, other.max.y)),
        }
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + Add<T, Output = T>,
{
    /// Returns the same box, translated by a vector.
    #[inline]
    pub fn translate(&self, by: Vector<T, U>) -> Self {
        Rect {
            min: self.min + by,
            max: self.max + by,
        }
    }
}

// impl<T, U> Rect<T, U>
// where
//     T: Copy + Add<T, Output = T> + Div<T, Output = T> + Two,
// {
//     #[inline]
//     pub fn origin(&self) -> Pos<T, U> {
//         (self.min + self.max.to_vector()) / T::TWO
//     }
// }

impl<T, U> Rect<T, U>
where
    T: Copy + Sub<T, Output = T>,
{
    #[inline]
    pub fn size(&self) -> Size<T, U> {
        (self.max - self.min).to_size()
    }

    /// Change the size of the box by adjusting the max endpoint
    /// without modifying the min endpoint.
    #[inline]
    pub fn set_size(&mut self, size: Size<T, U>) {
        let diff = (self.size() - size).to_vector();
        self.max -= diff;
    }

    #[inline]
    pub fn width(&self) -> T {
        self.max.x - self.min.x
    }

    #[inline]
    pub fn height(&self) -> T {
        self.max.y - self.min.y
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + Add<T, Output = T> + Sub<T, Output = T>,
{
    /// Inflates the box by the specified sizes on each side respectively.
    #[inline]
    #[must_use]
    pub fn inflate(&self, width: T, height: T) -> Self {
        Rect {
            min: pos(self.min.x - width, self.min.y - height),
            max: pos(self.max.x + width, self.max.y + height),
        }
    }

    /// Inflates the box by the specified sizes on each side respectively.
    #[inline]
    #[must_use]
    pub fn deflate(&self, width: T, height: T) -> Self
    where
        T: Neg<Output = T>,
    {
        self.inflate(-width, -height)
    }

    /// Calculate the size and position of an inner box.
    ///
    /// Subtracts the side offsets from all sides. The horizontal, vertical
    /// and applicate offsets must not be larger than the original side length.
    pub fn inner_box(&self, offsets: Sides<T, U>) -> Self {
        Rect {
            min: self.min + vector(offsets.left, offsets.top),
            max: self.max - vector(offsets.right, offsets.bottom),
        }
    }

    /// Calculate the b and position of an outer box.
    ///
    /// Add the offsets to all sides. The expanded box is returned.
    pub fn outer_box(&self, offsets: Sides<T, U>) -> Self {
        Rect {
            min: self.min - vector(offsets.left, offsets.top),
            max: self.max + vector(offsets.right, offsets.bottom),
        }
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + Zero + PartialOrd,
{
    /// Returns the smallest box containing all of the provided points.
    pub fn from_points<I>(points: I) -> Self
    where
        I: IntoIterator,
        I::Item: Borrow<Pos<T, U>>,
    {
        let mut points = points.into_iter();

        let (mut min_x, mut min_y) = match points.next() {
            Some(first) => first.borrow().to_tuple(),
            None => return Rect::zero(),
        };

        let (mut max_x, mut max_y) = (min_x, min_y);
        for point in points {
            let p = point.borrow();
            if p.x < min_x {
                min_x = p.x
            }
            if p.x > max_x {
                max_x = p.x
            }
            if p.y < min_y {
                min_y = p.y
            }
            if p.y > max_y {
                max_y = p.y
            }
        }

        Rect {
            min: pos(min_x, min_y),
            max: pos(max_x, max_y),
        }
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + One + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    /// Linearly interpolate between this box and another box.
    #[inline]
    pub fn lerp(&self, other: Self, t: T) -> Self {
        Self::new(self.min.lerp(other.min, t), self.max.lerp(other.max, t))
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + One + Add<Output = T> + Div<Output = T>,
{
    pub fn center(&self) -> Pos<T, U> {
        let two = T::one() + T::one();
        (self.min + self.max.to_vector()) / two
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy + Mul<T, Output = T> + Sub<T, Output = T>,
{
    #[inline]
    pub fn area(&self) -> T {
        let size = self.size();
        size.width * size.height
    }
}

impl<T, U> Rect<T, U>
where
    T: Zero,
{
    /// Constructor, setting all sides to zero.
    pub fn zero() -> Self {
        Rect::new(Pos::zero(), Pos::zero())
    }
}

impl<T: Copy + Mul, U> Mul<T> for Rect<T, U> {
    type Output = Rect<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        Rect::new(self.min * scale, self.max * scale)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<T> for Rect<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: T) {
        *self *= ScaleFactor::new(scale);
    }
}

impl<T: Copy + Div, U> Div<T> for Rect<T, U> {
    type Output = Rect<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        Rect::new(self.min / scale, self.max / scale)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<T> for Rect<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: T) {
        *self /= ScaleFactor::new(scale);
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for Rect<T, U1> {
    type Output = Rect<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        Rect::new(self.min * scale, self.max * scale)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<T, U, U>> for Rect<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: ScaleFactor<T, U, U>) {
        self.min *= scale;
        self.max *= scale;
    }
}

impl<T: Copy + Div, U1, U2> Div<ScaleFactor<T, U1, U2>> for Rect<T, U2> {
    type Output = Rect<T::Output, U1>;

    #[inline]
    fn div(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        Rect::new(self.min / scale, self.max / scale)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<ScaleFactor<T, U, U>> for Rect<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: ScaleFactor<T, U, U>) {
        self.min /= scale;
        self.max /= scale;
    }
}

impl<T, U> Rect<T, U>
where
    T: Copy,
{
    #[inline]
    pub fn x_range(&self) -> Range<T> {
        self.min.x..self.max.x
    }

    #[inline]
    pub fn y_range(&self) -> Range<T> {
        self.min.y..self.max.y
    }

    /// Cast the unit
    #[inline]
    pub fn cast_unit<V>(&self) -> Rect<T, V> {
        Rect::new(self.min.cast_unit(), self.max.cast_unit())
    }

    #[inline]
    #[must_use]
    pub fn map<R>(self, f: impl Fn(T) -> R) -> Rect<R, U> {
        Rect::new(self.min.map(&f), self.max.map(&f))
    }

    #[inline]
    pub fn scale<S: Copy>(&self, x: S, y: S) -> Self
    where
        T: Mul<S, Output = T>,
    {
        Rect {
            min: pos(self.min.x * x, self.min.y * y),
            max: pos(self.max.x * x, self.max.y * y),
        }
    }
}

impl<T: Float, U> Rect<T, U> {
    /// Returns true if all members are finite.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }
}

pub type PhysicalRect<F = f32> = Rect<F, PhysicalUnit>;

impl From<TaffyLayout> for Rect {
    fn from(value: TaffyLayout) -> Self {
        Self::from_min_size(value.location.into(), value.size.into())
    }
}

impl From<&TaffyLayout> for Rect {
    fn from(value: &TaffyLayout) -> Self {
        Self::from_min_size(value.location.into(), value.size.into())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoundedRect<F = f32, U = LogicalUnit> {
    pub inner: Rect<F, U>,
    pub radius: Option<F>,
}

impl<F, U> RoundedRect<F, U> {
    #[inline]
    pub const fn new(inner: Rect<F, U>, radius: Option<F>) -> Self {
        Self { inner, radius }
    }

    #[inline]
    pub fn new_from(inner: Rect<F, U>, radius: impl Into<Option<F>>) -> Self {
        Self::new(inner, radius.into())
    }

    #[inline]
    pub const fn from_rect(rect: Rect<F, U>) -> Self {
        Self::new(rect, None)
    }

    #[inline]
    pub fn with_radius(self, radius: Option<F>) -> Self {
        Self {
            inner: self.inner,
            radius,
        }
    }

    #[inline]
    pub fn with_radius_from(self, radius: impl Into<Option<F>>) -> Self {
        Self {
            inner: self.inner,
            radius: radius.into(),
        }
    }

    #[inline(always)]
    pub fn min(self) -> Pos<F, U> {
        self.inner.min
    }

    #[inline(always)]
    pub fn max(self) -> Pos<F, U> {
        self.inner.max
    }
}

impl<T, U> RoundedRect<T, U>
where
    T: Copy + Add<T, Output = T> + Sub<T, Output = T>,
{
    /// Inflates the box by the specified sizes on each side respectively.
    #[inline]
    #[must_use]
    pub fn inflate(&self, width: T, height: T) -> Self {
        Self {
            inner: self.inner.inflate(width, height),
            radius: self.radius,
        }
    }

    /// Inflates the box by the specified sizes on each side respectively.
    #[inline]
    #[must_use]
    pub fn deflate(&self, width: T, height: T) -> Self
    where
        T: Neg<Output = T>,
    {
        Self {
            inner: self.inner.deflate(width, height),
            radius: self.radius,
        }
    }

    /// Calculate the size and position of an inner box.
    ///
    /// Subtracts the side offsets from all sides. The horizontal, vertical
    /// and applicate offsets must not be larger than the original side length.
    pub fn inner_box(&self, offsets: Sides<T, U>) -> Self {
        Self {
            inner: self.inner.inner_box(offsets),
            radius: self.radius,
        }
    }

    /// Calculate the b and position of an outer box.
    ///
    /// Add the offsets to all sides. The expanded box is returned.
    pub fn outer_box(&self, offsets: Sides<T, U>) -> Self {
        Self {
            inner: self.inner.outer_box(offsets),
            radius: self.radius,
        }
    }
}

pub type PhysicalRoundedRect<F = f32> = RoundedRect<F, PhysicalUnit>;

impl<F, U> From<Rect<F, U>> for RoundedRect<F, U> {
    fn from(rect: Rect<F, U>) -> Self {
        Self::from_rect(rect)
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for RoundedRect<T, U1> {
    type Output = RoundedRect<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        RoundedRect {
            inner: self.inner * scale,
            radius: self.radius.map(|r| r * scale.0),
        }
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<T, U, U>> for RoundedRect<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: ScaleFactor<T, U, U>) {
        self.inner *= scale;

        if let Some(radius) = &mut self.radius {
            *radius *= scale.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_sdf() {
        let rect = Rect::<f32, LogicalUnit>::new(pos(0., 0.), pos(4., 4.));

        use crate::util::math::WindowScaleFactor;

        let scale_factor = WindowScaleFactor::new(2.);

        let physical_rect: PhysicalRect = rect * scale_factor;

        assert_eq!(physical_rect, Rect::new(pos(0., 0.), pos(8., 8.)))
    }
}
