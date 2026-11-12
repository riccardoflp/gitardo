//! Quick search, in the middle of the header (`search.box`).

use ui::layout::{Align, place};
use ui::{Icon, InputBox, Ui};

use crate::intent::Intent;
use crate::text::header as words;

/// The widest Quick search is.
const SEARCH_WIDTH: f32 = 420.0;

/// What Quick search shows. It finds nothing yet: what is typed stays in its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchBox;

/// Quick search, in the middle of the rectangle its caller clipped it to, as wide as it can be up to its widest; not
/// there at all when the header has no room for it.
///
/// Feature cards that fill it: `search.box` (finding commits, branches, files and the commands of the catalog, opened
/// with `Ctrl+K`), `search.history`, `search.go-to-hash`.
pub fn box_view(_state: &SearchBox, ui: &mut Ui<'_>) -> Vec<Intent> {
    let cell = ui.current_clip();
    let width = cell.width.min(SEARCH_WIDTH);
    if width < ui.theme().sizes.icon * 4.0 {
        return Vec::new();
    }
    let height = ui.theme().sizes.control_height;
    let rect = place(cell, (width, height), (Align::Center, Align::Center));
    let input = InputBox::new("", words::QUICK_SEARCH)
        .with_icon(Icon::Search)
        .with_key_hint(words::QUICK_SEARCH_KEYS);
    let _ = ui.input_box("quick search", rect, &input);
    Vec::new()
}
