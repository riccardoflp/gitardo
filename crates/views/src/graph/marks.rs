//! Marks beside the scroll bar of the timeline (`graph.scroll-marks`): where the commit checked out is among those
//! loaded, where the one selected is, and where what a search found is.

use ui::{Rect, Ui};

use crate::intent::Intent;

/// The marks of the timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMarks<'a> {
    /// How many rows the marks are counted over: the rows shown, and one more while there are more to load.
    pub total: usize,
    /// The row of the commit checked out, with the color of its line.
    pub checked_out: Option<(usize, usize)>,
    /// The row selected.
    pub selected: Option<usize>,
    /// The rows a search found.
    pub found: &'a [usize],
    /// The height of a row, to tell whether the list scrolls at all.
    pub row_height: f32,
}

/// Where the marks of some rows go down a track of a height: row `i` of `n` at `(i + 0.5) / n` of it, the marks
/// closer than a pixel made one.
#[must_use]
pub fn mark_positions(rows: &[usize], total: usize, height: f32) -> Vec<f32> {
    if total == 0 {
        return Vec::new();
    }
    let mut positions: Vec<f32> = rows
        .iter()
        .map(|row| (fraction(*row) + 0.5) / fraction(total) * height)
        .collect();
    positions.sort_by(f32::total_cmp);
    let mut merged: Vec<f32> = Vec::with_capacity(positions.len());
    for position in positions {
        if merged.last().is_none_or(|last| position - last >= 1.0) {
            merged.push(position);
        }
    }
    merged
}

/// A count as a fraction of a track.
#[allow(
    clippy::cast_precision_loss,
    reason = "a mark needs no more than a pixel's precision"
)]
fn fraction(count: usize) -> f32 {
    count as f32
}

/// The marks along the rectangle its caller clipped them to, at the right edge of the list. They take no clicks, and
/// there are none while every row is in sight, where a mark would stand beside another row than its own.
///
/// Feature: graph.scroll-marks
pub fn scroll_marks_view(state: &ScrollMarks<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    if fraction(state.total) * state.row_height <= rect.height {
        return Vec::new();
    }
    let theme = *ui.theme();
    let thickness = (theme.sizes.border * 2.0).max(2.0);
    let mut draw = |rows: &[usize], color, inset: f32| {
        for y in mark_positions(rows, state.total, rect.height) {
            let mark = Rect::new(
                rect.x + inset,
                rect.y + y - thickness * 0.5,
                (rect.width - inset).max(1.0),
                thickness,
            );
            ui.fill_rect(mark, color);
        }
    };
    draw(state.found, theme.colors.find_match, 0.0);
    if let Some((row, color)) = state.checked_out {
        draw(&[row], theme.lane(color), 0.0);
    }
    if let Some(row) = state.selected {
        draw(&[row], theme.colors.accent, rect.width * 0.5);
    }
    Vec::new()
}
