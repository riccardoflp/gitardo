//! A text another page gives a File page, as edits not saved: the result of a Merge page with choices not saved, when
//! the page closes (`merge-view.close`) or opens the file at a line (`merge-view.open-at-line`). It is one edit, which
//! `Ctrl+Z` in the File page takes back to the file as it is on disk.

use ui::editor::Command;
use views::file_view::{FilePageId, FileTarget};

use super::{go_to, next_serial, page_mut};
use crate::persisted::same_folder;
use crate::store::message::Effect;
use crate::store::state::State;

/// Opens the File page of a file of the working tree of the repository shown, or shows the one it has, with a text in
/// place of what it holds, not saved, at a line, from 1; the text waits for the file to be read.
pub fn open_with_text(state: &mut State, path: &str, (text, line): (String, usize)) -> Vec<Effect> {
    let target = FileTarget {
        path: path.to_owned(),
        revision: None,
        line: Some(line),
    };
    let mut effects = super::open(state, target);
    let Some(folder) = state.shown_folder().cloned() else {
        return effects;
    };
    let found = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.revision.is_none() && page.path == path && same_folder(&page.folder, &folder))
        .map(|page| page.id);
    if let Some(id) = found {
        if let Some(page) = page_mut(state, id) {
            page.pending_text = Some((text, line));
        }
        effects.extend(take_pending(state, id));
    }
    effects
}

/// Takes the text a page was given, once its file is read: in place of its text when they differ, as one edit, and
/// the cursor at the line given.
pub(super) fn take_pending(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let serial = next_serial(state);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let Some(document) = page.document() else {
        return Vec::new();
    };
    let is_same = page
        .pending_text
        .as_ref()
        .is_none_or(|(text, _)| document.buffer().to_bytes() == text.as_bytes());
    let Some((text, line)) = page.pending_text.take() else {
        return Vec::new();
    };
    let effects = if is_same {
        Vec::new()
    } else {
        super::edit::edit(state, id, &[Command::SelectAll, Command::Paste(text)])
    };
    if let Some(page) = page_mut(state, id) {
        go_to(page, line, serial);
    }
    effects
}
