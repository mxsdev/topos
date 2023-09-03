// Copyright 2013 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature = "cargo-clippy", allow(just_underscores_and_digits))]

use bytemuck::{Pod, Zeroable};
use core::cmp::{Eq, PartialEq};
use core::fmt;
use core::hash::Hash;
use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Sub};
use num_traits::NumCast;
use palette::num::Powu;

use crate::num::{One, Zero};
use crate::util::LogicalUnit;

use super::{Angle, Pos, Rect, Trig, Vector};

/// A 2d transform represented by a column-major 3 by 3 matrix, compressed down to 3 by 2.
///
/// Transforms can be parametrized over the source and destination units, to describe a
/// transformation from a space to another.
/// For example, `Transform2D<f32, WorldSpace, ScreenSpace>::transform_point4d`
/// takes a `Pos<f32, WorldSpace>` and returns a `Pos<f32, ScreenSpace>`.
///
/// Transforms expose a set of convenience methods for pre- and post-transformations.
/// Pre-transformations (`pre_*` methods) correspond to adding an operation that is
/// applied before the rest of the transformation, while post-transformations (`then_*`
/// methods) add an operation that is applied after.
///
/// The matrix representation is conceptually equivalent to a 3 by 3 matrix transformation
/// compressed to 3 by 2 with the components that aren't needed to describe the set of 2d
/// transformations we are interested in implicitly defined:
///
/// ```text
///  | m11 m12 0 |   |x|   |x'|
///  | m21 m22 0 | x |y| = |y'|
///  | m31 m32 1 |   |1|   |w |
/// ```
///
/// When translating Transform2D into general matrix representations, consider that the
/// representation follows the column-major notation with column vectors.
///
/// The translation terms are m31 and m32.
#[repr(C)]
pub struct CoordinateTransform<T = f32, Src = LogicalUnit, Dst = LogicalUnit> {
    pub m11: T,
    pub m12: T,
    pub m21: T,
    pub m22: T,
    pub m31: T,
    pub m32: T,
    #[doc(hidden)]
    pub _unit: PhantomData<(Src, Dst)>,
}

unsafe impl<T: Zeroable, Src, Dst> Zeroable for CoordinateTransform<T, Src, Dst> {}

unsafe impl<T: Pod, Src: 'static, Dst: 'static> Pod for CoordinateTransform<T, Src, Dst> {}

impl<T: Copy, Src, Dst> Copy for CoordinateTransform<T, Src, Dst> {}

impl<T: Clone, Src, Dst> Clone for CoordinateTransform<T, Src, Dst> {
    fn clone(&self) -> Self {
        CoordinateTransform {
            m11: self.m11.clone(),
            m12: self.m12.clone(),
            m21: self.m21.clone(),
            m22: self.m22.clone(),
            m31: self.m31.clone(),
            m32: self.m32.clone(),
            _unit: PhantomData,
        }
    }
}

impl<T, Src, Dst> Eq for CoordinateTransform<T, Src, Dst> where T: Eq {}

impl<T, Src, Dst> PartialEq for CoordinateTransform<T, Src, Dst>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.m11 == other.m11
            && self.m12 == other.m12
            && self.m21 == other.m21
            && self.m22 == other.m22
            && self.m31 == other.m31
            && self.m32 == other.m32
    }
}

impl<T, Src, Dst> Hash for CoordinateTransform<T, Src, Dst>
where
    T: Hash,
{
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.m11.hash(h);
        self.m12.hash(h);
        self.m21.hash(h);
        self.m22.hash(h);
        self.m31.hash(h);
        self.m32.hash(h);
    }
}

impl<T, Src, Dst> CoordinateTransform<T, Src, Dst> {
    /// Create a transform specifying its components in using the column-major-column-vector
    /// matrix notation.
    ///
    /// For example, the translation terms m31 and m32 are the last two parameters parameters.
    ///
    /// ```
    /// use euclid::default::Transform2D;
    /// let tx = 1.0;
    /// let ty = 2.0;
    /// let translation = Transform2D::new(
    ///   1.0, 0.0,
    ///   0.0, 1.0,
    ///   tx,  ty,
    /// );
    /// ```
    pub const fn new(m11: T, m12: T, m21: T, m22: T, m31: T, m32: T) -> Self {
        CoordinateTransform {
            m11,
            m12,
            m21,
            m22,
            m31,
            m32,
            _unit: PhantomData,
        }
    }
}

impl<T: Copy, Src, Dst> CoordinateTransform<T, Src, Dst> {
    /// Returns an array containing this transform's terms.
    ///
    /// The terms are laid out in the same order as they are
    /// specified in `Transform2D::new`, that is following the
    /// column-major-column-vector matrix notation.
    ///
    /// For example the translation terms are found in the
    /// last two slots of the array.
    #[inline]
    pub fn to_array(&self) -> [T; 6] {
        [self.m11, self.m12, self.m21, self.m22, self.m31, self.m32]
    }

