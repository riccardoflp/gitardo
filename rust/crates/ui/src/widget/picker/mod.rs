//! Pickers: an input box at the top middle of the window with a list under it of the items that match what is typed,
//! fuzzily (see [`fuzzy`]), the best first, their matched characters in bold. One item is chosen with Enter or a click;
//! or, in a picker of several, items are checked with a click or Ctrl+Space and Enter takes them all. The arrows, Page
//! Up and Page Down, and Home and End while nothing is typed, move through the list while the input box keeps the
//! keys, the row lit kept in view; a bar on the right scrolls a long list (`ui.picker-scroll`). Escape or a click
//! outside cancels. A picker whose rows the view draws itself uses [`list`].
//!
//! A picker is open while the view declares it, as a dialog is.

mod draw;
pub mod fuzzy;
pub(crate) mod list;

use std::collections::BTreeSet;
use std::hash::Hash;

use render::Rect;

use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};
use crate::layer::Layer;
use fuzzy::FuzzyMatch;

/// An item of a picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickerItem<'text> {
    /// Its words, which what is typed is matched against.
    pub label: &'text str,
    /// More about it, on the right of the row.
    pub description: &'text str,
}

/// How many items a picker chooses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PickerMode {
    /// One.
    #[default]
    Single,
    /// Any, each with a check box.
    Multiple,
}

/// A picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picker<'text> {
    /// What the empty input box says, such as **Choose a branch**.
    pub placeholder: &'text str,
    /// The items, in the order shown when nothing is typed.
    pub items: &'text [PickerItem<'text>],
    /// How many it chooses.
    pub mode: PickerMode,
    /// What its input box holds when it opens, such as what was typed in the box that opened it.
    pub query: &'text str,
    /// The items checked when it opens, by their places among the items, in a picker of several.
    pub checked: &'text [usize],
}

/// What a picker was answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerAnswer {
    /// The item chosen, by its place among the items.
    Chose(usize),
    /// The items checked, by their places among the items, in order.
    ChoseMany(Vec<usize>),
    /// The picker was closed with nothing chosen.
    Cancelled,
}

/// What a picker remembers between frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PickerState {
    /// The row lit, among the items that match.
    pub(crate) lit: usize,
    /// The first row shown, among the items that match.
    pub(crate) first_shown: usize,
    /// The items checked, by their places among the items.
    pub(crate) checked: BTreeSet<usize>,
    /// What was typed when the rows were last ranked.
    pub(crate) query: String,
}

/// The most rows a picker shows at once.
pub(crate) const SHOWN_ROWS: usize = 10;

/// A key of the picker, read from its input box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerKey {
    /// Moves the lit row by some rows, down when positive.
    Move(isize),
    /// Lights the first row.
    First,
    /// Lights the last row.
    Last,
    /// Chooses the lit row, or the rows checked.
    Choose,
    /// Checks or unchecks the lit row.
    Toggle,
    /// Closes the picker.
    Cancel,
}

/// What a key does in a picker, if anything; `Home` and `End` are the box's while something is typed.
fn picker_key(key: KeyEvent, is_typed: bool) -> Option<PickerKey> {
    let page = isize::try_from(SHOWN_ROWS).unwrap_or(1);
    match (key.key, key.modifiers) {
        (Key::Named(NamedKey::ArrowDown), Modifiers::NONE) => Some(PickerKey::Move(1)),
        (Key::Named(NamedKey::ArrowUp), Modifiers::NONE) => Some(PickerKey::Move(-1)),
        (Key::Named(NamedKey::PageDown), Modifiers::NONE) => Some(PickerKey::Move(page)),
        (Key::Named(NamedKey::PageUp), Modifiers::NONE) => Some(PickerKey::Move(-page)),
        (Key::Named(NamedKey::Home), Modifiers::NONE) if !is_typed => Some(PickerKey::First),
        (Key::Named(NamedKey::End), Modifiers::NONE) if !is_typed => Some(PickerKey::Last),
        (Key::Named(NamedKey::Enter), Modifiers::NONE) => Some(PickerKey::Choose),
        (Key::Character(' '), Modifiers::CONTROL) => Some(PickerKey::Toggle),
        (Key::Named(NamedKey::Escape), Modifiers::NONE) => Some(PickerKey::Cancel),
        _ => None,
    }
}

/// The row a move goes to, inside the rows there are.
fn moved(lit: usize, by: isize, count: usize) -> usize {
    let last = count.saturating_sub(1);
    lit.saturating_add_signed(by).min(last)
}

/// The first row shown of a list of `count` rows that shows `shown` at once, so that the row lit is shown whole: the
/// first shown before, unless the row lit is above it or below the last shown, and never past where the last row is
/// at the foot.
///
/// Feature: ui.picker-scroll
#[must_use]
pub fn first_shown(lit: usize, first: usize, shown: usize, count: usize) -> usize {
    let shown = shown.max(1);
    let first = if lit < first {
        lit
    } else if lit >= first + shown {
        lit + 1 - shown
    } else {
        first
    };
    first.min(count.saturating_sub(shown))
}

