//! The list of a picker whose rows the view draws itself, such as the references of a branch filter with their ticks:
//! the keys of the box over it move the row lit and keep it in view, the wheel and a bar scroll it by rows, and the
//! headings of its groups are never lit (`ui.picker-scroll`). Its rows are all as tall: one row of a list, or two for
//! rows that say something under their names.

use std::hash::Hash;

use render::Rect;

use super::first_shown;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Key, Modifiers, NamedKey};
use crate::interaction::{Sense, WidgetEvent};
use crate::widget::panel::Axis;
use crate::widget::scroll::ScrollAxis;
use crate::widget::scroll::bar::BarSpec;

/// A list of a picker: which of its rows are headings, what is typed over it, and the box whose keys move through it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickList<'a> {
    /// Whether each row, in order, is the heading of a group, which the keys step over.
    pub headings: &'a [bool],
    /// What is typed in the box over the list: when it changes, the first row that is not a heading is lit and the
    /// list goes back to its top.
    pub typed: &'a str,
    /// The input box whose keys move through the list.
    pub keys_from: WidgetId,
    /// Whether each row is as tall as two rows of a list, for a line under its name.
    pub is_tall: bool,
}

impl PickList<'_> {
    /// How tall each of its rows is.
    fn row_height(&self, ui: &Ui<'_>) -> f32 {
        let row = ui.theme().sizes.row_height;
        if self.is_tall { 2.0 * row } else { row }
    }
}

/// What a list of a picker did in a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct PickListResponse {
    /// The row lit, which is never a heading.
    pub lit: Option<usize>,
    /// The rows shown, each with where the view draws it, left of the bar when there is one.
    pub rows: Vec<(usize, Rect)>,
    /// The row lit when Enter was pressed in the box.
    pub chosen: Option<usize>,
}

/// What a list of a picker remembers between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PickListState {
    /// The row lit.
    lit: Option<usize>,
    /// How far the rows are scrolled, always by whole rows once the bar is let go.
    scroll: ScrollAxis,
    /// What was typed when the list was last drawn.
    typed: String,
}

/// Where a key moves the row lit to, among rows some of which are headings; `page` is how many rows are shown and
/// `is_typed` whether something is typed, when `Home` and `End` are the box's.
fn moved_to(key: Key, lit: Option<usize>, headings: &[bool], (page, is_typed): (usize, bool)) -> Option<usize> {
    let count = headings.len();
    let is_row = |index: &usize| headings.get(*index).is_some_and(|is_heading| !is_heading);
    let first = (0..count).find(is_row);
    let last = (0..count).rev().find(is_row);
    let Some(lit) = lit else {
        return first;
    };
    let down = |by: usize| (lit.saturating_add(by).min(count.saturating_sub(1))..count).find(is_row);
    let up = |by: usize| (0..=lit.saturating_sub(by)).rev().find(is_row);
    match key {
        Key::Named(NamedKey::ArrowDown) => down(1).or(Some(lit)),
        Key::Named(NamedKey::ArrowUp) => (0..lit).rev().find(is_row).or(Some(lit)),
        Key::Named(NamedKey::PageDown) => down(page.max(1)).or(last),
        Key::Named(NamedKey::PageUp) => up(page.max(1)).or(first),
        Key::Named(NamedKey::Home) if !is_typed => first,
        Key::Named(NamedKey::End) if !is_typed => last,
        _ => Some(lit),
    }
}

/// Whether a key moves through a list of a picker rather than the caret of its box.
fn is_list_key(key: Key, is_typed: bool) -> bool {
    match key {
        Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp | NamedKey::PageDown | NamedKey::PageUp) => true,
        Key::Named(NamedKey::Home | NamedKey::End) => !is_typed,
        _ => false,
    }
}

