//! What the keys do in a list: the arrows, Page Up, Page Down, Home and End move the row of the keyboard, Shift
//! extends the selection, Ctrl moves without selecting, Space and Ctrl+Space select, Ctrl+A selects all, and Enter
//! opens the row of the keyboard.

use super::selection::Selection;
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};

/// Whether a list lets several rows be selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionMode {
    /// One row at most.
    #[default]
    Single,
    /// Any rows, with Shift and Ctrl.
    Multiple,
    /// None: the rows are read, as the lines of a page, and are neither selected nor lit; the keys scroll them.
    Read,
}

/// What a key did to a list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyOutcome {
    /// Whether the selection or the row of the keyboard changed.
    pub(crate) changed: bool,
    /// The row Enter opened.
    pub(crate) activated: Option<usize>,
    /// Whether the row of the keyboard moved and should be brought into view.
    pub(crate) moved: bool,
    /// Whether the list used the key, so that it is not a shortcut of the view.
    pub(crate) is_used: bool,
}

/// The row the keyboard goes to from another by a key, in a list of a count with a number of rows to a page.
fn target(key: NamedKey, from: Option<usize>, count: usize, page: usize) -> Option<usize> {
    let last = count.checked_sub(1)?;
    let page = page.max(1);
    let row = match (key, from) {
        (NamedKey::ArrowUp | NamedKey::PageUp | NamedKey::ArrowDown | NamedKey::PageDown, None)
        | (NamedKey::Home, _) => 0,
        (NamedKey::End, _) => last,
        (NamedKey::ArrowUp, Some(row)) => row.saturating_sub(1),
        (NamedKey::ArrowDown, Some(row)) => row + 1,
        (NamedKey::PageUp, Some(row)) => row.saturating_sub(page),
        (NamedKey::PageDown, Some(row)) => row + page,
        _ => return None,
    };
    Some(row.min(last))
}

/// Applies a key to the selection of a list of a count, with a number of rows to a page.
pub(crate) fn apply_key(
    selection: &mut Selection,
    key: KeyEvent,
    mode: SelectionMode,
    (count, page): (usize, usize),
) -> KeyOutcome {
    let before = selection.clone();
    let mut outcome = KeyOutcome::default();
    let is_multiple = mode == SelectionMode::Multiple;
    match key.key {
        Key::Named(NamedKey::Enter) if key.modifiers.is_empty() => {
            outcome.activated = selection.cursor();
            outcome.is_used = true;
        }
        Key::Named(NamedKey::ContextMenu) => outcome.is_used = true,
        Key::Character('a') if key.modifiers == Modifiers::CONTROL && is_multiple => {
            selection.select_all(count);
            outcome.is_used = true;
        }
        Key::Character(' ') if key.modifiers.is_empty() || key.modifiers == Modifiers::CONTROL => {
            outcome.is_used = true;
            match (selection.cursor(), is_multiple && key.modifiers.command()) {
                (Some(row), true) => selection.toggle(row),
                (Some(row), false) => selection.select_only(row),
                (None, _) => {}
            }
        }
        Key::Named(named) => {
            let Some(row) = target(named, selection.cursor(), count, page) else {
                return outcome;
            };
            outcome.moved = true;
            outcome.is_used = true;
            match (is_multiple, key.modifiers) {
                (true, modifiers) if modifiers == Modifiers::SHIFT => selection.extend_to(row),
                (true, modifiers) if modifiers == Modifiers::CONTROL => selection.move_cursor(row),
                (_, modifiers) if modifiers.is_empty() || !is_multiple => selection.select_only(row),
                _ => {
                    outcome.moved = false;
                    outcome.is_used = false;
                }
            }
        }
        Key::Character(_) => {}
    }
    outcome.changed = *selection != before;
    outcome
}

#[cfg(test)]
#[allow(
    clippy::single_range_in_vec_init,
    reason = "a selection is a list of ranges of rows, and a list of one range is what one range selected is"
)]
mod tests {
    use super::*;

    fn press(key: NamedKey, modifiers: Modifiers) -> KeyEvent {
        KeyEvent::press(Key::Named(key), modifiers)
    }

    #[test]
    fn the_arrows_and_pages_move_within_the_list() {
        let mut selection = Selection::default();
        let size = (1_000_000, 20);
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(selection.cursor(), Some(0));
        let _ = apply_key(
            &mut selection,
            press(NamedKey::PageDown, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(selection.ranges(), &[20..21]);
        let _ = apply_key(
            &mut selection,
            press(NamedKey::End, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(selection.cursor(), Some(999_999));
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(selection.cursor(), Some(999_999));
        let _ = apply_key(
            &mut selection,
            press(NamedKey::PageUp, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(selection.cursor(), Some(999_979));
        let _ = apply_key(
            &mut selection,
            press(NamedKey::Home, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        let outcome = apply_key(
            &mut selection,
            press(NamedKey::ArrowUp, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!((selection.cursor(), outcome.changed), (Some(0), false));
        let outcome = apply_key(
            &mut selection,
            press(NamedKey::Enter, Modifiers::NONE),
            SelectionMode::Single,
            size,
        );
        assert_eq!(outcome.activated, Some(0));
    }

    #[test]
    fn shift_extends_ctrl_moves_alone_and_space_toggles() {
        let mut selection = Selection::single(10);
        let size = (100, 10);
        let multiple = SelectionMode::Multiple;
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::SHIFT),
            multiple,
            size,
        );
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::SHIFT),
            multiple,
            size,
        );
        assert_eq!(selection.ranges(), &[10..13]);
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::CONTROL),
            multiple,
            size,
        );
        let _ = apply_key(
            &mut selection,
            press(NamedKey::ArrowDown, Modifiers::CONTROL),
            multiple,
            size,
        );
        assert_eq!((selection.ranges(), selection.cursor()), (&[10..13][..], Some(14)));
        let space = KeyEvent::press(Key::Character(' '), Modifiers::CONTROL);
        let _ = apply_key(&mut selection, space, multiple, size);
        assert_eq!(selection.ranges(), &[10..13, 14..15]);
        let all = KeyEvent::press(Key::Character('a'), Modifiers::CONTROL);
        let _ = apply_key(&mut selection, all, multiple, size);
        assert_eq!(selection.ranges(), &[0..100]);
        let mut single = Selection::single(4);
        let _ = apply_key(
            &mut single,
            press(NamedKey::ArrowDown, Modifiers::SHIFT),
            SelectionMode::Single,
            size,
        );
        assert_eq!(single.ranges(), &[5..6]);
    }
}
