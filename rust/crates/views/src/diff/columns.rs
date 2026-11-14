//! The two columns of a Diff page and the gutter between them, which is dragged to make one wider
//! (`diff.resize-sides`).

use ui::{Rect, Split, Splitter, Ui};

use super::{DiffPageIntent, DiffPageState};
use crate::intent::Intent;

/// The narrowest a column is dragged.
pub const LEAST_WIDTH: f32 = 80.0;

/// Where the two columns and the gutter are, across the rows of a page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Columns {
    /// The left side.
    pub(crate) left: Rect,
    /// The gutter between them, with a band for each change.
    pub(crate) gutter: Rect,
    /// The right side, up to the scroll bar.
    pub(crate) right: Rect,
}

impl Columns {
    /// The columns of the rows of a page over a rectangle, the left side as wide as dragged, or as wide as the right.
    pub(crate) fn of(rect: Rect, left_width: Option<u32>, ui: &Ui<'_>) -> Columns {
        let theme = ui.theme();
        let gutter = theme.sizes.control_height;
        let line = theme.sizes.border;
        let room = (rect.width - theme.sizes.scrollbar).max(0.0);
        let width = left_width.map_or_else(
            || equal_width(room, ui),
            |width| {
                clamp_width(
                    crate::number::float_of_count(usize::try_from(width).unwrap_or(0)),
                    room,
                    ui,
                )
            },
        );
        let gutter_x = rect.x + width + line;
        let right_x = gutter_x + gutter;
        Columns {
            left: Rect::new(rect.x, rect.y, width, rect.height),
            gutter: Rect::new(gutter_x, rect.y, gutter, rect.height),
            right: Rect::new(right_x, rect.y, (rect.x + room - right_x).max(0.0), rect.height),
        }
    }
}

/// The width of the left side when both are as wide, in the room of the rows.
fn equal_width(room: f32, ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    ((room - theme.sizes.border - theme.sizes.control_height) * 0.5).max(0.0)
}

/// A width of the left side kept so that neither side is narrower than 80 pixels.
fn clamp_width(width: f32, room: f32, ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    let largest = room - theme.sizes.border - theme.sizes.control_height - LEAST_WIDTH;
    width.min(largest).max(LEAST_WIDTH).min(room.max(0.0))
}

/// The line between the left side and the gutter, over the rows the caller clipped it to: dragged, it gives to one side
/// what it takes from the other, never leaving either narrower than 80 pixels; a double click makes the two sides as
/// wide again. The width is kept with the page, and the next page opens with it.
///
/// Feature: diff.resize-sides
pub fn resize_sides_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let room = (rect.width - theme.sizes.scrollbar).max(0.0);
    let gutter = theme.sizes.control_height;
    let splitter = Splitter {
        split: Split::Columns,
        default_size: equal_width(room, ui),
        least_first: LEAST_WIDTH,
        least_second: LEAST_WIDTH + gutter,
        size: Some(Columns::of(rect, state.left_width, ui).left.width),
        step: 0.0,
    };
    let area = Rect::new(rect.x, rect.y, room, rect.height);
    let response = ui.splitter(("diff sides", state.id), area, &splitter);
    if response.is_reset {
        vec![Intent::Diff(super::DiffIntent::Page(
            state.id,
            DiffPageIntent::Resize(None),
        ))]
    } else if response.is_changed {
        let width = Some(u32::try_from(crate::number::count_of_float(response.size.round())).unwrap_or(u32::MAX));
        vec![Intent::Diff(super::DiffIntent::Page(
            state.id,
            DiffPageIntent::Resize(width),
        ))]
    } else {
        Vec::new()
    }
}