impl Ui<'_> {
    /// Declares the list of a picker over a rectangle and says which rows to draw where, which is lit and which was
    /// chosen with Enter; the view draws the rows, clipped to the rectangle.
    pub fn pick_list(&mut self, key: impl Hash, rect: Rect, list: &PickList<'_>) -> PickListResponse {
        let id = self.id(key);
        let row_height = list.row_height(self);
        let count = list.headings.len();
        let shown = crate::number::index_at(f64::from(rect.height), f64::from(row_height)).max(1);
        let mut state = self.memory().pick_lists.remove(&id).unwrap_or_default();
        if state.typed != list.typed || state.lit.is_none_or(|lit| lit >= count) {
            list.typed.clone_into(&mut state.typed);
            state.lit = moved_to(Key::Named(NamedKey::Home), None, list.headings, (shown, false));
            state.scroll.jump(0.0);
        }
        let chosen = self.pick_list_keys(list, &mut state, shown);
        let (_, events) = self.interact_with_events(id, rect, Sense::SCROLL);
        for event in events {
            if let WidgetEvent::Wheel { delta } = event {
                let rows = (delta.y / row_height).round();
                let rows = if rows == 0.0 { delta.y.signum() } else { rows };
                state.scroll.jump(state.scroll.target() + f64::from(rows * row_height));
            }
        }
        let content = crate::number::float_of_count(count) * row_height;
        state.scroll.clamp(f64::from(content - rect.height));
        let mut width = rect.width;
        if content > rect.height + 0.5 {
            width -= self.pick_list_bar(id, (rect, row_height), content, &mut state.scroll);
        }
        let offset = crate::number::narrow(state.scroll.offset());
        if let Some(lit) = state.lit {
            let lit_rect = Rect::new(rect.x, rect.y + row_of(lit, row_height) - offset, width, row_height);
            self.clip(rect, |ui| {
                let color = ui.theme().colors.list_inactive_selection;
                ui.fill_rounded_rect(lit_rect, ui.theme().sizes.radius.medium, color);
            });
        }
        let first = crate::number::index_at(f64::from(offset), f64::from(row_height));
        let rows = (first..count)
            .map(|index| {
                (
                    index,
                    Rect::new(rect.x, rect.y + row_of(index, row_height) - offset, width, row_height),
                )
            })
            .take_while(|(_, row)| row.y < rect.bottom())
            .collect();
        let response = PickListResponse {
            lit: state.lit,
            rows,
            chosen,
        };
        let _ = self.memory().pick_lists.insert(id, state);
        response
    }

    /// The keys of the box over a list of a picker: they move the row lit and scroll the list by whole rows so that it
    /// is shown whole. Returns the row lit when Enter was pressed.
    fn pick_list_keys(&mut self, list: &PickList<'_>, state: &mut PickListState, shown: usize) -> Option<usize> {
        let row_height = f64::from(list.row_height(self));
        let is_typed = !list.typed.is_empty();
        let mut chosen = None;
        for key in self.unclaimed_keys_of(list.keys_from) {
            if key.modifiers != Modifiers::NONE {
                continue;
            }
            if key.key == Key::Named(NamedKey::Enter) {
                self.claim_key(list.keys_from, key);
                chosen = state.lit;
                continue;
            }
            if !is_list_key(key.key, is_typed) {
                continue;
            }
            self.claim_key(list.keys_from, key);
            state.lit = moved_to(key.key, state.lit, list.headings, (shown, is_typed));
            let first = crate::number::index_at(state.scroll.target(), row_height);
            if let Some(lit) = state.lit {
                let first = first_shown(lit, first, shown, list.headings.len());
                state.scroll.jump(crate::number::wide_of_count(first) * row_height);
            }
        }
        chosen
    }

    /// Draws the bar of a list of a picker, of rows of a height, on its right and moves the scroll as it is dragged,
    /// snapped to whole rows once it is let go. Returns the width it takes.
    fn pick_list_bar(
        &mut self,
        id: WidgetId,
        (rect, row_height): (Rect, f32),
        content: f32,
        scroll: &mut ScrollAxis,
    ) -> f32 {
        let thickness = self.theme().sizes.scrollbar;
        let row_height = f64::from(row_height);
        let track = Rect::new(rect.right() - thickness, rect.y, thickness, rect.height);
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: f64::from(rect.height),
            content: f64::from(content),
        };
        self.scroll_bar(id.with("bar"), track, spec, scroll);
        let now = self.now();
        let _ = scroll.advance(now);
        let is_dragged = self.memory_view().drag.is_some_and(|drag| drag.id == id.with("bar"));
        if !is_dragged {
            let rows = (scroll.target() / row_height).round();
            scroll.jump((rows * row_height).min(spec.largest()));
        }
        thickness
    }
}

/// The top of a row, from the top of the list.
fn row_of(index: usize, row_height: f32) -> f32 {
    crate::number::float_of_count(index) * row_height
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUPS: [bool; 7] = [true, false, false, true, false, true, false];

    fn press(key: NamedKey, lit: usize) -> Option<usize> {
        moved_to(Key::Named(key), Some(lit), &GROUPS, (2, false))
    }

    #[test]
    fn the_keys_step_over_the_headings_and_stop_at_the_ends() {
        assert_eq!(press(NamedKey::ArrowDown, 2), Some(4));
        assert_eq!(press(NamedKey::ArrowUp, 4), Some(2));
        assert_eq!(press(NamedKey::ArrowUp, 1), Some(1));
        assert_eq!(press(NamedKey::ArrowDown, 6), Some(6));
        assert_eq!(press(NamedKey::End, 1), Some(6));
        assert_eq!(press(NamedKey::Home, 6), Some(1));
        assert_eq!(
            press(NamedKey::PageDown, 1),
            Some(4),
            "two rows down is a heading, and the row after it"
        );
        assert_eq!(press(NamedKey::PageUp, 4), Some(2));
        let typed = moved_to(Key::Named(NamedKey::End), Some(1), &GROUPS, (2, true));
        assert_eq!(typed, Some(1), "End is the box's while something is typed");
    }
}
