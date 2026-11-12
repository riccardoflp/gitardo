//! The lines of a graph of commits: from a lane at one height to a lane at another, straight down when the lanes are
//! the same and a curve that leaves and arrives upright when they are not, whole or in dashes; and the bar that runs
//! along the top of a list while more of it is read.

use std::time::Duration;

use std::f32::consts::{FRAC_PI_2, PI};

use render::{LineCap, Path, Point, Rect, Stroke};

use super::metrics;
use super::progress::arc_path;
use crate::frame::Ui;

/// How many straight pieces a curve is cut into to be drawn in dashes.
const CURVE_PIECES: u16 = 24;
/// How long the bar that runs takes to cross its track once.
const RUN: Duration = Duration::from_millis(1_200);
/// How many steps a run of the bar is drawn in: a frame for each.
const RUN_STEPS: u16 = 40;
/// How much of its track the running part of the bar covers.
const RUN_PART: f32 = 0.3;

impl Ui<'_> {
    /// Draws a line of the graph from one lane to another, downward: straight when both are at the same place across,
    /// otherwise a curve that leaves `from` and reaches `to` upright, so that it meets the straight lines above and
    /// under it without a corner.
    pub fn lane_line(&mut self, from: Point, to: Point, stroke: Stroke) {
        if (from.x - to.x).abs() < f32::EPSILON {
            self.line(from, to, stroke);
            return;
        }
        let (first, second) = controls(from, to);
        let mut path = Path::new();
        let _ = path.move_to(from).cubic_to(first, second, to);
        self.stroke_path(path, stroke);
    }

    /// Draws a line of the graph as [`Ui::lane_line`] does, in dashes of a length with gaps of a length.
    pub fn dashed_lane_line(&mut self, from: Point, to: Point, stroke: Stroke, (dash, gap): (f32, f32)) {
        let stroke = Stroke {
            cap: LineCap::Butt,
            ..stroke
        };
        let (first, second) = controls(from, to);
        let points: Vec<Point> = (0..=CURVE_PIECES)
            .map(|piece| cubic_at((from, first, second, to), f32::from(piece) / f32::from(CURVE_PIECES)))
            .collect();
        let period = (dash + gap).max(f32::EPSILON);
        let mut travelled = 0.0_f32;
        for pair in points.windows(2) {
            let [start, end] = pair else {
                continue;
            };
            let length = (end.x - start.x).hypot(end.y - start.y);
            let mut along = 0.0_f32;
            while along < length {
                let phase = (travelled + along) % period;
                let left_in_phase = if phase < dash { dash - phase } else { period - phase };
                let step = left_in_phase.min(length - along);
                if phase < dash {
                    let piece_start = lerp(*start, *end, along / length);
                    let piece_end = lerp(*start, *end, (along + step) / length);
                    self.line(piece_start, piece_end, stroke);
                }
                along += step.max(f32::EPSILON);
            }
            travelled += length;
        }
    }

    /// Draws a ring in dashes, such as the dot of a commit not made yet: the draft, or a fold.
    pub fn dashed_ring(&mut self, center: Point, radius: f32, (stroke, dashes): (Stroke, u8)) {
        let stroke = Stroke {
            cap: LineCap::Butt,
            ..stroke
        };
        let step = 2.0 * PI / f32::from(dashes.max(1));
        for dash in 0..dashes {
            let start = f32::from(dash) * step;
            self.stroke_path(arc_path(center, radius, start, step * 0.6), stroke);
        }
    }

    /// Draws the edge of a rounded rectangle in dashes along its sides, with its corners whole: the card of the draft.
    pub fn dashed_border(&mut self, rect: Rect, radius: f32, (stroke, (dash, gap)): (Stroke, (f32, f32))) {
        let width = stroke.width;
        let color = stroke.color;
        let half = width * 0.5;
        let corners = [
            (Point::new(rect.x + radius, rect.y + radius), PI),
            (Point::new(rect.right() - radius, rect.y + radius), PI + FRAC_PI_2),
            (Point::new(rect.right() - radius, rect.bottom() - radius), 0.0),
            (Point::new(rect.x + radius, rect.bottom() - radius), FRAC_PI_2),
        ];
        for (center, start) in corners {
            self.stroke_path(arc_path(center, radius - half, start, FRAC_PI_2), stroke);
        }
        let mut along = rect.x + radius;
        while along < rect.right() - radius {
            let length = dash.min(rect.right() - radius - along);
            self.fill_rect(Rect::new(along, rect.y, length, width), color);
            self.fill_rect(Rect::new(along, rect.bottom() - width, length, width), color);
            along += dash + gap;
        }
        let mut down = rect.y + radius;
        while down < rect.bottom() - radius {
            let length = dash.min(rect.bottom() - radius - down);
            self.fill_rect(Rect::new(rect.x, down, width, length), color);
            self.fill_rect(Rect::new(rect.right() - width, down, width, length), color);
            down += dash + gap;
        }
    }

    /// Draws a thin bar along the top of a rectangle, with a part of it running across, for work whose length is not
    /// known, such as the next commits being read; asks for the frame of its next step.
    pub fn running_bar(&mut self, rect: Rect) {
        let theme = *self.theme();
        let thickness = metrics::progress_thickness(&theme);
        let track = Rect::new(rect.x, rect.y, rect.width, thickness.min(rect.height));
        let step_length = RUN / u32::from(RUN_STEPS);
        let steps = self.now().as_nanos() / step_length.as_nanos().max(1);
        let step = u16::try_from(steps % u128::from(RUN_STEPS)).unwrap_or(0);
        let next =
            Duration::from_nanos(u64::try_from((steps + 1).saturating_mul(step_length.as_nanos())).unwrap_or(u64::MAX));
        self.request_frame_at(next);
        let fraction = f32::from(step) / f32::from(RUN_STEPS);
        let part = track.width * RUN_PART;
        let start = track.x - part + (track.width + part) * fraction;
        let left = start.max(track.x);
        let right = (start + part).min(track.right());
        if right > left {
            self.fill_rect(
                Rect::new(left, track.y, right - left, track.height),
                theme.colors.accent,
            );
        }
    }
}

