//! Lists of rows of one height, as long as a million rows or more: only the rows in sight are laid out and drawn. The
//! list draws the background of each row, for the pointer, the selection and the keyboard, and the view draws what is
//! in the row.

mod keys;
mod selection;

use std::hash::Hash;
use std::ops::Range;

pub use keys::SelectionMode;
pub use selection::Selection;

use render::{Border, Color, Rect};

use super::scroll::GLIDE_FRAME;
use super::scroll::bar::BarSpec;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Modifiers, PointerButton};
use crate::interaction::{Response, Sense, WidgetEvent};
use crate::memory::ListState;
use crate::number;
use crate::widget::find::{ListSearch, RowMatches, RowOwner, RowSearch, SearchedList};
use crate::widget::panel::Axis;

/// A list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct List {
    /// How many rows it has.
    pub row_count: usize,
    /// The height of each row, in logical pixels.
    pub row_height: f32,
    /// Whether several rows can be selected.
    pub selection_mode: SelectionMode,
}

/// A row being drawn, for the view that draws what is in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    /// Which row, from zero.
    pub index: usize,
    /// Where it is.
    pub rect: Rect,
    /// Whether it is selected.
    pub is_selected: bool,
    /// Whether the pointer is over it.
    pub is_hovered: bool,
    /// The color of its text: that of the selection when it is selected in a list with the focus.
    pub foreground: Color,
}

/// What a list tells its view.
#[derive(Clone, Debug, PartialEq)]
pub struct ListResponse {
    /// What happened to the list as a whole.
    pub response: Response,
    /// The selection, when it changed in this frame.
    pub changed_selection: Option<Selection>,
    /// The row opened by a double click or Enter.
    pub activated: Option<usize>,
    /// The row the menu of the context was asked for on, by its button or its key.
    pub context_row: Option<usize>,
    /// The rows in sight.
    pub visible_rows: Range<usize>,
}

/// The rows of a list in sight, and where the first of them starts.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Window {
    /// The rows in sight.
    pub(crate) rows: Range<usize>,
    /// Where the first row starts, in logical pixels from the top of the list, at most zero.
    pub(crate) first_top: f32,
}

/// The rows of a list in sight for an offset, and where the first starts.
pub(crate) fn visible_rows(list: &List, offset: f64, view: f32) -> Window {
    let height = f64::from(list.row_height);
    let first = number::index_at(offset, height).min(list.row_count);
    let shown = number::index_at(f64::from(view), height) + 2;
    let end = first.saturating_add(shown).min(list.row_count);
    let first_top = number::narrow(number::wide_of_count(first) * height - offset);
    Window {
        rows: first..end,
        first_top,
    }
}

/// The row at a height in a list, from its top, for an offset; nothing past the last row.
pub(crate) fn row_at(list: &List, offset: f64, y: f32) -> Option<usize> {
    let row = number::index_at(offset + f64::from(y), f64::from(list.row_height));
    (y >= 0.0 && row < list.row_count).then_some(row)
}

