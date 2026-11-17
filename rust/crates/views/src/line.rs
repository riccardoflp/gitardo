//! Controls on a line: where a button or a box of the height of the others goes in a cell of a bar, and the widths
//! of the window below which the places give up their words or their room.

use ui::Rect;
use ui::Ui;
use ui::layout::{Align, place};

/// The width of the window, in logical pixels, below which the header gives up its words for its icons and the side
/// column is put away.
pub(crate) const NARROW_BELOW: f32 = 900.0;
/// The width of the window, in logical pixels, below which the inspector opens over the timeline.
pub(crate) const INSPECTOR_OVER_BELOW: f32 = 700.0;
/// The width of the branch rail below which it shows only icons: narrower than the side column can be dragged.
pub(crate) const ICON_RAIL_BELOW: f32 = 120.0;

/// A control of the height of the others, of a width, at the start of a cell and in its middle down.
pub(crate) fn control_in(ui: &Ui<'_>, cell: Rect, width: f32) -> Rect {
    let height = ui.theme().sizes.control_height;
    place(cell, (width, height), (Align::Start, Align::Center))
}

/// A count of rows as a length in rows, which an `f32` holds exactly up to 65,535; more are as many.
pub(crate) fn rows_of(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

/// Whether the branch rail, at a width, shows only its icons.
pub(crate) fn is_icon_rail(width: f32) -> bool {
    width < ICON_RAIL_BELOW
}

/// Whether the window is narrow: the header shows icons for words, the side column is put away.
pub(crate) fn is_narrow(ui: &Ui<'_>) -> bool {
    ui.window().width < NARROW_BELOW
}