/// The items that match a query, best first, with how each matched.
pub(crate) fn ranked(items: &[PickerItem<'_>], query: &str) -> Vec<(usize, FuzzyMatch)> {
    fuzzy::rank(query, items.iter().map(|item| item.label))
}

impl Ui<'_> {
    /// Declares a picker, and returns what it was answered in the frame it was.
    pub fn picker(&mut self, key: impl Hash, picker: &Picker<'_>) -> Option<PickerAnswer> {
        let id = self.id(key);
        let input = id.with("query");
        let is_new = self.declare_overlay(id, &[input, id]);
        if is_new {
            let end = picker.query.len();
            self.memory().set_input_text(input, picker.query, (end, end));
            self.request_focus(input);
        }
        let mut state = self.memory().pickers.remove(&id).unwrap_or_default();
        if is_new {
            state.checked = picker.checked.iter().copied().collect();
        }
        let query = self.memory_view().input_text(input).unwrap_or_default().to_owned();
        let mut answer = self.picker_keys(input, picker, &mut state);
        let rect = self.picker_rect(picker, &state.query);
        let drawn = self.layer(Layer::Popup, |ui| {
            ui.within(id, |ui| ui.draw_picker(rect, picker, &mut state))
        });
        answer = answer.or(drawn);
        let now_typed = self.memory_view().input_text(input).unwrap_or_default().to_owned();
        if now_typed != query {
            state.lit = 0;
            state.first_shown = 0;
            let now = self.now();
            self.request_frame_at(now);
        }
        state.query = now_typed;
        if answer.is_none() && !is_new && self.pressed_outside(rect) {
            answer = Some(PickerAnswer::Cancelled);
        }
        let _ = self.memory().pickers.insert(id, state);
        answer
    }

    /// Where a picker goes: at the top middle of the window, as high as the rows it shows.
    fn picker_rect(&self, picker: &Picker<'_>, query: &str) -> Rect {
        let theme = self.theme();
        let window = self.window();
        let spacing = theme.sizes.spacing;
        let width = (theme.sizes.row_height * 28.0)
            .min(window.width - 2.0 * spacing.huge)
            .max(0.0);
        let count = ranked(picker.items, query).len().clamp(1, SHOWN_ROWS);
        let rows = crate::number::float_of_count(count) * theme.sizes.row_height;
        let height = crate::widget::metrics::control_height(theme) + 2.0 * spacing.small + spacing.small + rows;
        Rect::new(
            window.x + (window.width - width) * 0.5,
            window.y + spacing.huge,
            width,
            height,
        )
    }

    /// The keys of a picker, read from its input box, before it is drawn.
    fn picker_keys(&mut self, input: WidgetId, picker: &Picker<'_>, state: &mut PickerState) -> Option<PickerAnswer> {
        let rows = ranked(picker.items, &state.query);
        let mut answer = None;
        let is_typed = !state.query.is_empty();
        for key in self.unclaimed_keys_of(input) {
            let Some(action) = picker_key(key, is_typed) else {
                continue;
            };
            self.claim_key(input, key);
            let lit_item = rows.get(state.lit).map(|(item, _)| *item);
            match (action, picker.mode) {
                (PickerKey::Move(by), _) => {
                    state.lit = moved(state.lit, by, rows.len());
                    state.first_shown = first_shown(state.lit, state.first_shown, SHOWN_ROWS, rows.len());
                }
                (PickerKey::First, _) => {
                    state.lit = 0;
                    state.first_shown = 0;
                }
                (PickerKey::Last, _) => {
                    state.lit = rows.len().saturating_sub(1);
                    state.first_shown = first_shown(state.lit, state.first_shown, SHOWN_ROWS, rows.len());
                }
                (PickerKey::Choose, PickerMode::Single) => answer = lit_item.map(PickerAnswer::Chose),
                (PickerKey::Choose, PickerMode::Multiple) => {
                    answer = Some(PickerAnswer::ChoseMany(state.checked.iter().copied().collect()));
                }
                (PickerKey::Toggle, PickerMode::Multiple) => {
                    if let Some(item) = lit_item {
                        toggle(&mut state.checked, item);
                    }
                }
                (PickerKey::Toggle, PickerMode::Single) => {}
                (PickerKey::Cancel, _) => answer = Some(PickerAnswer::Cancelled),
            }
        }
        answer
    }
}

/// Checks an item, or unchecks it when it is checked.
pub(crate) fn toggle(checked: &mut BTreeSet<usize>, item: usize) {
    if !checked.remove(&item) {
        let _ = checked.insert(item);
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    #[test]
    fn the_lit_row_moves_inside_the_rows_and_stays_in_sight() {
        assert_eq!(moved(0, -1, 5), 0);
        assert_eq!(moved(3, 10, 5), 4);
        assert_eq!(moved(0, 1, 0), 0);
        let enter = KeyEvent::press(Key::Named(NamedKey::Enter), Modifiers::NONE);
        assert_eq!(picker_key(enter, false), Some(PickerKey::Choose));
        let toggle_key = KeyEvent::press(Key::Character(' '), Modifiers::CONTROL);
        assert_eq!(picker_key(toggle_key, false), Some(PickerKey::Toggle));
        assert_eq!(
            picker_key(KeyEvent::press(Key::Character(' '), Modifiers::NONE), false),
            None
        );
        let end = KeyEvent::press(Key::Named(NamedKey::End), Modifiers::NONE);
        assert_eq!(picker_key(end, false), Some(PickerKey::Last));
        assert_eq!(
            picker_key(end, true),
            None,
            "End moves the caret while something is typed"
        );
    }

    #[test]
    fn ui_picker_scroll__down_on_the_last_row_shown_scrolls_the_list_by_one_row() {
        assert_eq!(first_shown(10, 0, 10, 200), 1);
        assert_eq!(first_shown(12, 0, 10, 200), 3);
        assert_eq!(first_shown(2, 5, 10, 200), 2);
        assert_eq!(first_shown(6, 5, 10, 200), 5);
        assert_eq!(first_shown(199, 0, 10, 200), 190, "End shows the last row at the foot");
        assert_eq!(first_shown(3, 0, 10, 5), 0);
    }
}
