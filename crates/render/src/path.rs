//! Paths: outlines made of straight lines and Bézier curves, filled or stroked by the canvas, with the rule that says
//! which parts of a path are inside and how the ends of a line look.

use crate::geometry::{Point, Rect, Transform};

/// How far along its tangent a cubic Bézier curve puts its control points to follow a quarter of a circle closely:
/// `4/3 × (√2 − 1)`, which makes the curve meet the circle at its ends and at its middle.
const QUARTER_CIRCLE: f32 = 0.552_284_8;

/// Which parts of an outline that crosses itself are inside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    /// A point is inside when the outline winds around it at least once in either direction: contours in the same
    /// direction add up, contours in opposite directions cut holes.
    #[default]
    NonZero,
    /// A point is inside when the outline crosses any ray from it an odd number of times: every contour inside
    /// another cuts a hole, whatever its direction.
    EvenOdd,
}

/// How the ends of an open contour are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    /// The stroke ends square at the end point.
    #[default]
    Butt,
    /// The stroke ends with a half circle around the end point.
    Round,
    /// The stroke ends square, half its width past the end point.
    Square,
}

/// One step of a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Starts a new contour at a point.
    MoveTo(Point),
    /// A straight line to a point.
    LineTo(Point),
    /// A quadratic Bézier curve through a control point to a point.
    QuadraticTo(Point, Point),
    /// A cubic Bézier curve through two control points to a point.
    CubicTo(Point, Point, Point),
    /// A straight line back to where the contour started, which closes it.
    Close,
}

/// An outline of one or more contours, each a series of straight lines and curves from a starting point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    /// The steps, in order.
    commands: Vec<Command>,
}

impl Path {
    /// A path with nothing in it.
    #[must_use]
    pub fn new() -> Path {
        Path::default()
    }

    /// The steps of the path, in order.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Whether the path has no step.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Starts a new contour at a point.
    pub fn move_to(&mut self, point: Point) -> &mut Path {
        self.commands.push(Command::MoveTo(point));
        self
    }

    /// A straight line to a point.
    pub fn line_to(&mut self, point: Point) -> &mut Path {
        self.commands.push(Command::LineTo(point));
        self
    }

    /// A quadratic Bézier curve through a control point to a point.
    pub fn quadratic_to(&mut self, control: Point, point: Point) -> &mut Path {
        self.commands.push(Command::QuadraticTo(control, point));
        self
    }

    /// A cubic Bézier curve through two control points to a point.
    pub fn cubic_to(&mut self, first_control: Point, second_control: Point, point: Point) -> &mut Path {
        self.commands
            .push(Command::CubicTo(first_control, second_control, point));
        self
    }

    /// Closes the contour with a straight line back to where it started.
    pub fn close(&mut self) -> &mut Path {
        self.commands.push(Command::Close);
        self
    }

    /// Adds the steps of another path after these.
    pub fn extend(&mut self, other: &Path) -> &mut Path {
        self.commands.extend_from_slice(&other.commands);
        self
    }

    /// The same path with every point moved by a transform.
    #[must_use]
    pub fn transformed(&self, transform: Transform) -> Path {
        let apply = |point| transform.apply(point);
        let commands = self
            .commands
            .iter()
            .map(|command| match *command {
                Command::MoveTo(point) => Command::MoveTo(apply(point)),
                Command::LineTo(point) => Command::LineTo(apply(point)),
                Command::QuadraticTo(control, point) => Command::QuadraticTo(apply(control), apply(point)),
                Command::CubicTo(first, second, point) => Command::CubicTo(apply(first), apply(second), apply(point)),
                Command::Close => Command::Close,
            })
            .collect();
        Path { commands }
    }

