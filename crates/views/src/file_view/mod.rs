//! The File page (`file-view.page`): a file of the working tree, which can be edited, or a file as it is at a revision,
//! read only, in Gitardo's own editor (`file-view.edit`), with the colors of its code (`file-view.highlight`), a box
//! that finds in it (`file-view.find`) and **Go to line** (`file-view.go-to-line`).

mod edit;
mod find;
mod go_to_line;
mod question;

use std::ops::Range;

use model::CommitId;
use ui::editor::{Buffer, Command, Cursor};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, CodeSpan, Icon, Key, Label, Modifiers, NamedKey, TextKind, Tone, Ui};

use crate::intent::Intent;
use crate::text::file_view as words;

pub use edit::edit_view;
pub use find::find_view;
pub use go_to_line::{GoToLineState, go_to_line_view, parse_line};
pub use question::{CloseQuestion, close_question_view};

/// The number of a File page, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilePageId(pub u64);

/// A file to open in a File page: a file of the working tree, or a file at a revision, and the line to open it at.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileTarget {
    /// The path from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version is shown, read only; none for the working tree.
    pub revision: Option<CommitId>,
    /// The line to open at, from 1; none for where the page is, or the first line.
    pub line: Option<usize>,
}

impl FileTarget {
    /// A file of the working tree, at its first line.
    #[must_use]
    pub fn working_tree(path: &str) -> FileTarget {
        FileTarget {
            path: path.to_owned(),
            revision: None,
            line: None,
        }
    }
}

/// What a File page shows under its bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileShown<'a> {
    /// The file is being read.
    Reading,
    /// The file has a NUL byte.
    NotText,
    /// The file could not be read: what the system or git said.
    Failed(&'a str),
    /// The text of the file in the editor.
    Text(FileText<'a>),
}

/// The text of a File page, as its editor draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileText<'a> {
    /// The text.
    pub buffer: &'a Buffer,
    /// The caret and the selection of the page.
    pub cursor: Cursor,
    /// The colors of the code, by line.
    pub colors: &'a [Vec<CodeSpan>],
    /// A line to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
    /// When the editor takes the keyboard, once for each number.
    pub focus: Option<u64>,
}

/// The switches of the box that finds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FindOptions {
    /// Whether `Parse` finds only `Parse`.
    pub match_case: bool,
    /// Whether a match is a whole word.
    pub whole_word: bool,
    /// Whether what is typed is a regular expression.
    pub is_regex: bool,
}

/// The box that finds, while it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindState<'a> {
    /// What is typed.
    pub typed: &'a str,
    /// Its switches.
    pub options: FindOptions,
    /// Every match, as bytes of the text, in order.
    pub marks: &'a [Range<usize>],
    /// The match gone to, by its place among them.
    pub current: Option<usize>,
    /// Whether what is typed is a regular expression that is not valid.
    pub is_invalid: bool,
    /// When the box takes the keyboard, once for each number: each time `Ctrl+F` is pressed.
    pub focus: u64,
}

/// What a File page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilePageState<'a> {
    /// Which page it is.
    pub id: FilePageId,
    /// Its title in the trail: the name of the file, and the short hash of a revision.
    pub title: &'a str,
    /// The path of the file from the top of the working tree.
    pub path: &'a str,
    /// Whether typing does nothing: a file at a revision.
    pub is_read_only: bool,
    /// Whether it has edits not saved.
    pub is_unsaved: bool,
    /// What it shows.
    pub shown: FileShown<'a>,
    /// The box that finds, while it is open.
    pub find: Option<FindState<'a>>,
    /// **Go to line**, while it is asked.
    pub go_to_line: Option<GoToLineState<'a>>,
    /// How many columns a tab goes to.
    pub tab_width: usize,
}

/// Something asked of the File pages.
#[derive(Clone, Debug, PartialEq)]
pub enum FileViewIntent {
    /// Open the page of a file, or show the one there is, at the line asked for.
    Open(FileTarget),
    /// Something asked of a page.
    Page(FilePageId, FilePageIntent),
    /// Save the file of the working tree open at this path, from the Changes page.
    Save(String),
    /// **Save all** on the Changes page: every file of the working tree open with edits not saved (`changes.save`).
    SaveAll,
    /// The answer to **Save changes to app.rs?**
    AnswerClose(CloseAnswer),
}