    /// Returns an array containing this transform's terms transposed.
    ///
    /// The terms are laid out in transposed order from the same order of
    /// `Transform3D::new` and `Transform3D::to_array`, that is following
    /// the row-major-column-vector matrix notation.
    ///
    /// For example the translation terms are found at indices 2 and 5
    /// in the array.
    #[inline]
    pub fn to_array_transposed(&self) -> [T; 6] {
        [self.m11, self.m21, self.m31, self.m12, self.m22, self.m32]
    }

    /// Equivalent to `to_array` with elements packed two at a time
    /// in an array of arrays.
    #[inline]
    pub fn to_arrays(&self) -> [[T; 2]; 3] {
        [
            [self.m11, self.m12],
            [self.m21, self.m22],
            [self.m31, self.m32],
        ]
    }

    /// Create a transform providing its components via an array
    /// of 6 elements instead of as individual parameters.
    ///
    /// The order of the components corresponds to the
    /// column-major-column-vector matrix notation (the same order
    /// as `Transform2D::new`).
    #[inline]
    pub fn from_array(array: [T; 6]) -> Self {
        Self::new(array[0], array[1], array[2], array[3], array[4], array[5])
    }

    /// Equivalent to `from_array` with elements packed two at a time
    /// in an array of arrays.
    ///
    /// The order of the components corresponds to the
    /// column-major-column-vector matrix notation (the same order
    /// as `Transform3D::new`).
    #[inline]
    pub fn from_arrays(array: [[T; 2]; 3]) -> Self {
        Self::new(
            array[0][0],
            array[0][1],
            array[1][0],
            array[1][1],
            array[2][0],
            array[2][1],
        )
    }

    /// Returns the same transform with a different source unit.
    #[inline]
    pub fn with_source<NewSrc>(&self) -> CoordinateTransform<T, NewSrc, Dst> {
        CoordinateTransform::new(self.m11, self.m12, self.m21, self.m22, self.m31, self.m32)
    }

    /// Returns the same transform with a different destination unit.
    #[inline]
    pub fn with_destination<NewDst>(&self) -> CoordinateTransform<T, Src, NewDst> {
        CoordinateTransform::new(self.m11, self.m12, self.m21, self.m22, self.m31, self.m32)
    }
}

impl<T: NumCast + Copy, Src, Dst> CoordinateTransform<T, Src, Dst> {
    /// Cast from one numeric representation to another, preserving the units.
    #[inline]
    pub fn cast<NewT: NumCast>(&self) -> CoordinateTransform<NewT, Src, Dst> {
        self.try_cast().unwrap()
    }

    /// Fallible cast from one numeric representation to another, preserving the units.
    pub fn try_cast<NewT: NumCast>(&self) -> Option<CoordinateTransform<NewT, Src, Dst>> {
        match (
            NumCast::from(self.m11),
            NumCast::from(self.m12),
            NumCast::from(self.m21),
            NumCast::from(self.m22),
            NumCast::from(self.m31),
            NumCast::from(self.m32),
        ) {
            (Some(m11), Some(m12), Some(m21), Some(m22), Some(m31), Some(m32)) => {
                Some(CoordinateTransform::new(m11, m12, m21, m22, m31, m32))
            }
            _ => None,
        }
    }
}

impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Zero + One,
{
    /// Create an identity matrix:
    ///
    /// ```text
    /// 1 0
    /// 0 1
    /// 0 0
    /// ```
    #[inline]
    pub fn identity() -> Self {
        Self::translation(T::zero(), T::zero())
    }

    /// Intentional not public, because it checks for exact equivalence
    /// while most consumers will probably want some sort of approximate
    /// equivalence to deal with floating-point errors.
    fn is_identity(&self) -> bool
    where
        T: PartialEq,
    {
        *self == Self::identity()
    }
}

/// Methods for combining generic transformations
impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Copy + Add<Output = T> + Mul<Output = T>,
{
    /// Returns the multiplication of the two matrices such that mat's transformation
    /// applies after self's transformation.
    #[must_use]
    pub fn then<NewDst>(
        &self,
        mat: &CoordinateTransform<T, Dst, NewDst>,
    ) -> CoordinateTransform<T, Src, NewDst> {
        CoordinateTransform::new(
            self.m11 * mat.m11 + self.m12 * mat.m21,
            self.m11 * mat.m12 + self.m12 * mat.m22,
            self.m21 * mat.m11 + self.m22 * mat.m21,
            self.m21 * mat.m12 + self.m22 * mat.m22,
            self.m31 * mat.m11 + self.m32 * mat.m21 + mat.m31,
            self.m31 * mat.m12 + self.m32 * mat.m22 + mat.m32,
        )
    }
}