/// The two control points of the curve from one lane to another: halfway down, above the start and above the end.
fn controls(from: Point, to: Point) -> (Point, Point) {
    let middle = f32::midpoint(from.y, to.y);
    (Point::new(from.x, middle), Point::new(to.x, middle))
}

/// The point of a cubic curve at a fraction of its way.
fn cubic_at((start, first, second, end): (Point, Point, Point, Point), at: f32) -> Point {
    let rest = 1.0 - at;
    let weights = [
        rest * rest * rest,
        3.0 * rest * rest * at,
        3.0 * rest * at * at,
        at * at * at,
    ];
    let points = [start, first, second, end];
    let mut x = 0.0;
    let mut y = 0.0;
    for (weight, point) in weights.iter().zip(points) {
        x += weight * point.x;
        y += weight * point.y;
    }
    Point::new(x, y)
}

/// The point at a fraction of the way between two points.
fn lerp(start: Point, end: Point, at: f32) -> Point {
    Point::new(start.x + (end.x - start.x) * at, start.y + (end.y - start.y) * at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curve_starts_and_ends_where_it_is_asked_to() {
        let from = Point::new(10.0, 0.0);
        let to = Point::new(40.0, 26.0);
        let (first, second) = controls(from, to);
        assert_eq!(cubic_at((from, first, second, to), 0.0), from);
        assert_eq!(cubic_at((from, first, second, to), 1.0), to);
        let middle = cubic_at((from, first, second, to), 0.5);
        assert!(
            (middle.x - 25.0).abs() < 0.01 && (middle.y - 13.0).abs() < 0.01,
            "{middle:?}"
        );
    }
}
