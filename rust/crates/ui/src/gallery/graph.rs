//! The graph beside the rows of the timeline: a lane for each line of history, a dot for each commit, a ring for a
//! merge, and curves where a branch leaves a lane or joins one. Each row draws its own part of it, so that a million
//! rows cost what the rows in sight cost.
//!
//! The history repeats in blocks of ten: the branch being worked on, in the second lane, has two commits on the merge
//! of a fix into main, in the first lane; the fix, in the third lane, has two commits that leave main a little lower.

use render::{LineCap, Path, Point, Rect, Stroke, Theme};

use super::data::BLOCK;
use crate::frame::Ui;

/// How far apart the lanes are, and how far the first is from the start of the row.
const LANE_STEP: f32 = 20.0;

/// The width at the start of each row that the graph takes, which the highlight of the row leaves out.
pub(super) const LEAD: f32 = 76.0;

/// A part of the lines of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Segment {
    /// A lane that goes past the row without a commit in it.
    Past(usize),
    /// The lane of the commit, from the top of the row to its dot.
    Up,
    /// The lane of the commit, from its dot to the bottom of the row.
    Down,
    /// A curve that comes from another lane in the row above into the dot.
    In(usize),
    /// A curve that leaves the dot for another lane in the row below.
    Out(usize),
}

/// The part of the graph in a row: the lane of its commit, whether the commit is a merge, and its lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    /// The lane of the commit.
    lane: usize,
    /// Whether the commit is a merge, drawn as a ring.
    is_merge: bool,
    /// The lines through the row, at most four.
    segments: [Option<Segment>; 4],
}

impl Cell {
    /// A cell of a commit in a lane with its lines.
    fn new(lane: usize, segments: &[Segment]) -> Cell {
        let mut all = [None; 4];
        for (place, segment) in all.iter_mut().zip(segments) {
            *place = Some(*segment);
        }
        Cell {
            lane,
            is_merge: false,
            segments: all,
        }
    }

    /// The same cell, of a merge.
    fn merge(self) -> Cell {
        Cell { is_merge: true, ..self }
    }
}

/// The part of the graph in a row. Main and the branch begin at the top of the history: above the first block, the
/// first lane has no line.
fn cell(row: usize) -> Cell {
    let is_top = row < BLOCK;
    let main = if is_top { None } else { Some(Segment::Past(0)) };
    let main_up = if is_top { None } else { Some(Segment::Up) };
    let with = |lane: usize, extra: Option<Segment>, segments: &[Segment]| {
        let mut all: Vec<Segment> = segments.to_vec();
        all.extend(extra);
        Cell::new(lane, &all)
    };
    match row % BLOCK {
        0 => with(1, main, &[Segment::Down]),
        1 => with(1, main, &[Segment::Up, Segment::Out(0)]),
        2 => with(0, main_up, &[Segment::In(1), Segment::Down, Segment::Out(2)]).merge(),
        3 => Cell::new(2, &[Segment::In(0), Segment::Down, Segment::Past(0)]),
        4 => Cell::new(0, &[Segment::Up, Segment::Down, Segment::Past(2)]),
        5 => Cell::new(2, &[Segment::Up, Segment::Out(0), Segment::Past(0)]),
        6 => Cell::new(0, &[Segment::Up, Segment::In(2), Segment::Down]),
        _ => Cell::new(0, &[Segment::Up, Segment::Down]),
    }
}

/// Where a lane runs across a row.
fn lane_x(rect: Rect, lane: usize) -> f32 {
    rect.x + LANE_STEP * (1.0 + crate::number::float_of_count(lane))
}

/// The half of an S-shaped curve from one lane to another between two rows: from the dot of a row down to the edge
/// between the rows, or from that edge down to the dot of the next row. Both halves meet halfway across.
fn half_curve(from: Point, to: Point) -> Path {
    let mut path = Path::new();
    let _ = path.move_to(from).cubic_to(
        Point::new(from.x, f32::midpoint(from.y, to.y)),
        Point::new((from.x * 3.0 + to.x) * 0.25, to.y),
        to,
    );
    path
}

/// The other half of the S-shaped curve: from the edge between the rows, halfway across, down to a dot.
fn second_half_curve(from: Point, to: Point) -> Path {
    let mut path = Path::new();
    let _ = path.move_to(from).cubic_to(
        Point::new((from.x + to.x * 3.0) * 0.25, from.y),
        Point::new(to.x, f32::midpoint(from.y, to.y)),
        to,
    );
    path
}

impl Ui<'_> {
    /// Draws the part of the graph in a row.
    pub(super) fn graph_cell(&mut self, row: usize, rect: Rect) {
        let theme = *self.theme();
        let cell = cell(row);
        let center = Point::new(lane_x(rect, cell.lane), rect.center().y);
        for segment in cell.segments.iter().flatten() {
            self.graph_segment(rect, (center, cell.lane), *segment, &theme);
        }
        let radius = theme.sizes.graph_dot;
        let ground = theme.colors.page_background;
        let color = theme.lane(cell.lane);
        self.fill_circle(center, radius + theme.sizes.spacing.tiny, ground);
        self.fill_circle(center, radius, color);
        if cell.is_merge {
            self.fill_circle(center, radius - theme.sizes.graph_line, ground);
        }
    }

    /// Draws a line of the graph across a row, from or to the dot at the center.
    fn graph_segment(&mut self, rect: Rect, (center, lane): (Point, usize), segment: Segment, theme: &Theme) {
        let stroke = |lane: usize| Stroke {
            width: theme.sizes.graph_line,
            cap: LineCap::Butt,
            color: theme.lane(lane),
        };
        let top = rect.y;
        let bottom = rect.bottom();
        match segment {
            Segment::Past(other) => {
                let x = lane_x(rect, other);
                self.line(Point::new(x, top), Point::new(x, bottom), stroke(other));
            }
            Segment::Up => self.line(Point::new(center.x, top), center, stroke(lane)),
            Segment::Down => self.line(center, Point::new(center.x, bottom), stroke(lane)),
            Segment::In(other) => {
                let from = Point::new(f32::midpoint(lane_x(rect, other), center.x), top);
                self.stroke_path(second_half_curve(from, center), stroke(other));
            }
            Segment::Out(other) => {
                let to = Point::new(f32::midpoint(lane_x(rect, other), center.x), bottom);
                self.stroke_path(half_curve(center, to), stroke(lane));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_that_leaves_a_row_comes_into_the_next() {
        for row in 0..3 * BLOCK {
            let leaving = ends(row, |segment, lane| match segment {
                Segment::Past(other) => Some(other),
                Segment::Down => Some(lane),
                Segment::Out(other) => Some(other + 100 * (lane + 1)),
                Segment::Up | Segment::In(_) => None,
            });
            let coming = ends(row + 1, |segment, lane| match segment {
                Segment::Past(other) => Some(other),
                Segment::Up => Some(lane),
                Segment::In(other) => Some(lane + 100 * (other + 1)),
                Segment::Down | Segment::Out(_) => None,
            });
            assert_eq!(leaving, coming, "between rows {row} and {}", row + 1);
        }
    }

    /// The lanes at one edge of a row, sorted; a curve is told apart from a straight line by the lane it comes from.
    fn ends(row: usize, end: impl Fn(Segment, usize) -> Option<usize>) -> Vec<usize> {
        let cell = cell(row);
        let mut lanes: Vec<usize> = cell
            .segments
            .iter()
            .flatten()
            .filter_map(|segment| end(*segment, cell.lane))
            .collect();
        lanes.sort_unstable();
        lanes
    }
}
