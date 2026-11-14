//! How far to scroll one side of a diff when the other is scrolled (`diff.scroll-together`).

use std::ops::Range;

/// One side scrolled, and where the changes are on both sides, in pixels from the top of each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollInput<'a> {
    /// Each change, from its top to its bottom on the side scrolled, then on the other side, in order.
    pub changes: &'a [(Range<f32>, Range<f32>)],
    /// The height of all the lines of the side scrolled, then of the other side.
    pub heights: (f32, f32),
    /// How far the side scrolled is scrolled from its top.
    pub top: f32,
    /// The height of the view of each side.
    pub view: f32,
}

/// How far to scroll the other side, so that what is in the middle of the view of one side is in the middle of the
/// other. The anchors are the top and the bottom of each change on both sides and the ends of the two texts, made never
/// to go backwards; between two anchors the middle maps in proportion, past the last one for one. A side scrolled to
/// its end brings the other to its end.
///
/// Feature: diff.scroll-together
#[must_use]
pub fn scroll_together(input: &ScrollInput<'_>) -> f32 {
    let (height, other_height) = input.heights;
    let (end, other_end) = ((height - input.view).max(0.0), (other_height - input.view).max(0.0));
    if input.top >= end && end > 0.0 {
        return other_end;
    }
    let anchors = anchors(input);
    let middle = input.top + input.view * 0.5;
    let mapped = anchors
        .windows(2)
        .find_map(|pair| match pair {
            [(from, to), (next_from, next_to)] if middle >= *from && middle < *next_from => {
                let part = (middle - from) / (next_from - from);
                Some(to + part * (next_to - to))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            let (last_from, last_to) = anchors.last().copied().unwrap_or((0.0, 0.0));
            last_to + (middle - last_from)
        });
    (mapped - input.view * 0.5).clamp(0.0, other_end)
}

/// The anchors, as pairs of the side scrolled and the other side, never going backwards on either.
fn anchors(input: &ScrollInput<'_>) -> Vec<(f32, f32)> {
    let mut points = vec![(0.0, 0.0)];
    for (side, other) in input.changes {
        points.push((side.start, other.start));
        points.push((side.end, other.end));
    }
    points.push(input.heights);
    let mut anchors: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    for (from, to) in points {
        let (last_from, last_to) = anchors.last().copied().unwrap_or((0.0, 0.0));
        anchors.push((from.max(last_from), to.max(last_to)));
    }
    anchors
}