impl Ui<'_> {
    /// Declares a list over a rectangle, and calls `draw_row` for each row in sight.
    pub fn list(
        &mut self,
        key: impl Hash,
        rect: Rect,
        list: &List,
        draw_row: impl FnMut(&mut Ui<'_>, &Row),
    ) -> ListResponse {
        let id = self.id(key);
        self.list_of(id, (rect, list), None, draw_row)
    }

    /// Declares a list whose rows find in page searches all of, in sight or not, by the text `search` gives of each.
    pub fn searchable_list(
        &mut self,
        key: impl Hash,
        (rect, list): (Rect, &List),
        search: &ListSearch<'_>,
        draw_row: impl FnMut(&mut Ui<'_>, &Row),
    ) -> ListResponse {
        let id = self.id(key);
        self.list_of(id, (rect, list), Some(search), draw_row)
    }

    /// Declares a list, searched by find in page when a search is given.
    fn list_of(
        &mut self,
        id: WidgetId,
        (rect, list): (Rect, &List),
        search: Option<&ListSearch<'_>>,
        mut draw_row: impl FnMut(&mut Ui<'_>, &Row),
    ) -> ListResponse {
        let (response, events) = self.interact_with_events(id, rect, Sense::CLICK | Sense::FOCUS | Sense::SCROLL);
        let mut state = self.memory().lists.remove(&id).unwrap_or_default();
        state.selection.limit(list.row_count);
        let before = state.selection.clone();
        let mut outcome = ListResponse {
            response,
            changed_selection: None,
            activated: None,
            context_row: None,
            visible_rows: 0..0,
        };
        for event in events {
            self.list_event(rect, list, &mut state, (event, &mut outcome));
        }
        let content = number::wide_of_count(list.row_count) * f64::from(list.row_height);
        state.scroll.clamp(content - f64::from(rect.height));
        if state.scroll.advance(self.now()) {
            let next = self.now() + GLIDE_FRAME;
            self.request_frame_at(next);
        }
        let window = visible_rows(list, state.scroll.offset(), rect.height);
        let hovered_row = self
            .pointer()
            .filter(|_| outcome.response.hovered)
            .and_then(|point| row_at(list, state.scroll.offset(), point.y - rect.y));
        let is_searched = search.is_some_and(|search| self.search_rows(id, (rect, list), search));
        self.clip(rect, |ui| {
            let mut top = rect.y + window.first_top;
            for index in window.rows.clone() {
                let row_rect = Rect::new(rect.x, top, rect.width, list.row_height);
                let row = ui.row_background(row_rect, (index, hovered_row), &state.selection, &outcome.response);
                if is_searched {
                    ui.recording.set_owner(Some(RowOwner { list: id, row: index }));
                }
                draw_row(ui, &row);
                top += list.row_height;
            }
            ui.recording.set_owner(None);
        });
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: f64::from(rect.height),
            content,
        };
        let thickness = self.theme().sizes.scrollbar;
        let track = Rect::new(rect.right() - thickness, rect.y, thickness, rect.height);
        self.scroll_bar(id.with("bar"), track, spec, &mut state.scroll);
        if state.selection != before {
            outcome.changed_selection = Some(state.selection.clone());
        }
        outcome.visible_rows = window.rows;
        let _ = self.memory().lists.insert(id, state);
        outcome
    }

    /// Applies one event to a list.
    fn list_event(
        &mut self,
        rect: Rect,
        list: &List,
        state: &mut ListState,
        (event, outcome): (WidgetEvent, &mut ListResponse),
    ) {
        let page = number::index_at(f64::from(rect.height), f64::from(list.row_height)).max(1);
        let height = f64::from(list.row_height);
        match event {
            WidgetEvent::Wheel { delta } => state.scroll.scroll_by(f64::from(delta.y), self.now()),
            WidgetEvent::Key(key) => {
                let result = keys::apply_key(&mut state.selection, key, list.selection_mode, (list.row_count, page));
                if result.is_used {
                    self.claim_key(outcome.response.id, key);
                }
                outcome.activated = outcome.activated.or(result.activated);
                if key.key == crate::input::Key::Named(crate::input::NamedKey::ContextMenu) {
                    outcome.context_row = state.selection.cursor();
                }
                if let (true, Some(row)) = (result.moved, state.selection.cursor()) {
                    let start = number::wide_of_count(row) * height;
                    state.scroll.reveal((start, start + height), f64::from(rect.height));
                }
            }
            WidgetEvent::Pressed {
                button,
                position,
                modifiers,
                click_count,
            } => {
                let Some(row) = row_at(list, state.scroll.offset(), position.y - rect.y) else {
                    return;
                };
                press_row(state, list.selection_mode, (row, button, modifiers));
                if button == PointerButton::Primary && click_count == 2 {
                    outcome.activated = Some(row);
                }
                if button == PointerButton::Secondary {
                    outcome.context_row = Some(row);
                }
                let start = number::wide_of_count(row) * height;
                state.scroll.reveal((start, start + height), f64::from(rect.height));
            }
            _ => {}
        }
    }

    /// Draws the background of a row, for the pointer, the selection and the keyboard, and says how the row is.
    fn row_background(
        &mut self,
        rect: Rect,
        (index, hovered): (usize, Option<usize>),
        selection: &Selection,
        list: &Response,
    ) -> Row {
        let theme = *self.theme();
        let colors = theme.colors;
        let is_selected = selection.contains(index);
        let is_hovered = hovered == Some(index);
        let is_active = list.has_focus && self.is_window_focused();
        let background = match (is_selected, is_active, is_hovered) {
            (true, true, _) => colors.list_selection,
            (true, false, _) => colors.list_inactive_selection,
            (false, _, true) => colors.list_hover,
            (false, _, false) => Color::TRANSPARENT,
        };
        self.fill_rect(rect, background);
        if list.shows_focus && selection.cursor() == Some(index) {
            let border = Border {
                width: theme.sizes.focus_ring,
                radius: 0.0,
                color: colors.focus_ring,
            };
            self.border(rect, border);
        }
        Row {
            index,
            rect,
            is_selected,
            is_hovered,
            foreground: if is_selected && is_active {
                colors.list_selection_foreground
            } else {
                colors.foreground
            },
        }
    }
}

impl Ui<'_> {
    /// Searches every row of a list for find in page, once per query, and says whether it did.
    fn search_rows(&mut self, id: WidgetId, (rect, list): (Rect, &List), search: &ListSearch<'_>) -> bool {
        if !self.recording.is_on() {
            return false;
        }
        let find = &self.memory_view().find;
        let (query, case) = (find.query.clone(), find.case);
        let wanted = RowSearch {
            query: &query,
            case,
            generation: search.generation,
            row_count: list.row_count,
        };
        let is_known = self
            .memory_view()
            .list_searches
            .get(&id)
            .is_some_and(|known| known.is_for(&wanted));
        if !is_known {
            let matches = RowMatches::search(&wanted, search.row_text);
            let _ = self.memory().list_searches.insert(id, matches);
        }
        self.recording.lists.push(SearchedList {
            list: id,
            rect,
            row_height: list.row_height,
        });
        true
    }
}