    /// The smallest rectangle that holds every point of the path, control points included, which is never smaller
    /// than the curves themselves; nothing for an empty path.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.commands.iter().flat_map(|command| match *command {
            Command::MoveTo(point) | Command::LineTo(point) => [Some(point), None, None],
            Command::QuadraticTo(control, point) => [Some(control), Some(point), None],
            Command::CubicTo(first, second, point) => [Some(first), Some(second), Some(point)],
            Command::Close => [None, None, None],
        });
        let first = points.find_map(|point| point)?;
        let (mut low, mut high) = (first, first);
        for point in points.flatten() {
            low = Point::new(low.x.min(point.x), low.y.min(point.y));
            high = Point::new(high.x.max(point.x), high.y.max(point.y));
        }
        Some(Rect::new(low.x, low.y, high.x - low.x, high.y - low.y))
    }

    /// A rectangle, clockwise from its top left corner.
    #[must_use]
    pub fn rect(rect: Rect) -> Path {
        let mut path = Path::new();
        let _ = path
            .move_to(Point::new(rect.x, rect.y))
            .line_to(Point::new(rect.right(), rect.y))
            .line_to(Point::new(rect.right(), rect.bottom()))
            .line_to(Point::new(rect.x, rect.bottom()))
            .close();
        path
    }

    /// A rectangle with its corners rounded to a radius, which is made smaller when the rectangle cannot hold it.
    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f32) -> Path {
        let radius = radius.min(rect.width * 0.5).min(rect.height * 0.5).max(0.0);
        if radius <= 0.0 {
            return Path::rect(rect);
        }
        let handle = radius * (1.0 - QUARTER_CIRCLE);
        let (left, top, right, bottom) = (rect.x, rect.y, rect.right(), rect.bottom());
        let mut path = Path::new();
        let _ = path
            .move_to(Point::new(left + radius, top))
            .line_to(Point::new(right - radius, top))
            .cubic_to(
                Point::new(right - handle, top),
                Point::new(right, top + handle),
                Point::new(right, top + radius),
            )
            .line_to(Point::new(right, bottom - radius))
            .cubic_to(
                Point::new(right, bottom - handle),
                Point::new(right - handle, bottom),
                Point::new(right - radius, bottom),
            )
            .line_to(Point::new(left + radius, bottom))
            .cubic_to(
                Point::new(left + handle, bottom),
                Point::new(left, bottom - handle),
                Point::new(left, bottom - radius),
            )
            .line_to(Point::new(left, top + radius))
            .cubic_to(
                Point::new(left, top + handle),
                Point::new(left + handle, top),
                Point::new(left + radius, top),
            )
            .close();
        path
    }

    /// A circle, as four cubic curves clockwise from its rightmost point.
    #[must_use]
    pub fn circle(center: Point, radius: f32) -> Path {
        Path::ellipse(center, Point::new(radius, radius))
    }

    /// An ellipse with its axes along the axes of the screen, from its center and its two radii.
    #[must_use]
    pub fn ellipse(center: Point, radii: Point) -> Path {
        let handles = radii * QUARTER_CIRCLE;
        // Each quarter goes from one end of an axis to the next, its control points along the tangents at both ends.
        let at = |x: f32, y: f32| Point::new(center.x + x, center.y + y);
        let mut path = Path::new();
        let _ = path
            .move_to(at(radii.x, 0.0))
            .cubic_to(at(radii.x, handles.y), at(handles.x, radii.y), at(0.0, radii.y))
            .cubic_to(at(-handles.x, radii.y), at(-radii.x, handles.y), at(-radii.x, 0.0))
            .cubic_to(at(-radii.x, -handles.y), at(-handles.x, -radii.y), at(0.0, -radii.y))
            .cubic_to(at(handles.x, -radii.y), at(radii.x, -handles.y), at(radii.x, 0.0))
            .close();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_know_their_bounds_and_move_with_a_transform() {
        let path = Path::circle(Point::new(10.0, 10.0), 5.0);
        assert_eq!(path.bounds(), Some(Rect::new(5.0, 5.0, 10.0, 10.0)));
        let moved = path.transformed(Transform::translation(1.0, 2.0));
        assert_eq!(moved.bounds(), Some(Rect::new(6.0, 7.0, 10.0, 10.0)));
        assert_eq!(Path::new().bounds(), None);
        assert_eq!(
            Path::rounded_rect(Rect::new(0.0, 0.0, 4.0, 4.0), 0.0),
            Path::rect(Rect::new(0.0, 0.0, 4.0, 4.0))
        );
    }
}
