//! Strokes: the outline of the area within half a width of a path, which the rasterizer then fills.
//!
//! Each contour is offset by half the width on both of its sides. Where two lines meet, the outer side goes round the
//! corner on an arc of a circle and the inner side takes the point where its two offset lines cross. Arcs are cut by
//! halving the angle between two unit vectors until the chord is within the tolerance, so that no trigonometry, and
//! nothing that may differ between machines, is used.

use crate::flatten::{Polyline, TOLERANCE};
use crate::geometry::Point;

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

/// The outline of the stroke of a path made of straight lines, as closed contours to fill with the non-zero rule.
pub(crate) fn stroke_outline(contours: &[Polyline], width: f32, cap: LineCap) -> Vec<Polyline> {
    let radius = width * 0.5;
    let mut outline = Vec::new();
    for contour in contours {
        let points = without_repeats(&contour.points, contour.is_closed);
        match points.len() {
            0 => {}
            1 => outline.extend(dot(points.first().copied().unwrap_or_default(), radius, cap)),
            _ if contour.is_closed => outline.extend(closed_sides(&points, radius)),
            _ => outline.push(open_outline(&points, radius, cap)),
        }
    }
    outline
}

/// The points of a contour without those that repeat the one before, nor, for a closed one, a last point that repeats
/// the first.
fn without_repeats(points: &[Point], is_closed: bool) -> Vec<Point> {
    let mut kept: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if kept.last().is_none_or(|last| (*point - *last).length() > 1e-4) {
            kept.push(*point);
        }
    }
    if is_closed
        && kept.len() > 1
        && kept
            .first()
            .zip(kept.last())
            .is_some_and(|(first, last)| (*first - *last).length() <= 1e-4)
    {
        let _ = kept.pop();
    }
    kept
}

/// A stroke of no length: a dot with a round cap, a square with a square cap, nothing with a butt cap.
fn dot(center: Point, radius: f32, cap: LineCap) -> Option<Polyline> {
    let right = Point::new(1.0, 0.0);
    let mut points = Vec::new();
    match cap {
        LineCap::Butt => return None,
        LineCap::Round => {
            let down = right.perpendicular();
            for (from, to) in [
                (right, down),
                (down, right * -1.0),
                (right * -1.0, down * -1.0),
                (down * -1.0, right),
            ] {
                points.push(center + from * radius);
                arc(
                    &mut points,
                    Corner {
                        center,
                        from,
                        to,
                        radius,
                    },
                );
            }
        }
        LineCap::Square => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                points.push(center + Point::new(x, y) * radius);
            }
        }
    }
    Some(Polyline {
        points,
        is_closed: true,
    })
}

/// The outline of an open contour: along one side, round the end, back along the other side, round the start.
fn open_outline(points: &[Point], radius: f32, cap: LineCap) -> Polyline {
    let reversed: Vec<Point> = points.iter().rev().copied().collect();
    let mut outline = Vec::new();
    side(&mut outline, points, radius);
    end_cap(&mut outline, points, radius, cap);
    side(&mut outline, &reversed, radius);
    end_cap(&mut outline, &reversed, radius, cap);
    Polyline {
        points: outline,
        is_closed: true,
    }
}

/// The two sides of a closed contour, each a closed contour, the second in the opposite direction so that the stroke
/// between them is filled and what is inside is not.
fn closed_sides(points: &[Point], radius: f32) -> [Polyline; 2] {
    let reversed: Vec<Point> = points.iter().rev().copied().collect();
    [points, &reversed].map(|contour| {
        let mut outline = Vec::new();
        let count = contour.len();
        for index in 0..count {
            let before = contour.get((index + count - 1) % count);
            let at = contour.get(index);
            let after = contour.get((index + 1) % count);
            if let (Some(before), Some(at), Some(after)) = (before, at, after) {
                join(&mut outline, [*before, *at, *after], radius);
            }
        }
        Polyline {
            points: outline,
            is_closed: true,
        }
    })
}

/// The left side of an open contour, offset by the radius, from its first point to its last.
fn side(outline: &mut Vec<Point>, points: &[Point], radius: f32) {
    if let (Some(first), Some(second)) = (points.first(), points.get(1)) {
        outline.push(*first + left_normal(*first, *second) * radius);
    }
    for triple in points.windows(3) {
        if let [before, at, after] = triple {
            join(outline, [*before, *at, *after], radius);
        }
    }
    if let [.., before, last] = points {
        outline.push(*last + left_normal(*before, *last) * radius);
    }
}

/// The end of a stroke at the last point of a contour, from its left side round to its right side, going forwards.
fn end_cap(outline: &mut Vec<Point>, points: &[Point], radius: f32, cap: LineCap) {
    let [.., before, last] = points else {
        return;
    };
    let normal = left_normal(*before, *last);
    let forward = normal.perpendicular();
    match cap {
        LineCap::Butt => {}
        LineCap::Round => {
            arc(
                outline,
                Corner {
                    center: *last,
                    from: normal,
                    to: forward,
                    radius,
                },
            );
            outline.push(*last + forward * radius);
            arc(
                outline,
                Corner {
                    center: *last,
                    from: forward,
                    to: normal * -1.0,
                    radius,
                },
            );
        }
        LineCap::Square => {
            outline.push(*last + (normal + forward) * radius);
            outline.push(*last + (forward - normal) * radius);
        }
    }
}

