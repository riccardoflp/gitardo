//! Points, rectangles and transforms: the floating-point geometry of drawing, and the whole-pixel rectangles of the
//! frame buffer.

use std::ops::{Add, Mul, Sub};

use crate::number;

/// A point, or a vector between two points, in pixels from the top left, `y` growing downwards.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// The distance to the right.
    pub x: f32,
    /// The distance downwards.
    pub y: f32,
}

impl Point {
    /// The origin, or the vector of no length.
    pub const ZERO: Point = Point::new(0.0, 0.0);

    /// A point from its two coordinates.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    /// The length of the vector from the origin to this point.
    #[must_use]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// The dot product with another vector.
    #[must_use]
    pub fn dot(self, other: Point) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// The cross product with another vector: positive when `other` turns clockwise from this one on the screen.
    #[must_use]
    pub fn cross(self, other: Point) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// The vector of the same direction and length 1, or nothing when this one has no length.
    #[must_use]
    pub fn normalized(self) -> Option<Point> {
        let length = self.length();
        (length > f32::EPSILON).then(|| self * (1.0 / length))
    }

    /// The vector turned a quarter turn clockwise on the screen, where `y` grows downwards.
    #[must_use]
    pub fn perpendicular(self) -> Point {
        Point::new(-self.y, self.x)
    }

    /// The point halfway to another.
    #[must_use]
    pub fn midpoint(self, other: Point) -> Point {
        (self + other) * 0.5
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Point {
    type Output = Point;

    fn mul(self, factor: f32) -> Point {
        Point::new(self.x * factor, self.y * factor)
    }
}

/// A rectangle with its sides along the axes, in floating-point pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The left side.
    pub x: f32,
    /// The top side.
    pub y: f32,
    /// The width, never negative for a rectangle that holds something.
    pub width: f32,
    /// The height, never negative for a rectangle that holds something.
    pub height: f32,
}

impl Rect {
    /// A rectangle from its top left corner and its size.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect { x, y, width, height }
    }

    /// The right side.
    #[must_use]
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// The bottom side.
    #[must_use]
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// The point in the middle.
    #[must_use]
    pub fn center(self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Whether the rectangle holds no area.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Whether a point is inside, the left and top sides included and the right and bottom ones not.
    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }

    /// The rectangle made smaller by a distance on every side, or larger when the distance is negative.
    #[must_use]
    pub fn inset(self, distance: f32) -> Rect {
        Rect::new(
            self.x + distance,
            self.y + distance,
            (self.width - 2.0 * distance).max(0.0),
            (self.height - 2.0 * distance).max(0.0),
        )
    }

    /// The part both rectangles cover, empty when they do not meet.
    #[must_use]
    pub fn intersection(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }
}

/// A rectangle of whole pixels of the frame buffer: the columns from `left` to `right` and the rows from `top` to
/// `bottom`, the right and bottom ones excluded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelRect {
    /// The first column.
    pub left: i32,
    /// The first row.
    pub top: i32,
    /// The column after the last one.
    pub right: i32,
    /// The row after the last one.
    pub bottom: i32,
}

impl PixelRect {
    /// A rectangle from its sides.
    #[must_use]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
        PixelRect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// The whole of a frame of a width and a height.
    #[must_use]
    pub fn of_size(width: u32, height: u32) -> PixelRect {
        PixelRect::new(0, 0, number::coordinate(width), number::coordinate(height))
    }

    /// The smallest rectangle of whole pixels that holds every part of a floating-point one.
    #[must_use]
    pub fn enclosing(rect: Rect) -> PixelRect {
        PixelRect::new(
            number::floor(rect.x),
            number::floor(rect.y),
            number::ceil(rect.right()),
            number::ceil(rect.bottom()),
        )
    }

    /// The number of columns, zero when empty.
    #[must_use]
    pub fn width(self) -> u32 {
        number::size(self.right.saturating_sub(self.left))
    }

    /// The number of rows, zero when empty.
    #[must_use]
    pub fn height(self) -> u32 {
        number::size(self.bottom.saturating_sub(self.top))
    }

    /// The number of pixels.
    #[must_use]
    pub fn area(self) -> u64 {
        u64::from(self.width()) * u64::from(self.height())
    }

