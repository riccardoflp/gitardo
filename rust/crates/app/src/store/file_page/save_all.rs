//! **Save all** on the Changes page (`changes.save`): the text of every File page of the working tree of the repository
//! shown that has edits not saved, written at once, and each page marked saved once it is.

use std::sync::Arc;

use model::contract::{Answers, FeatureId, Target};
use model::file_view::FileSave;
use views::file_view::FilePageId;

use super::{FileContent, page_mut};
use crate::persisted::same_folder;
use crate::store::feature::Ended;
use crate::store::message::Effect;
use crate::store::state::State;

/// The operation of **Save all**.
const SAVE: FeatureId = FeatureId("changes.save");

/// Runs `changes.save` on the pages of files of the working tree of the repository shown with edits not saved.
pub(super) fn save_all(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = (read.folder.clone(), Arc::clone(&read.state));
    let unsaved: Vec<(FilePageId, FileSave)> = state
        .file_pages
        .pages
        .iter()
        .filter(|page| page.revision.is_none() && same_folder(&page.folder, &repository.0))
        .filter_map(|page| {
            let document = page.document().filter(|document| document.is_unsaved())?;
            let file = FileSave {
                path: page.path.clone(),
                contents: document.buffer().to_bytes(),
                read: page.read.clone(),
            };
            Some((page.id, file))
        })
        .collect();
    if unsaved.is_empty() {
        return Vec::new();
    }
    let saving: Vec<(FilePageId, Vec<u8>)> = unsaved.iter().map(|(id, file)| (*id, file.contents.clone())).collect();
    let target = Target::FileSaves(unsaved.into_iter().map(|(_, file)| file).collect());
    let effects = crate::store::feature::run_in(state, repository, (SAVE, target), Answers::new());
    if !effects.is_empty() {
        state.file_pages.saving_all = saving;
    }
    effects
}

/// **Save all** ended: once written, each text is what is saved and on disk; refused, cancelled or failed, the edits
/// stay not saved.
pub(super) fn ended(state: &mut State, ended: &Ended) {
    let is_saving = state
        .operations
        .running
        .as_ref()
        .is_some_and(|running| running.feature == SAVE);
    if !is_saving {
        return;
    }
    let saving = std::mem::take(&mut state.file_pages.saving_all);
    if !matches!(ended, Ended::Ran { .. }) {
        return;
    }
    for (id, written) in saving {
        saved(state, id, written);
    }
}

/// A page whose text was written: the text is what is saved, unless it was typed in since, and what is on disk.
pub(super) fn saved(state: &mut State, id: FilePageId, written: Vec<u8>) {
    let Some(page) = page_mut(state, id) else {
        return;
    };
    if let FileContent::Text(document) = &mut page.content
        && document.buffer().to_bytes() == written
    {
        document.mark_saved();
    }
    page.read = written;
}
