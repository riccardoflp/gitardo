//! What is asked of a File page: its text edited, with the colors of its code, its matches and its Diff pages kept up
//! to date (`file-view.edit`); the box that finds (`file-view.find`); **Go to line** (`file-view.go-to-line`); and the
//! operations of the page, `file-view.save` and `file-view.open-external`, with what follows a save.

use std::sync::Arc;

use model::contract::{Answers, FeatureId, Target};
use model::file_view::{FileAt, FileSave};
use text::find::{FindQuery, find_all};
use ui::editor::{Change, Command, Cursor, apply};
use views::file_view::{FilePageId, FilePageIntent};

use super::{FileContent, FilePage, FindBox, close, next_serial, page_mut};
use crate::store::feature::Ended;
use crate::store::message::Effect;
use crate::store::state::State;

/// Something asked of a page.
pub fn reduce_page(state: &mut State, id: FilePageId, intent: FilePageIntent) -> Vec<Effect> {
    let serial = next_serial(state);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    match intent {
        FilePageIntent::Edit(commands) => return edit(state, id, &commands),
        FilePageIntent::Markdown(asked) => return super::markdown::reduce(state, id, asked),
        FilePageIntent::Image(asked) => return super::image::reduce(state, id, asked),
        FilePageIntent::Save => return save(state, id, false),
        FilePageIntent::OpenExternal => return open_external(state, id),
        FilePageIntent::OpenFind => {
            let selected = page
                .document()
                .map(|document| document.buffer().slice(page.cursor.range()).to_owned());
            let find = page.find.get_or_insert_with(FindBox::default);
            if let Some(selected) = selected.filter(|text| !text.is_empty() && !text.contains('\n')) {
                find.typed = selected;
            }
            find.focus = serial;
            find_again(page);
        }
        FilePageIntent::FindTyped(text) => {
            if let Some(find) = page.find.as_mut() {
                find.typed = text;
                find.current = None;
            }
            find_again(page);
            go_to_match(page, Step::Nearest, serial);
        }
        FilePageIntent::FindOptions(options) => {
            if let Some(find) = page.find.as_mut() {
                find.options = options;
            }
            find_again(page);
            go_to_match(page, Step::Nearest, serial);
        }
        FilePageIntent::FindNext => go_to_match(page, Step::Next, serial),
        FilePageIntent::FindPrevious => go_to_match(page, Step::Previous, serial),
        FilePageIntent::CloseFind => {
            page.find = None;
            page.focus = Some(serial);
        }
        FilePageIntent::OpenGoToLine => {
            let line = page
                .document()
                .map_or(1, |document| document.buffer().position(page.cursor.head).line + 1);
            page.go_to_line = Some(line.to_string());
        }
        FilePageIntent::GoToLineTyped(text) => page.go_to_line = Some(text),
        FilePageIntent::GoToLine(line) => {
            page.go_to_line = None;
            super::go_to(page, line, serial);
        }
        FilePageIntent::CloseGoToLine => {
            page.go_to_line = None;
            page.focus = Some(serial);
        }
    }
    Vec::new()
}

/// Does what the keys and the pointer asked of the text of a page: the colors of its code and its matches follow, the
/// Diff pages of the file follow the text, and they are read again with the text not saved.
pub(super) fn edit(state: &mut State, id: FilePageId, commands: &[Command]) -> Vec<Effect> {
    let Some(edited) = edit_with(state, id, commands, None).filter(|edited| !edited.changes.is_empty()) else {
        return Vec::new();
    };
    let file = (edited.top.as_path(), edited.path.as_str());
    crate::store::diff_page::follow_document(state, file, (&edited.changes, None));
    crate::store::diff_page::read_file_again(state, &edited.top, &edited.path)
}

/// What an edit of the text of a file did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edited {
    /// How the text changed, in order.
    pub changes: Vec<Change>,
    /// The cursor given, where the edit left it.
    pub cursor: Cursor,
    /// The top folder of the working tree of the file.
    pub top: std::path::PathBuf,
    /// The path of the file from it.
    pub path: String,
}

/// Does what the keys and the pointer asked of the text of a page, with the cursor of the page, or with another one
/// given, such as that of a Diff page that edits the file, whose edits move the cursor of the page as they move its
/// text; the colors of its code, its matches and its blame follow. Says how the text changed, none when it did not.
pub fn edit_with(state: &mut State, id: FilePageId, commands: &[Command], given: Option<Cursor>) -> Option<Edited> {
    let is_blamed = crate::store::blame::shows_file(state, id);
    let page = page_mut(state, id)?;
    let is_read_only = page.revision.is_some();
    let FileContent::Text(document) = &mut page.content else {
        return None;
    };
    let mut cursor = given.unwrap_or(page.cursor);
    let mut changes = Vec::new();
    let mut blamed_edits = Vec::new();
    for command in commands {
        if is_read_only && command.is_edit() {
            continue;
        }
        let before = is_blamed.then(|| document.buffer().text().to_owned());
        let made = apply(document, &mut cursor, command);
        if let Some(before) = before {
            blamed_edits.extend(crate::store::blame::line_changes(
                &before,
                &made,
                document.buffer().text(),
            ));
        }
        changes.extend(made);
    }
    if given.is_none() {
        page.cursor = cursor;
    } else {
        for change in &changes {
            page.cursor.follow(change);
        }
    }
    match changes.as_slice() {
        [] => {}
        [change] => page.colors.follow(document.buffer(), change),
        _ => page.colors = super::Colors::of(&page.path, document.buffer()),
    }
    if !changes.is_empty() {
        find_again(page);
    }
    let edited = Edited {
        changes,
        cursor,
        top: page.top.clone(),
        path: page.path.clone(),
    };
    crate::store::blame::mark_edits(state, id, &blamed_edits);
    Some(edited)
}

