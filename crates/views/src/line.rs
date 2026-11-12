//! Controls on a line: where a button or a box of the height of the others goes in a cell of a bar, and the widths
//! of the window below which the places give up their words or their room.

use ui::Rect;
use ui::Ui;
use ui::layout::{Align, place};

/// The width of the window, in logical pixels, below which the header gives up its words for its icons and the branch
/// rail shows only icons.
pub(crate) const NARROW_BELOW: f32 = 900.0;
/// The width of the window, in logical pixels, below which the inspector opens over the timeline.
pub(crate) const INSPECTOR_OVER_BELOW: f32 = 700.0;
/// The width of the branch rail when it shows only icons.
pub(crate) const ICON_RAIL_WIDTH: f32 = 56.0;

/// A control of the height of the others, of a width, at the start of a cell and in its middle down.
pub(crate) fn control_in(ui: &Ui<'_>, cell: Rect, width: f32) -> Rect {
    let height = ui.theme().sizes.control_height;
    place(cell, (width, height), (Align::Start, Align::Center))
}

/// Whether the window is narrow: the header shows icons for words, the rail only icons.
pub(crate) fn is_narrow(ui: &Ui<'_>) -> bool {
    ui.window().width < NARROW_BELOW
}