/// The unit vector a quarter turn left of the direction from one point to another, as one walks along the line.
fn left_normal(from: Point, to: Point) -> Point {
    (to - from).normalized().unwrap_or(Point::new(1.0, 0.0)).perpendicular() * -1.0
}

/// The left side of a stroke where two lines meet at a point: round the corner when the left side is outside the
/// turn, at the crossing of the two offset lines when it is inside.
fn join(outline: &mut Vec<Point>, points: [Point; 3], radius: f32) {
    let [before, at, after] = points;
    let incoming = left_normal(before, at);
    let outgoing = left_normal(at, after);
    let direction_out = outgoing.perpendicular();
    let turn = incoming.cross(outgoing);
    if turn.abs() < 1e-6 && incoming.dot(outgoing) > 0.0 {
        outline.push(at + outgoing * radius);
        return;
    }
    let is_inside = direction_out.dot(incoming) > 0.0;
    if is_inside {
        let middle = (incoming + outgoing).normalized().unwrap_or(incoming);
        let cosine = middle.dot(incoming).max(1e-3);
        let reach = radius * (1.0 - cosine * cosine).sqrt() / cosine;
        let shortest = (at - before).length().min((after - at).length());
        if reach <= shortest {
            outline.push(at + middle * (radius / cosine));
        } else {
            outline.push(at + incoming * radius);
            outline.push(at + outgoing * radius);
        }
        return;
    }
    outline.push(at + incoming * radius);
    if incoming.dot(outgoing) < -0.999 {
        // The line turns right back: the arc goes round the front of the point, through the incoming direction.
        let forward = incoming.perpendicular();
        arc(
            outline,
            Corner {
                center: at,
                from: incoming,
                to: forward,
                radius,
            },
        );
        outline.push(at + forward * radius);
        arc(
            outline,
            Corner {
                center: at,
                from: forward,
                to: outgoing,
                radius,
            },
        );
    } else {
        arc(
            outline,
            Corner {
                center: at,
                from: incoming,
                to: outgoing,
                radius,
            },
        );
    }
    outline.push(at + outgoing * radius);
}

/// An arc of a circle between two directions less than half a turn apart.
#[derive(Clone, Copy)]
struct Corner {
    /// The center of the circle.
    center: Point,
    /// The unit vector of the start of the arc.
    from: Point,
    /// The unit vector of the end of the arc.
    to: Point,
    /// The radius.
    radius: f32,
}

/// The points strictly between the ends of an arc, each chord within the tolerance of the circle.
fn arc(outline: &mut Vec<Point>, corner: Corner) {
    let Some(middle) = (corner.from + corner.to).normalized() else {
        return;
    };
    // The chord between the two ends strays from the circle by the radius times one minus the cosine of half the
    // angle, which is the dot product of an end with the middle.
    let sagitta = corner.radius * (1.0 - middle.dot(corner.from));
    if sagitta <= TOLERANCE {
        return;
    }
    arc(outline, Corner { to: middle, ..corner });
    outline.push(corner.center + middle * corner.radius);
    arc(outline, Corner { from: middle, ..corner });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(points: &[(f32, f32)], is_closed: bool) -> Polyline {
        Polyline {
            points: points.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
            is_closed,
        }
    }

    #[test]
    fn a_straight_line_with_butt_caps_is_a_rectangle() {
        let outline = stroke_outline(&[line(&[(0.0, 0.0), (10.0, 0.0)], false)], 2.0, LineCap::Butt);
        assert_eq!(
            outline[0].points,
            vec![
                Point::new(0.0, -1.0),
                Point::new(10.0, -1.0),
                Point::new(10.0, 1.0),
                Point::new(0.0, 1.0)
            ]
        );
    }

    #[test]
    fn round_caps_reach_half_the_width_past_the_ends() {
        let outline = stroke_outline(&[line(&[(0.0, 0.0), (10.0, 0.0)], false)], 4.0, LineCap::Round);
        let rightmost = outline[0].points.iter().map(|point| point.x).fold(f32::MIN, f32::max);
        assert!((rightmost - 12.0).abs() < 1e-4, "{rightmost}");
        assert!(
            outline[0]
                .points
                .iter()
                .all(
                    |point| (point.length().min((*point - Point::new(10.0, 0.0)).length())) <= 2.0 + 1e-4
                        || point.x > 0.0 && point.x < 10.0
                )
        );
    }

    #[test]
    fn a_closed_contour_has_two_sides_and_a_point_a_dot() {
        let square = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], true);
        assert_eq!(stroke_outline(&[square], 2.0, LineCap::Butt).len(), 2);
        assert_eq!(
            stroke_outline(&[line(&[(1.0, 1.0)], false)], 2.0, LineCap::Butt).len(),
            0
        );
        assert_eq!(
            stroke_outline(&[line(&[(1.0, 1.0)], false)], 2.0, LineCap::Square)[0]
                .points
                .len(),
            4
        );
    }
}
