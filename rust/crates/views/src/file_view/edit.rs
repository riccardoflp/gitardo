//! The editor of a File page (`file-view.edit`): what the keys and the pointer ask of the text, and the context menu
//! of the editor, with cut, copy, paste, **Find** and **Go to line**, the history of the file and of its lines, and the
//! operations of the blame; with the blame beside its lines and at the end of the line of the cursor.

use ui::editor::Command;
use ui::{CodeEditor, MenuEntry, MenuItem, Ui};

use super::{FilePageIntent, FilePageState, FileShown, page_intent};
use crate::blame::FileBlameView;
use crate::intent::Intent;
use crate::text::blame as blame_words;
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
    /// An operation of the blame or of the history, on the line of the cursor.
    Run(&'static str),
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
        gutter: state.blame.gutter_columns(),
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
        wrap: state.wrap,
    };
    let answer = ui.code_editor(("file editor", state.id), rect, &editor);
    let mut commands = answer.commands;
    let caret_line = text.buffer.position(text.cursor.head).line;
    let blame = FileBlameView {
        page: state.id,
        blame: state.blame,
        places: answer.places,
        rows: &answer.rows,
        visible: answer.visible_lines.clone(),
        caret_line,
        line_count: text.buffer.line_count(),
        caret_text: text.buffer.line(caret_line),
        tab_width: state.tab_width,
    };
    let mut intents = crate::blame::file_view(&blame, ui);
    intents.extend(crate::blame::current_line_view(&blame, ui));
    let entries = menu_entries(state, text.cursor.has_selection());
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
        Some(MenuChoice::Run(feature)) => intents.push(super::blame_intent(state, feature)),
        Some(MenuChoice::Paste) | None => {}
    }
    if !commands.is_empty() {
        intents.insert(0, page_intent(state, FilePageIntent::Edit(commands)));
    }
    intents
}

/// The entries of the context menu of the editor: cut, copy and paste, **Find** and **Go to line**, then the history
/// of the file and of the lines selected, and, in a file of a repository, the operations of the blame.
fn menu_entries(state: &FilePageState<'_>, has_selection: bool) -> Vec<MenuEntry<'static, MenuChoice>> {
    let entry = |label, keys, choice, is_on: bool| {
        let item = MenuItem::new(label, choice).with_shortcut(keys);
        MenuEntry::Item(if is_on { item } else { item.disabled() })
    };
    let run = |label, feature| MenuEntry::Item(MenuItem::new(label, MenuChoice::Run(feature)));
    let mut entries = vec![
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
        MenuEntry::Separator,
        run(blame_words::SHOW_FILE_HISTORY, "file-history.file"),
    ];
    if !state.is_read_only {
        entries.push(run(blame_words::SHOW_LINES_HISTORY, "file-history.lines"));
    }
    if state.blame.can_blame {
        entries.extend([
            MenuEntry::Separator,
            run(blame_words::TOGGLE_FILE_BLAME, "blame.toggle-file"),
            entry(
                blame_words::OPEN_LINE_COMMIT,
                blame_words::OPEN_LINE_COMMIT_KEYS,
                MenuChoice::Run("blame.open-line-commit"),
                true,
            ),
            entry(
                blame_words::REVEAL_LINE_COMMIT,
                blame_words::REVEAL_LINE_COMMIT_KEYS,
                MenuChoice::Run("blame.reveal-line-commit"),
                true,
            ),
            run(blame_words::BLAME_BEFORE, "blame.previous"),
        ]);
    }
    entries
}
