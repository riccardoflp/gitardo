//! A list steered by its view: the rows the view keeps selected, a row the view asks to bring into sight, and the keys
//! the view reads itself, such as the arrows of a history that pass over some rows.

use super::selection::Selection;
use crate::input::{Key, KeyEvent, Modifiers};
use crate::memory::ListState;

/// What a view asks of a list it steers, in each frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Steering<'text> {
    /// The rows the view keeps selected: the list starts each frame from them, so that the view's store is where the
    /// selection lives. None leaves the selection to the list.
    pub selection: Option<&'text Selection>,
    /// A row to bring into sight, done once for each number it carries.
    pub reveal: Option<Reveal>,
    /// The keys the list leaves to its view, by key and modifiers: it neither applies nor claims them, and the view
    /// reads them with [`crate::Ui::unclaimed_keys_of`].
    pub left_to_view: &'text [(Key, Modifiers)],
}

/// A row a view asks its list to bring into sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reveal {
    /// A number that changes each time the view asks again: the list brings the row into sight once for each.
    pub serial: u64,
    /// The row.
    pub row: usize,
    /// Where the row goes.
    pub place: RevealPlace,
}

/// Where a row brought into sight goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RevealPlace {
    /// Scrolled the least: to the top when it is above, to the bottom when it is below, not at all when in sight.
    Nearest,
    /// In the middle of the list.
    Center,
}

impl Steering<'_> {
    /// Whether the list leaves a key to its view.
    pub(crate) fn leaves(&self, key: &KeyEvent) -> bool {
        self.left_to_view
            .iter()
            .any(|(left, modifiers)| *left == key.key && *modifiers == key.modifiers)
    }

    /// Applies what the view asks to the state of the list before its events: the selection given, and the row to
    /// bring into sight when its number is new, for rows of a height in a view of a height.
    pub(crate) fn apply(&self, state: &mut ListState, (row_height, view): (f64, f64)) {
        if let Some(selection) = self.selection {
            state.selection = selection.clone();
        }
        let Some(reveal) = self.reveal.filter(|reveal| state.revealed != Some(reveal.serial)) else {
            return;
        };
        state.revealed = Some(reveal.serial);
        let start = crate::number::wide_of_count(reveal.row) * row_height;
        match reveal.place {
            RevealPlace::Nearest => state.scroll.reveal((start, start + row_height), view),
            RevealPlace::Center => state.scroll.jump((start + row_height * 0.5 - view * 0.5).max(0.0)),
        }
    }
}
