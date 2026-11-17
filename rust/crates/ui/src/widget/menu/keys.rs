//! What the keys do in a menu: the arrows up and down move through the items of the deepest level open, skipping
//! separators and what cannot be used, Page Up and Page Down by as many entries as the level shows; Right, Enter and
//! Space open a submenu; Enter and Space choose an item; Left
//! and Escape close a level; and a letter goes to the next item whose label starts with it.

use super::{MenuEntry, entries_of};
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};

/// What a key did to a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyAction<T> {
    /// It moved through the menu, or did nothing.
    Moved,
    /// It chose an item, whose value this is.
    Chose(T),
    /// It closed the last level: the menu is closed.
    Closed,
}

/// Applies a key to the levels open of a menu: the item lit in each, the last the deepest. `page` is how many entries
/// the deepest level shows.
pub(crate) fn apply_key<T: Copy>(
    levels: &mut Vec<Option<usize>>,
    root: &[MenuEntry<'_, T>],
    key: KeyEvent,
    page: usize,
) -> KeyAction<T> {
    let entries = entries_of(root, levels);
    let lit = levels.last().copied().flatten();
    let plain = key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT;
    match key.key {
        Key::Named(NamedKey::ArrowDown) => set_lit(levels, next_usable(entries, lit, Direction::Forward)),
        Key::Named(NamedKey::ArrowUp) => set_lit(levels, next_usable(entries, lit, Direction::Backward)),
        Key::Named(NamedKey::Home) => set_lit(levels, next_usable(entries, None, Direction::Forward)),
        Key::Named(NamedKey::End) => set_lit(levels, next_usable(entries, None, Direction::Backward)),
        Key::Named(NamedKey::PageDown) => set_lit(levels, paged(entries, lit, page, Direction::Forward)),
        Key::Named(NamedKey::PageUp) => set_lit(levels, paged(entries, lit, page, Direction::Backward)),
        Key::Named(NamedKey::ArrowRight) => open_lit(levels, entries, lit),
        Key::Named(NamedKey::Enter) | Key::Character(' ') => {
            if let Some(MenuEntry::Item(item)) = lit.and_then(|index| entries.get(index)) {
                return KeyAction::Chose(item.value);
            }
            open_lit(levels, entries, lit);
        }
        Key::Named(NamedKey::ArrowLeft) if levels.len() > 1 => {
            let _ = levels.pop();
        }
        Key::Named(NamedKey::Escape) => {
            let _ = levels.pop();
            if levels.is_empty() {
                return KeyAction::Closed;
            }
        }
        Key::Character(letter) if plain => set_lit(levels, next_starting_with(entries, lit, letter)),
        _ => {}
    }
    KeyAction::Moved
}

/// Which way to go through the items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    /// Down.
    Forward,
    /// Up.
    Backward,
}

/// Lights an item of the deepest level, when one was found.
fn set_lit(levels: &mut [Option<usize>], item: Option<usize>) {
    if let (Some(last), Some(item)) = (levels.last_mut(), item) {
        *last = Some(item);
    }
}

