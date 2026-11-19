//! The text of a file that a Diff page edits while no File page shows it (`diff.edit`): kept in a File page that is not
//! among the pages of the window, so that the File page of the file, the Changes page and **Save** find it as they find
//! any other. Opening the File page of the file shows it; it is forgotten once it has no edits not saved and no Diff
//! page shows the file.

use std::path::{Path, PathBuf};

use ui::editor::{Cursor, Document};
use views::diff::DiffVersion;
use views::file_view::FilePageId;

use super::{Colors, FileContent, FilePage};
use crate::persisted::same_folder;
use crate::store::state::State;

/// The File page, shown or not, that holds the text of a file of a working tree.
#[must_use]
pub fn document_of(state: &State, top: &Path, path: &str) -> Option<FilePageId> {
    state
        .file_pages
        .pages
        .iter()
        .find(|page| page.revision.is_none() && page.path == path && same_folder(&page.top, top))
        .filter(|page| page.document().is_some())
        .map(|page| page.id)
}

/// Where a file is: the folder of its repository as found in the workspace, the top folder of its working tree, and
/// the read of the repository it was read against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldFile {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// The top folder of the working tree.
    pub top: PathBuf,
    /// The read of the repository.
    pub generation: u64,
}

/// Holds the text of a file of a working tree in a File page that is not shown, from the bytes a Diff page read of it;
/// none for bytes that are not text.
pub fn hold(state: &mut State, file: HeldFile, (path, bytes): (&str, &[u8])) -> Option<FilePageId> {
    let document = Document::from_bytes(bytes).ok()?;
    let id = FilePageId(state.file_pages.next);
    state.file_pages.next += 1;
    let title = path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned();
    state.file_pages.pages.push(FilePage {
        id,
        folder: file.folder,
        top: file.top,
        path: path.to_owned(),
        revision: None,
        title,
        colors: Colors::of(path, document.buffer()),
        content: FileContent::Text(Box::new(document)),
        read: bytes.to_vec(),
        cursor: Cursor::default(),
        reveal: None,
        focus: None,
        pending_line: None,
        pending_text: None,
        job: None,
        generation: file.generation,
        find: None,
        go_to_line: None,
        is_hidden: true,
        markdown: super::markdown::preview_of(path, state.settings.markdown_open_as_preview),
        image: None,
    });
    Some(id)
}

/// Whether a page not shown is still needed: it has edits not saved, or an open Diff page shows its file as the working
/// tree.
pub(super) fn is_needed(state: &State, page: &FilePage) -> bool {
    page.is_unsaved()
        || state.diff_pages.pages.iter().any(|diff| {
            diff.is_open
                && diff.target.right == DiffVersion::WorkingTree
                && same_folder(&diff.top, &page.top)
                && diff.target.files.iter().any(|file| file.right == page.path)
        })
}