/// Finds again what the box of a page has typed, in the text as it is.
pub(super) fn find_again(page: &mut FilePage) {
    let Some(find) = page.find.as_mut() else {
        return;
    };
    let text = match &page.content {
        FileContent::Text(document) => document.buffer().text(),
        _ => "",
    };
    let query = FindQuery {
        text: &find.typed,
        match_case: find.options.match_case,
        whole_word: find.options.whole_word,
        is_regex: find.options.is_regex,
    };
    if let Ok(marks) = find_all(text, &query) {
        find.current = find.current.filter(|current| *current < marks.len());
        find.marks = marks;
        find.is_invalid = false;
    } else {
        find.marks = Vec::new();
        find.current = None;
        find.is_invalid = true;
    }
}

/// Which match to go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// The first at the cursor or after it, as what is typed changes.
    Nearest,
    /// The one after the current one, the first after the last.
    Next,
    /// The one before the current one, the last before the first.
    Previous,
}

/// Goes to a match: it is selected and brought to the middle.
fn go_to_match(page: &mut FilePage, step: Step, serial: u64) {
    let Some(find) = page.find.as_mut() else {
        return;
    };
    let count = find.marks.len();
    if count == 0 {
        return;
    }
    let from = page.cursor.range().start;
    let nearest = find.marks.iter().position(|mark| mark.start >= from).unwrap_or(0);
    let current = match (step, find.current) {
        (Step::Nearest, _) | (Step::Next | Step::Previous, None) => nearest,
        (Step::Next, Some(current)) => (current + 1) % count,
        (Step::Previous, Some(current)) => (current + count - 1) % count,
    };
    find.current = Some(current);
    let Some(mark) = find.marks.get(current).cloned() else {
        return;
    };
    if let FileContent::Text(document) = &page.content {
        page.reveal = Some((serial, document.buffer().position(mark.start).line));
    }
    page.cursor = Cursor {
        anchor: mark.start,
        head: mark.end,
        goal: None,
    };
}

/// Runs `file-view.save` on the text of a page with edits not saved, to close the page once it is saved when asked.
pub(crate) fn save(state: &mut State, id: FilePageId, then_close: bool) -> Vec<Effect> {
    let Some(page) = state.file_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let Some(document) = page.document().filter(|document| document.is_unsaved()) else {
        return Vec::new();
    };
    let contents = document.buffer().to_bytes();
    let target = Target::FileSave(Box::new(FileSave {
        path: page.path.clone(),
        contents: contents.clone(),
        read: page.read.clone(),
    }));
    let effects = run(state, FeatureId("file-view.save"), target);
    if !effects.is_empty() {
        state.file_pages.saving = Some((id, contents, then_close));
    }
    effects
}

/// Runs `file-view.open-external` on the file of a page, at the line of its cursor.
fn open_external(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let Some(page) = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.id == id && page.revision.is_none())
    else {
        return Vec::new();
    };
    let line = page
        .document()
        .map_or(1, |document| document.buffer().position(page.cursor.head).line + 1);
    let target = Target::FileAt(FileAt {
        path: page.path.clone(),
        line,
    });
    run(state, FeatureId("file-view.open-external"), target)
}

/// Runs an operation of the File page in the repository shown.
fn run(state: &mut State, feature: FeatureId, target: Target) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = (read.folder.clone(), Arc::clone(&read.state));
    crate::store::feature::run_in(state, repository, (feature, target), Answers::new())
}

/// A save ended: once written, the text is what is saved and on disk, and the page closes when that was asked; a save
/// refused, cancelled or failed leaves the edits not saved.
pub fn ended(state: &mut State, ended: &Ended) {
    super::save_all::ended(state, ended);
    let is_saving = state
        .operations
        .running
        .as_ref()
        .is_some_and(|running| running.feature == FeatureId("file-view.save"));
    if !is_saving {
        return;
    }
    let Some((id, written, then_close)) = state.file_pages.saving.take() else {
        return;
    };
    if !matches!(ended, Ended::Ran { .. }) {
        return;
    }
    super::save_all::saved(state, id, written);
    if then_close {
        close(state, id);
    }
}