    /// Whether it holds no pixel.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// The pixels both rectangles hold; empty when they do not meet.
    #[must_use]
    pub fn intersection(self, other: PixelRect) -> PixelRect {
        PixelRect::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    /// The smallest rectangle that holds both; an empty one counts for nothing.
    #[must_use]
    pub fn union(self, other: PixelRect) -> PixelRect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        PixelRect::new(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Whether every pixel of the other rectangle is in this one.
    #[must_use]
    pub fn contains(self, other: PixelRect) -> bool {
        other.is_empty()
            || (other.left >= self.left
                && other.top >= self.top
                && other.right <= self.right
                && other.bottom <= self.bottom)
    }

    /// Whether the two share a pixel or a side, so that their union adds no row or column between them.
    #[must_use]
    pub fn touches(self, other: PixelRect) -> bool {
        self.left <= other.right && other.left <= self.right && self.top <= other.bottom && other.top <= self.bottom
    }

    /// The rectangle moved by a number of columns and rows.
    #[must_use]
    pub fn translated(self, columns: i32, rows: i32) -> PixelRect {
        PixelRect::new(
            self.left + columns,
            self.top + rows,
            self.right + columns,
            self.bottom + rows,
        )
    }
}

/// An affine transform: where the unit vectors of the axes go, and where the origin goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Where the vector `(1, 0)` goes.
    pub x_axis: Point,
    /// Where the vector `(0, 1)` goes.
    pub y_axis: Point,
    /// Where the origin goes.
    pub translation: Point,
}

impl Transform {
    /// The transform that moves nothing.
    pub const IDENTITY: Transform = Transform {
        x_axis: Point::new(1.0, 0.0),
        y_axis: Point::new(0.0, 1.0),
        translation: Point::ZERO,
    };

    /// A transform that moves every point by a vector.
    #[must_use]
    pub const fn translation(x: f32, y: f32) -> Transform {
        Transform {
            translation: Point::new(x, y),
            ..Transform::IDENTITY
        }
    }

    /// A transform that scales both axes from the origin, each by its factor.
    #[must_use]
    pub const fn scale(x: f32, y: f32) -> Transform {
        Transform {
            x_axis: Point::new(x, 0.0),
            y_axis: Point::new(0.0, y),
            translation: Point::ZERO,
        }
    }

    /// Where a point goes.
    #[must_use]
    pub fn apply(self, point: Point) -> Point {
        self.x_axis * point.x + self.y_axis * point.y + self.translation
    }

    /// This transform done after another: the result applies `first`, then this one.
    #[must_use]
    pub fn after(self, first: Transform) -> Transform {
        let origin = self.apply(Point::ZERO);
        Transform {
            x_axis: self.apply(first.x_axis) - origin,
            y_axis: self.apply(first.y_axis) - origin,
            translation: self.apply(first.translation),
        }
    }

    /// The factor by which the transform scales lengths, taken as the larger of its two axes; used to keep the width
    /// of strokes and the flattening of curves right after scaling.
    #[must_use]
    pub fn largest_scale(self) -> f32 {
        self.x_axis.length().max(self.y_axis.length())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_compose_in_the_order_they_are_done() {
        let scale_then_move = Transform::translation(10.0, 20.0).after(Transform::scale(2.0, 3.0));
        assert_eq!(scale_then_move.apply(Point::new(1.0, 1.0)), Point::new(12.0, 23.0));
        let move_then_scale = Transform::scale(2.0, 3.0).after(Transform::translation(10.0, 20.0));
        assert_eq!(move_then_scale.apply(Point::new(1.0, 1.0)), Point::new(22.0, 63.0));
    }

    #[test]
    fn pixel_rectangles_meet_join_and_enclose() {
        let first = PixelRect::new(0, 0, 10, 10);
        let second = PixelRect::new(5, 5, 20, 8);
        assert_eq!(first.intersection(second), PixelRect::new(5, 5, 10, 8));
        assert_eq!(first.union(second), PixelRect::new(0, 0, 20, 10));
        assert_eq!(first.union(PixelRect::default()), first);
        assert!(first.touches(PixelRect::new(10, 0, 12, 2)));
        assert!(!first.touches(PixelRect::new(11, 0, 12, 2)));
        assert_eq!(
            PixelRect::enclosing(Rect::new(0.5, -0.5, 1.0, 1.0)),
            PixelRect::new(0, -1, 2, 1)
        );
        assert_eq!(PixelRect::new(3, 3, 1, 1).area(), 0);
    }
}