/// A press on a row: a click selects it, Shift extends to it, Ctrl toggles it, and the button of context menus selects
/// it unless it is already selected.
fn press_row(state: &mut ListState, mode: SelectionMode, (row, button, modifiers): (usize, PointerButton, Modifiers)) {
    let selection = &mut state.selection;
    let is_multiple = mode == SelectionMode::Multiple;
    match button {
        PointerButton::Primary if is_multiple && modifiers == Modifiers::SHIFT => selection.extend_to(row),
        PointerButton::Primary if is_multiple && modifiers == Modifiers::CONTROL => selection.toggle(row),
        PointerButton::Primary => selection.select_only(row),
        PointerButton::Secondary if !selection.contains(row) => selection.select_only(row),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_rows_in_sight_are_laid_out_even_in_a_million() {
        let list = List {
            row_count: 1_000_000,
            row_height: 22.0,
            selection_mode: SelectionMode::Single,
        };
        let window = visible_rows(&list, 11_000_011.0, 440.0);
        assert_eq!(window.rows, 500_000..500_022);
        assert_eq!(window.first_top, -11.0);
        let end = visible_rows(&list, 21_999_560.0, 440.0);
        assert_eq!(end.rows, 999_980..1_000_000);
        assert_eq!(row_at(&list, 11_000_011.0, 12.0), Some(500_001));
        assert_eq!(row_at(&list, 21_999_560.0, 445.0), None);
        assert_eq!(row_at(&list, 0.0, -1.0), None);
    }
}