/// Methods for creating and combining translation transformations
impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Zero + One,
{
    /// Create a 2d translation transform:
    ///
    /// ```text
    /// 1 0
    /// 0 1
    /// x y
    /// ```
    #[inline]
    pub fn translation(x: T, y: T) -> Self {
        let _0 = || T::zero();
        let _1 = || T::one();

        Self::new(_1(), _0(), _0(), _1(), x, y)
    }

    /// Applies a translation after self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn then_translate(&self, v: Vector<T, Dst>) -> Self
    where
        T: Copy + Add<Output = T> + Mul<Output = T>,
    {
        self.then(&CoordinateTransform::translation(v.x, v.y))
    }

    /// Applies a translation before self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn pre_translate(&self, v: Vector<T, Src>) -> Self
    where
        T: Copy + Add<Output = T> + Mul<Output = T>,
    {
        CoordinateTransform::translation(v.x, v.y).then(self)
    }
}

/// Methods for creating and combining rotation transformations
impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Zero + Trig,
{
    /// Returns a rotation transform.
    #[inline]
    pub fn rotation(theta: impl Into<Angle<T>>) -> Self {
        let theta = theta.into();
        let _0 = Zero::zero();
        let cos = theta.get().cos();
        let sin = theta.get().sin();
        CoordinateTransform::new(cos, sin, _0 - sin, cos, _0, _0)
    }

    /// Applies a rotation after self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn then_rotate(&self, theta: Angle<T>) -> Self {
        self.then(&CoordinateTransform::rotation(theta))
    }

    /// Applies a rotation before self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn pre_rotate(&self, theta: Angle<T>) -> Self {
        CoordinateTransform::rotation(theta).then(self)
    }
}

/// Methods for creating and combining scale transformations
impl<T, Src, Dst> CoordinateTransform<T, Src, Dst> {
    /// Create a 2d scale transform:
    ///
    /// ```text
    /// x 0
    /// 0 y
    /// 0 0
    /// ```
    #[inline]
    pub fn scale(x: T, y: T) -> Self
    where
        T: Zero,
    {
        let _0 = || Zero::zero();

        Self::new(x, _0(), _0(), y, _0(), _0())
    }

    /// Applies a scale after self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn then_scale(&self, x: T, y: T) -> Self
    where
        T: Copy + Add<Output = T> + Mul<Output = T> + Zero,
    {
        self.then(&CoordinateTransform::scale(x, y))
    }

    /// Applies a scale before self's transformation and returns the resulting transform.
    #[inline]
    #[must_use]
    pub fn pre_scale(&self, x: T, y: T) -> Self
    where
        T: Copy + Mul<Output = T>,
    {
        CoordinateTransform::new(
            self.m11 * x,
            self.m12 * x,
            self.m21 * y,
            self.m22 * y,
            self.m31,
            self.m32,
        )
    }
}

/// Methods for apply transformations to objects
impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Copy + Add<Output = T> + Mul<Output = T>,
{
    /// Returns the given point transformed by this transform.
    #[inline]
    #[must_use]
    pub fn transform_point(&self, point: Pos<T, Src>) -> Pos<T, Dst> {
        Pos::new(
            point.x * self.m11 + point.y * self.m21 + self.m31,
            point.x * self.m12 + point.y * self.m22 + self.m32,
        )
    }

    /// Returns the given vector transformed by this matrix.
    #[inline]
    #[must_use]
    pub fn transform_vector(&self, vec: Vector<T, Src>) -> Vector<T, Dst> {
        Vector::new(
            vec.x * self.m11 + vec.y * self.m21,
            vec.x * self.m12 + vec.y * self.m22,
        )
    }

    /// Returns a rectangle that encompasses the result of transforming the given rectangle by this
    /// transform.
    #[inline]
    #[must_use]
    pub fn outer_transformed_rect(&self, rect: &Rect<T, Src>) -> Rect<T, Dst>
    where
        T: Sub<Output = T> + Zero + PartialOrd,
    {
        let min = rect.min;
        let max = rect.max;
        Rect::from_points(&[
            self.transform_point(min),
            self.transform_point(max),
            self.transform_point(Pos::new(max.x, min.y)),
            self.transform_point(Pos::new(min.x, max.y)),
        ])
    }
}

