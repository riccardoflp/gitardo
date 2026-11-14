//! Lists whose rows are read rather than chosen, as the lines of a page: no row is selected or lit, and the keys scroll
//! the rows.

use render::{Rect, Theme};

use super::{List, Row, RowShape, Selection, SelectionMode};
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::KeyEvent;
use crate::interaction::Response;
use crate::memory::ListState;
use crate::number;
use crate::widget::scroll::{KeyScroll, key_scroll};

impl Ui<'_> {
    /// A row being drawn: its background for the pointer, the selection and the keyboard, but in a list that is read.
    pub(crate) fn row_of(
        &mut self,
        mode: SelectionMode,
        shape: RowShape,
        rows: (usize, Option<usize>),
        (selection, list): (&Selection, &Response),
    ) -> Row {
        if mode == SelectionMode::Read {
            read_row(self.theme(), shape.rect, rows)
        } else {
            self.row_background(shape, rows, selection, list)
        }
    }

    /// Scrolls a list that is read by the arrows, Page Up, Page Down, Home and End.
    pub(crate) fn scroll_read_list(
        &mut self,
        (rect, list): (Rect, &List<'_>),
        state: &mut ListState,
        (key, id): (KeyEvent, WidgetId),
    ) {
        let line = f64::from(list.row_height);
        let page = (f64::from(rect.height) - line).max(line);
        let content = number::wide_of_count(list.row_count) * line;
        let scroll = key_scroll(key.key, key.modifiers, (line, page));
        if scroll.is_some() {
            self.claim_key(id, key);
        }
        let now = self.now();
        match scroll {
            Some(KeyScroll::By(distance)) => state.scroll.scroll_by(distance, now),
            Some(KeyScroll::Start) => state.scroll.glide_to(0.0, now),
            Some(KeyScroll::End) => state.scroll.glide_to(content, now),
            None => {}
        }
    }
}

/// A row of a list whose rows are read: nothing behind it, the text in the color of the interface.
fn read_row(theme: &Theme, rect: Rect, (index, hovered): (usize, Option<usize>)) -> Row {
    Row {
        index,
        rect,
        is_selected: false,
        is_hovered: hovered == Some(index),
        foreground: theme.colors.foreground,
    }
}
