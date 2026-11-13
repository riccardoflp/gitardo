//! The editor of a File page (`file-view.edit`): what the keys and the pointer ask of the text, and the context menu
//! of the editor, with cut, copy, paste, **Find** and **Go to line**.

use ui::editor::Command;
use ui::{CodeEditor, MenuEntry, MenuItem, Ui};

use super::{FilePageIntent, FilePageState, FileShown, page_intent};
use crate::intent::Intent;
use crate::text::file_view as words;

/// An entry of the context menu of the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MenuChoice {
    /// Cut.
    Cut,
    /// Copy.
    Copy,
    /// Paste, which comes with `Ctrl+V` only for now.
    Paste,
    /// Find.
    Find,
    /// Go to line.
    GoToLine,
}

/// The editor of a page over the rectangle its caller clipped it to, and its context menu; gives what the keys, the
/// pointer and the menu asked of the text.
///
/// Feature: file-view.edit
pub fn edit_view(state: &FilePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let FileShown::Text(text) = state.shown else {
        return Vec::new();
    };
    let rect = ui.current_clip();
    let (marks, current_mark) = state.find.map_or((&[][..], None), |find| (find.marks, find.current));
    let editor = CodeEditor {
        name: state.title,
        buffer: text.buffer,
        cursor: text.cursor,
        colors: text.colors,
        marks,
        current_mark,
        is_read_only: state.is_read_only,
        reveal: text.reveal,
        focus: text.focus,
        tab_width: state.tab_width,
    };
    let answer = ui.code_editor(("file editor", state.id), rect, &editor);
    let mut commands = answer.commands;
    let mut intents = Vec::new();
    let has_selection = text.cursor.has_selection();
    let entry = |label, keys, choice, is_on: bool| {
        let item = MenuItem::new(label, choice).with_shortcut(keys);
        MenuEntry::Item(if is_on { item } else { item.disabled() })
    };
    let entries = [
        entry(
            words::CUT,
            words::CUT_KEYS,
            MenuChoice::Cut,
            has_selection && !state.is_read_only,
        ),
        entry(words::COPY, words::COPY_KEYS, MenuChoice::Copy, has_selection),
        entry(words::PASTE, words::PASTE_KEYS, MenuChoice::Paste, false),
        MenuEntry::Separator,
        entry(words::FIND, words::FIND_KEYS, MenuChoice::Find, true),
        entry(words::GO_TO_LINE, words::GO_TO_LINE_KEYS, MenuChoice::GoToLine, true),
    ];
    let chosen = ui.context_menu(("file editor menu", state.id), &answer.response, &entries);
    match chosen {
        Some(MenuChoice::Cut | MenuChoice::Copy) => {
            let copied = text.buffer.slice_as_file(text.cursor.range());
            ui.copy_to_clipboard(copied);
            if chosen == Some(MenuChoice::Cut) {
                commands.push(Command::DeleteSelection);
            }
        }
        Some(MenuChoice::Find) => intents.push(page_intent(state, FilePageIntent::OpenFind)),
        Some(MenuChoice::GoToLine) => intents.push(page_intent(state, FilePageIntent::OpenGoToLine)),
        Some(MenuChoice::Paste) | None => {}
    }
    if !commands.is_empty() {
        intents.insert(0, page_intent(state, FilePageIntent::Edit(commands)));
    }
    intents
}