impl<Src, Dst> CoordinateTransform<f32, Src, Dst> {
    pub fn eigenvalues(&self) -> (Vector, Vector) {
        let discriminant = ((self.m11 - self.m22).powu(2) - 4.0 * self.m12 * self.m21);
        let is_complex = discriminant.is_sign_negative();

        let r = discriminant.abs().sqrt() / 2.0;
        let val = (self.m11 + self.m22) / 2.;

        match is_complex {
            false => (Vector::new(val + r, 0.), Vector::new(val - r, 0.)),
            true => (Vector::new(val, r), Vector::new(val, -r)),
        }
    }

    pub fn scale_factor(&self) -> (f32, f32) {
        (
            (self.m11.powu(2) + self.m12.powu(2)).sqrt(),
            (self.m21.powu(2) + self.m22.powu(2)).sqrt(),
        )
    }
}

impl<T, Src, Dst> CoordinateTransform<T, Src, Dst>
where
    T: Copy + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + PartialEq + Zero + One,
{
    /// Computes and returns the determinant of this transform.
    pub fn determinant(&self) -> T {
        self.m11 * self.m22 - self.m12 * self.m21
    }

    /// Returns whether it is possible to compute the inverse transform.
    #[inline]
    pub fn is_invertible(&self) -> bool {
        self.determinant() != Zero::zero()
    }

    /// Returns the inverse transform if possible.
    #[must_use]
    pub fn inverse(&self) -> Option<CoordinateTransform<T, Dst, Src>> {
        let det = self.determinant();

        let _0: T = Zero::zero();
        let _1: T = One::one();

        if det == _0 {
            return None;
        }

        let inv_det = _1 / det;
        Some(CoordinateTransform::new(
            inv_det * self.m22,
            inv_det * (_0 - self.m12),
            inv_det * (_0 - self.m21),
            inv_det * self.m11,
            inv_det * (self.m21 * self.m32 - self.m22 * self.m31),
            inv_det * (self.m31 * self.m12 - self.m11 * self.m32),
        ))
    }
}

impl<T, Src, Dst> Default for CoordinateTransform<T, Src, Dst>
where
    T: Zero + One,
{
    /// Returns the [identity transform](#method.identity).
    fn default() -> Self {
        Self::identity()
    }
}

impl<T, Src, Dst> fmt::Debug for CoordinateTransform<T, Src, Dst>
where
    T: Copy + fmt::Debug + PartialEq + One + Zero,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_identity() {
            write!(f, "[I]")
        } else {
            self.to_array().fmt(f)
        }
    }
}

impl<Src> Into<accesskit::Affine> for CoordinateTransform<f64, Src, Src> {
    fn into(self) -> accesskit::Affine {
        accesskit::Affine::new(self.to_array())
    }
}

impl<Src> Into<accesskit::Affine> for CoordinateTransform<f32, Src, Src> {
    fn into(self) -> accesskit::Affine {
        accesskit::Affine::new(self.to_array().map(|x| x as f64))
    }
}

impl<Src> Into<Box<accesskit::Affine>> for CoordinateTransform<f32, Src, Src> {
    fn into(self) -> Box<accesskit::Affine> {
        Box::new(self.into())
    }
}

impl<Src> Into<Box<accesskit::Affine>> for CoordinateTransform<f64, Src, Src> {
    fn into(self) -> Box<accesskit::Affine> {
        Box::new(self.into())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TransformationList {
    pub transformations: Vec<CoordinateTransform>,
    pub(crate) transformation_inverses: Vec<CoordinateTransform>,
    pub(crate) determinants: Vec<f32>,
}

impl Default for TransformationList {
    fn default() -> Self {
        Self {
            transformations: vec![CoordinateTransform::identity()],
            transformation_inverses: vec![CoordinateTransform::identity()],
            determinants: vec![1.],
        }
    }
}

impl TransformationList {
    pub fn push_transform(&mut self, transform: CoordinateTransform) -> usize {
        let idx = self.transformations.len();

        self.transformations.push(transform);
        self.transformation_inverses
            .push(transform.inverse().unwrap_or(CoordinateTransform::zeroed()));
        self.determinants.push(transform.determinant());

        idx
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> &CoordinateTransform {
        &self.transformations[idx]
    }

    pub fn get_inverse(&mut self, idx: usize) -> CoordinateTransform {
        // self.transformation_cache[idx].get_inverse(&self.transformations[idx])
        self.transformation_inverses[idx]
    }

    pub fn get_determinant(&mut self, idx: usize) -> f32 {
        self.determinants[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factoe() {
        let (sx, sy) = CoordinateTransform::<f32, LogicalUnit, LogicalUnit>::scale(5., 1.)
            .then_rotate(Angle::degrees(90.))
            .scale_factor();

        assert_eq!(sx, 5.);
        assert_eq!(sy, 1.);
    }
}
