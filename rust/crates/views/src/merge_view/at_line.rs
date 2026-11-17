//! The line of a Merge page the cursor is on, and the file opened in the File page at the line of the result it stands
//! for (`merge-view.open-at-line`): a click on a line puts the cursor on it, `F4` and **Jump to source** in the context
//! menu of a line open the file.

use model::merge_view::Column;
use ui::{Key, MenuEntry, Modifiers, NamedKey, Rect, Sense, Ui};

use super::{MergeIntent, MergePageIntent, MergePageState};
use crate::icons;
use crate::intent::Intent;
use crate::text::merge_view as words;

/// `F4`, which no widget of the page used: the File page at the line of the result the cursor stands for; with no
/// cursor, at the first line of the region chosen, or at the first line.
///
/// Feature: merge-view.open-at-line
pub fn open_at_line_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let is_asked = ui
        .unclaimed_keys()
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Function(4)) && key.modifiers == Modifiers::NONE);
    if !is_asked {
        return Vec::new();
    }
    let at = state.cursor.unwrap_or_else(|| {
        let region = state
            .current
            .and_then(|current| state.text.and_then(|text| text.regions.get(current)));
        (Column::Result, region.map_or(0, |region| region.result.start))
    });
    vec![asked(state, MergePageIntent::OpenAtLine(at.0, at.1))]
}

/// A line of a column, where it is drawn: a click puts the cursor on it, and its context menu has **Jump to source**;
/// the line the cursor is on is marked.
pub(crate) fn line_cell(
    ui: &mut Ui<'_>,
    state: &MergePageState<'_>,
    (rect, column, line): (Rect, Column, usize),
) -> Vec<Intent> {
    if state.cursor == Some((column, line)) {
        let color = ui.theme().colors.text_selection;
        ui.fill_rect(rect, color);
    }
    let response = ui.interact(ui.id(("merge line", state.id, column, line)), rect, Sense::CLICK);
    let mut intents = Vec::new();
    if response.clicked || response.secondary_clicked {
        intents.push(asked(state, MergePageIntent::Cursor(column, line)));
    }
    let entries = [MenuEntry::Item(icons::action(
        icons::OPEN_FILE,
        words::JUMP_TO_SOURCE,
        true,
    ))];
    if ui.context_menu(("merge line menu", state.id, column, line), &response, &entries) == Some(true) {
        intents.push(asked(state, MergePageIntent::OpenAtLine(column, line)));
    }
    intents
}

/// What a page is asked.
fn asked(state: &MergePageState<'_>, intent: MergePageIntent) -> Intent {
    Intent::MergeView(MergeIntent::Page(state.id, intent))
}