/// Something asked of a File page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePageIntent {
    /// What the keys and the pointer asked of the text and the cursor, in order.
    Edit(Vec<Command>),
    /// `Ctrl+S`.
    Save,
    /// **Open in external editor**.
    OpenExternal,
    /// `Ctrl+F`, or **Find** in the menu.
    OpenFind,
    /// The text of the box changed.
    FindTyped(String),
    /// A switch of the box changed.
    FindOptions(FindOptions),
    /// `F3`, `Enter` in the box, or the arrow ▼.
    FindNext,
    /// `Shift+F3`, `Shift+Enter` in the box, or the arrow ▲.
    FindPrevious,
    /// `Escape`, or the button that closes the box.
    CloseFind,
    /// `Ctrl+G`, or **Go to line** in the menu.
    OpenGoToLine,
    /// The text of **Go to line** changed.
    GoToLineTyped(String),
    /// **Go**: the line, from 1.
    GoToLine(usize),
    /// **Go to line** cancelled.
    CloseGoToLine,
}

/// The answers to **Save changes to app.rs?**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseAnswer {
    /// Save, then close.
    Save,
    /// Close, and forget the edits.
    DontSave,
    /// Keep the page open.
    Cancel,
}

/// The File page, over the rectangle its caller clipped it to: its bar, the box that finds when it is open, and the
/// editor, or what it says of a file it cannot show.
///
/// Feature: file-view.page
pub fn page_view(state: &FilePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut intents = Vec::new();
    ui.region(("file page", state.id), (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::new(rect);
        let bar = area.top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        bar_view(state, ui, bar, &mut intents);
        if state.find.is_some() {
            let find = area.top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
            intents.extend(ui.clip(find, |ui| find_view(state, ui)));
        }
        let body = area.rest();
        let said = match state.shown {
            FileShown::Text(_) => None,
            FileShown::Reading => Some(words::READING),
            FileShown::NotText => Some(words::NOT_TEXT),
            FileShown::Failed(error) => Some(error),
        };
        match said {
            Some(said) => {
                let place = inset(body, Insets::all(theme.sizes.spacing.huge));
                let _ = ui.label(place, &Label::new(said).tone(Tone::Muted));
            }
            None => intents.extend(ui.clip(body, |ui| edit_view(state, ui))),
        }
    });
    if state.go_to_line.is_some() {
        intents.extend(go_to_line_view(state, ui));
    }
    intents.extend(shortcuts(state, ui));
    intents
}

/// The bar of the page: the path of the file, and **Open in external editor** for a file of the working tree.
fn bar_view(state: &FilePageState<'_>, ui: &mut Ui<'_>, rect: ui::Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, spacing.medium));
    let mut parts = Area::with_gap(inner, spacing.medium);
    if !state.is_read_only {
        let external = Button::secondary(words::OPEN_EXTERNAL).with_icon(Icon::File);
        let (width, height) = ui.button_size(&external);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("open external", state.id), cell, &external).clicked {
            intents.push(page_intent(state, FilePageIntent::OpenExternal));
        }
    }
    let _ = ui.label(
        parts.rest(),
        &Label::new(state.path).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let line = ui::Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, ui::Axis::Horizontal);
}

/// The keys of the page no widget used: `Ctrl+S` saves, `Ctrl+F` finds, `Ctrl+G` (`Cmd+L` on macOS) goes to a line,
/// `F3` and `Shift+F3` go from match to match, and `Escape` closes the box that finds.
fn shortcuts(state: &FilePageState<'_>, ui: &Ui<'_>) -> Vec<Intent> {
    if !matches!(state.shown, FileShown::Text(_)) {
        return Vec::new();
    }
    let mut asked = Vec::new();
    for key in ui.unclaimed_keys() {
        let character = match key.key {
            Key::Character(character) => Some(character.to_ascii_lowercase()),
            Key::Named(_) => None,
        };
        let is_command = key.modifiers.command() && !key.modifiers.shift() && !key.modifiers.contains(Modifiers::ALT);
        let go_to_line_key = if cfg!(target_os = "macos") { 'l' } else { 'g' };
        match (key.key, character) {
            (_, Some('s')) if is_command && !state.is_read_only => asked.push(FilePageIntent::Save),
            (_, Some('f')) if is_command => asked.push(FilePageIntent::OpenFind),
            (_, Some(letter)) if is_command && letter == go_to_line_key => asked.push(FilePageIntent::OpenGoToLine),
            (Key::Named(NamedKey::Function(3)), _) if key.modifiers == Modifiers::NONE => {
                asked.push(FilePageIntent::FindNext);
            }
            (Key::Named(NamedKey::Function(3)), _) if key.modifiers == Modifiers::SHIFT => {
                asked.push(FilePageIntent::FindPrevious);
            }
            (Key::Named(NamedKey::Escape), _) if state.find.is_some() && key.modifiers == Modifiers::NONE => {
                asked.push(FilePageIntent::CloseFind);
            }
            _ => {}
        }
    }
    asked.into_iter().map(|intent| page_intent(state, intent)).collect()
}

/// What a page asks.
fn page_intent(state: &FilePageState<'_>, intent: FilePageIntent) -> Intent {
    Intent::FileView(FileViewIntent::Page(state.id, intent))
}