/// Opens the submenu of the item lit, with its first usable item lit.
fn open_lit<T>(levels: &mut Vec<Option<usize>>, entries: &[MenuEntry<'_, T>], lit: Option<usize>) {
    if let Some(MenuEntry::Submenu { entries: inner, .. }) = lit.and_then(|index| entries.get(index)) {
        levels.push(next_usable(inner, None, Direction::Forward));
    }
}

/// The next usable entry from one, round from the end to the start, or the first or last when none is lit.
fn next_usable<T>(entries: &[MenuEntry<'_, T>], from: Option<usize>, direction: Direction) -> Option<usize> {
    let count = entries.len();
    (1..=count)
        .map(|step| match (from, direction) {
            (None, Direction::Forward) => step - 1,
            (None, Direction::Backward) => count - step,
            (Some(at), Direction::Forward) => (at + step) % count,
            (Some(at), Direction::Backward) => (at + count * 2 - step) % count,
        })
        .find(|index| entries.get(*index).is_some_and(MenuEntry::is_usable))
}

/// The usable entry a page away from the one lit, without going round: the nearest usable one at that place or before
/// it, or the last or first usable one.
fn paged<T>(entries: &[MenuEntry<'_, T>], from: Option<usize>, page: usize, direction: Direction) -> Option<usize> {
    let last = entries.len().checked_sub(1)?;
    let at = match (from, direction) {
        (None, _) => return next_usable(entries, None, direction),
        (Some(at), Direction::Forward) => at.saturating_add(page.max(1)).min(last),
        (Some(at), Direction::Backward) => at.saturating_sub(page.max(1)),
    };
    let is_usable = |index: &usize| entries.get(*index).is_some_and(MenuEntry::is_usable);
    let back_toward_lit = match direction {
        Direction::Forward => (0..=at).rev().find(is_usable),
        Direction::Backward => (at..=last).find(is_usable),
    };
    back_toward_lit.or(from)
}

/// The next usable entry after one whose label starts with a letter, in any case, round from the end to the start.
fn next_starting_with<T>(entries: &[MenuEntry<'_, T>], from: Option<usize>, letter: char) -> Option<usize> {
    let count = entries.len();
    let start = from.map_or(0, |at| at + 1);
    (0..count).map(|step| (start + step) % count).find(|index| {
        entries.get(*index).is_some_and(|entry| {
            entry.is_usable()
                && entry
                    .label()
                    .chars()
                    .next()
                    .is_some_and(|first| first.to_lowercase().eq(letter.to_lowercase()))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::menu::MenuItem;

    fn press(key: Key) -> KeyEvent {
        KeyEvent::press(key, Modifiers::NONE)
    }

    const INNER: [MenuEntry<'static, u8>; 2] = [
        MenuEntry::Item(MenuItem::new("Soft", 10)),
        MenuEntry::Item(MenuItem::new("Hard", 11)),
    ];

    fn root() -> [MenuEntry<'static, u8>; 5] {
        [
            MenuEntry::Item(MenuItem::new("Copy", 1)),
            MenuEntry::Item(MenuItem::new("Cut", 2).disabled()),
            MenuEntry::Separator,
            MenuEntry::Submenu {
                label: "Reset",
                icon: None,
                entries: &INNER,
            },
            MenuEntry::Item(MenuItem::new("Create", 3)),
        ]
    }

    #[test]
    fn the_arrows_skip_separators_and_disabled_items_and_go_round() {
        let root = root();
        let mut levels = vec![None];
        let down = press(Key::Named(NamedKey::ArrowDown));
        let _ = apply_key(&mut levels, &root, down, 3);
        assert_eq!(levels, vec![Some(0)]);
        let _ = apply_key(&mut levels, &root, down, 3);
        assert_eq!(levels, vec![Some(3)]);
        let _ = apply_key(&mut levels, &root, down, 3);
        let _ = apply_key(&mut levels, &root, down, 3);
        assert_eq!(levels, vec![Some(0)]);
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::ArrowUp)), 3);
        assert_eq!(levels, vec![Some(4)]);
        let _ = apply_key(&mut levels, &root, press(Key::Character('r')), 3);
        assert_eq!(levels, vec![Some(3)]);
        let _ = apply_key(&mut levels, &root, press(Key::Character('c')), 3);
        assert_eq!(
            levels,
            vec![Some(4)],
            "the next item starting with c after Reset is Create"
        );
    }

    #[test]
    fn right_opens_a_submenu_left_and_escape_close_one_level_and_enter_chooses() {
        let root = root();
        let mut levels = vec![Some(3)];
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::ArrowRight)), 3);
        assert_eq!(levels, vec![Some(3), Some(0)]);
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::ArrowLeft)), 3);
        assert_eq!(levels, vec![Some(3)]);
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::Enter)), 3);
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::ArrowDown)), 3);
        let chosen = apply_key(&mut levels, &root, press(Key::Named(NamedKey::Enter)), 3);
        assert_eq!(chosen, KeyAction::Chose(11));
        let escape = press(Key::Named(NamedKey::Escape));
        assert_eq!(apply_key(&mut levels, &root, escape, 3), KeyAction::Moved);
        assert_eq!(apply_key(&mut levels, &root, escape, 3), KeyAction::Closed);
    }

    #[test]
    fn page_down_and_page_up_move_by_the_entries_shown_without_going_round() {
        let root = root();
        let mut levels = vec![Some(0)];
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::PageDown)), 3);
        assert_eq!(levels, vec![Some(3)], "three entries down");
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::PageDown)), 3);
        assert_eq!(levels, vec![Some(4)], "the last, not round to the first");
        let _ = apply_key(&mut levels, &root, press(Key::Named(NamedKey::PageUp)), 10);
        assert_eq!(levels, vec![Some(0)]);
    }
}
