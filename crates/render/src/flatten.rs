//! Curves turned into straight lines close enough to them that the difference cannot be seen.

use crate::geometry::Point;
use crate::number;
use crate::path::{Command, Path};

/// How far, in pixels, the straight lines may stray from the curve they stand for.
pub(crate) const TOLERANCE: f32 = 0.1;

/// The most lines a curve is cut into, so that a curve with wild control points cannot take all the memory.
const MOST_LINES_PER_CURVE: usize = 256;

/// A contour made of straight lines: its points in order, and whether it closes on itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Polyline {
    /// The points, from the first to the last.
    pub(crate) points: Vec<Point>,
    /// Whether the last point joins the first: a contour closed by the path, rather than one left open.
    pub(crate) is_closed: bool,
}

/// The contours of a path as straight lines, in the coordinates of the path.
pub(crate) fn flatten(path: &Path) -> Vec<Polyline> {
    let mut contours = Vec::new();
    let mut current = Polyline::default();
    let mut start = Point::ZERO;
    for command in path.commands() {
        let last = current.points.last().copied().unwrap_or(start);
        match *command {
            Command::MoveTo(point) => {
                finish(&mut contours, &mut current);
                start = point;
                current.points.push(point);
            }
            Command::LineTo(point) => push_after(&mut current, start, point),
            Command::QuadraticTo(control, point) => {
                let count = quadratic_lines(last, control, point);
                for step in 1..=count {
                    let time = number::float_of_count(step) / number::float_of_count(count);
                    push_after(&mut current, start, quadratic_at(last, control, point, time));
                }
            }
            Command::CubicTo(first, second, point) => {
                let count = cubic_lines([last, first, second, point]);
                for step in 1..=count {
                    let time = number::float_of_count(step) / number::float_of_count(count);
                    push_after(&mut current, start, cubic_at([last, first, second, point], time));
                }
            }
            Command::Close => {
                current.is_closed = true;
                finish(&mut contours, &mut current);
            }
        }
    }
    finish(&mut contours, &mut current);
    contours
}

/// Adds a point to the contour, starting it at the starting point first when a line comes before any move.
fn push_after(contour: &mut Polyline, start: Point, point: Point) {
    if contour.points.is_empty() {
        contour.points.push(start);
    }
    contour.points.push(point);
}

/// Ends the contour being built, keeping it when it has at least a point; the next one starts where it started, as a
/// path goes on after it is closed.
fn finish(contours: &mut Vec<Polyline>, current: &mut Polyline) {
    if !current.points.is_empty() {
        contours.push(std::mem::take(current));
    }
}

/// How many lines a quadratic curve needs: with `n` lines of equal steps of its parameter, the distance to the curve
/// is at most `|p0 − 2p1 + p2| / (4n²)`, a quarter of the bound of its second derivative over `n²`.
fn quadratic_lines(start: Point, control: Point, end: Point) -> usize {
    let bend = (start - control * 2.0 + end).length();
    lines_for(bend * 0.25)
}

/// How many lines a cubic curve needs: with `n` lines, the distance to the curve is at most `M / 8n²`, where `M`, the
/// largest second derivative, is at most six times the larger of `|p0 − 2p1 + p2|` and `|p1 − 2p2 + p3|`.
fn cubic_lines(points: [Point; 4]) -> usize {
    let [start, first, second, end] = points;
    let bend = (start - first * 2.0 + second)
        .length()
        .max((first - second * 2.0 + end).length());
    lines_for(bend * 0.75)
}

/// The number of lines `n` that keeps `factor / n²` within the tolerance.
fn lines_for(factor: f32) -> usize {
    let count = number::ceil((factor / TOLERANCE).sqrt()).max(1);
    number::index(count).min(MOST_LINES_PER_CURVE)
}

/// The point of a quadratic curve at a time from 0 to 1.
fn quadratic_at(start: Point, control: Point, end: Point, time: f32) -> Point {
    let rest = 1.0 - time;
    start * (rest * rest) + control * (2.0 * rest * time) + end * (time * time)
}

/// The point of a cubic curve at a time from 0 to 1.
fn cubic_at(points: [Point; 4], time: f32) -> Point {
    let [start, first, second, end] = points;
    let rest = 1.0 - time;
    start * (rest * rest * rest)
        + first * (3.0 * rest * rest * time)
        + second * (3.0 * rest * time * time)
        + end * (time * time * time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    #[test]
    fn lines_stay_lines_and_contours_close() {
        let contours = flatten(&Path::rect(Rect::new(0.0, 0.0, 2.0, 3.0)));
        assert_eq!(contours.len(), 1);
        assert!(contours[0].is_closed);
        assert_eq!(contours[0].points.len(), 4);
    }

    #[test]
    fn curves_become_lines_near_the_curve() {
        let contours = flatten(&Path::circle(Point::new(0.0, 0.0), 50.0));
        let points = &contours[0].points;
        assert!(points.len() > 16, "{} points", points.len());
        for pair in points.windows(2) {
            let middle = pair[0].midpoint(pair[1]);
            assert!((middle.length() - 50.0).abs() < TOLERANCE * 2.0, "{middle:?}");
        }
    }

    #[test]
    fn an_open_contour_starts_where_the_path_moved_to() {
        let mut path = Path::new();
        let _ = path
            .move_to(Point::new(1.0, 1.0))
            .quadratic_to(Point::new(5.0, 1.0), Point::new(5.0, 5.0));
        let contours = flatten(&path);
        assert!(!contours[0].is_closed);
        assert_eq!(contours[0].points.first(), Some(&Point::new(1.0, 1.0)));
        assert_eq!(contours[0].points.last(), Some(&Point::new(5.0, 5.0)));
    }
}
