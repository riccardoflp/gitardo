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
    /// At the top of the list, as far as the rows after it let it go.
    Top,
}

impl ListState {
    /// Keeps a list where it is when the height of its rows changes from one frame to the next, as when a view draws
    /// them lower: the row at its top stays at its top, and the row of the keyboard stays in sight when it was.
    pub(crate) fn follow_row_height(&mut self, row_height: f32, view: f64) {
        let Some(before) = self.row_height.replace(row_height) else {
            return;
        };
        if before <= 0.0 || (before - row_height).abs() < f32::EPSILON {
            return;
        }
        let (old, new) = (f64::from(before), f64::from(row_height));
        let offset = self.scroll.offset();
        let in_sight = self.selection.cursor().filter(|row| {
            let start = crate::number::wide_of_count(*row) * old;
            start + old > offset && start < offset + view
        });
        self.scroll.jump(offset / old * new);
        if let Some(row) = in_sight {
            let start = crate::number::wide_of_count(row) * new;
            self.scroll.reveal((start, start + new), view);
        }
    }
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
            RevealPlace::Top => state.scroll.jump(start),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests fail by panicking")]
mod tests {
    use super::*;
    use crate::widget::list::Selection;

    /// A list of rows of a height, scrolled by an offset, with the row of the keyboard on a row.
    fn list(row_height: f32, offset: f64, cursor: Option<usize>) -> ListState {
        let mut state = ListState::default();
        state.follow_row_height(row_height, 260.0);
        state.scroll.jump(offset);
        if let Some(row) = cursor {
            state.selection = Selection::single(row);
        }
        state
    }

    #[test]
    fn the_row_at_the_top_of_a_list_stays_at_its_top_when_its_rows_become_lower_or_taller() {
        let mut state = list(52.0, 520.0, None);
        state.follow_row_height(26.0, 260.0);
        assert!(
            (state.scroll.offset() - 260.0).abs() < 0.01,
            "the eleventh row is still the first in sight"
        );
        state.follow_row_height(52.0, 260.0);
        assert!((state.scroll.offset() - 520.0).abs() < 0.01);
        state.follow_row_height(52.0, 260.0);
        assert!(
            (state.scroll.offset() - 520.0).abs() < 0.01,
            "the same height moves nothing"
        );
    }

    #[test]
    fn the_row_of_the_keyboard_in_sight_stays_in_sight_when_the_rows_become_taller() {
        // Rows 10 to 19 are in sight; the row of the keyboard is the last of them.
        let mut state = list(26.0, 260.0, Some(19));
        state.follow_row_height(52.0, 260.0);
        let offset = state.scroll.offset();
        assert!(19.0 * 52.0 >= offset && 20.0 * 52.0 <= offset + 260.0, "{offset}");
        // A row of the keyboard that was out of sight does not pull the list to it.
        let mut state = list(26.0, 260.0, Some(40));
        state.follow_row_height(52.0, 260.0);
        assert!((state.scroll.offset() - 520.0).abs() < 0.01);
    }
}
