//! The editor of a File page (`file-view.edit`): what the keys and the pointer ask of the text, and the context menu
//! of the editor, with cut, copy, paste, **Find** and **Go to line**, the history of the file and of its lines, and the
//! operations of the blame; with the blame beside its lines and at the end of the line of the cursor.

use ui::editor::Command;
use ui::{CodeEditor, MenuEntry, MenuItem, Ui};

use super::{FilePageIntent, FilePageState, FileShown, page_intent};
use crate::blame::FileBlameView;
use crate::icons;
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
        lit_line: None,
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

/// An entry of the context menu of the editor with its keys, disabled when it cannot be used.
fn keyed(item: MenuItem<'static, MenuChoice>, keys: &'static str, is_on: bool) -> MenuEntry<'static, MenuChoice> {
    let item = item.with_shortcut(keys);
    MenuEntry::Item(if is_on { item } else { item.disabled() })
}

/// An entry of the context menu of the editor that runs an operation of the catalog on the file.
fn run(label: &'static str, feature: &'static str) -> MenuItem<'static, MenuChoice> {
    icons::operation_item(label, feature, MenuChoice::Run(feature))
}

/// The entries of the context menu of the editor: cut, copy and paste, **Find** and **Go to line**, then the history
/// of the file and of the lines selected, and, in a file of a repository, the operations of the blame.
fn menu_entries(state: &FilePageState<'_>, has_selection: bool) -> Vec<MenuEntry<'static, MenuChoice>> {
    let can_cut = has_selection && !state.is_read_only;
    let mut entries = vec![
        keyed(
            icons::action(icons::CUT, words::CUT, MenuChoice::Cut),
            words::CUT_KEYS,
            can_cut,
        ),
        keyed(
            icons::action(icons::COPY, words::COPY, MenuChoice::Copy),
            words::COPY_KEYS,
            has_selection,
        ),
        keyed(
            icons::action(icons::PASTE, words::PASTE, MenuChoice::Paste),
            words::PASTE_KEYS,
            false,
        ),
        MenuEntry::Separator,
        keyed(
            icons::action(icons::SEARCH, words::FIND, MenuChoice::Find),
            words::FIND_KEYS,
            true,
        ),
        keyed(
            icons::without_icon(words::GO_TO_LINE, MenuChoice::GoToLine),
            words::GO_TO_LINE_KEYS,
            true,
        ),
        MenuEntry::Separator,
        MenuEntry::Item(run(blame_words::SHOW_FILE_HISTORY, "file-history.file")),
    ];
    if !state.is_read_only {
        entries.push(MenuEntry::Item(run(
            blame_words::SHOW_LINES_HISTORY,
            "file-history.lines",
        )));
    }
    if state.blame.can_blame {
        entries.extend([
            MenuEntry::Separator,
            MenuEntry::Item(run(blame_words::TOGGLE_FILE_BLAME, "blame.toggle-file")),
            keyed(
                run(blame_words::OPEN_LINE_COMMIT, "blame.open-line-commit"),
                blame_words::OPEN_LINE_COMMIT_KEYS,
                true,
            ),
            keyed(
                run(blame_words::REVEAL_LINE_COMMIT, "blame.reveal-line-commit"),
                blame_words::REVEAL_LINE_COMMIT_KEYS,
                true,
            ),
            MenuEntry::Item(run(blame_words::BLAME_BEFORE, "blame.previous")),
        ]);
    }
    entries
}
