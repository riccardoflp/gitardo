//! The File pages as the store keeps them (`file-view.page`): the file of each, read on a worker from the disk or at a
//! revision, its text open in the editor with the cursor of the page, read again when it changes on disk and has no
//! edits not saved, and the question asked before closing a page whose edits are not saved.

mod colors;
mod edit;
mod given;
mod save_all;

use std::ops::Range;
use std::path::{Path, PathBuf};

use model::CommitId;
use ui::editor::{Cursor, Document, NotText};
use views::file_view::{
    CloseAnswer, CloseQuestion, FilePageId, FilePageState, FileShown, FileTarget, FileText, FileViewIntent,
    FindOptions, FindState, GoToLineState,
};
use views::page::Page;
use views::text::file_view as words;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::file_view::FileJob;

pub use colors::{Colors, colors_of};
pub use edit::{ended, reduce_page};
pub use given::open_with_text;

/// How many columns a tab goes to in the editor.
const TAB_WIDTH: usize = 4;

/// Something asked of the File pages.
pub fn reduce(state: &mut State, intent: FileViewIntent) -> Vec<Effect> {
    match intent {
        FileViewIntent::Open(target) => open(state, target),
        FileViewIntent::Page(id, intent) => reduce_page(state, id, intent),
        FileViewIntent::Save(path) => {
            let found = state
                .file_pages
                .pages
                .iter()
                .find(|page| page.revision.is_none() && page.path == path && page.is_unsaved())
                .map(|page| page.id);
            found.map(|id| edit::save(state, id, false)).unwrap_or_default()
        }
        FileViewIntent::SaveAll => save_all::save_all(state),
        FileViewIntent::AnswerClose(answer) => answer_close(state, answer),
    }
}

/// The File pages open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FilePages {
    /// The pages, in the order they were opened.
    pub pages: Vec<FilePage>,
    /// The number of the next page.
    pub next: u64,
    /// The number of the last line brought to the middle, or of the last time the editor or a box took the keyboard.
    pub serial: u64,
    /// The page whose close asks **Save changes to app.rs?**
    pub closing: Option<FilePageId>,
    /// The page being saved, the bytes written, and whether it closes once they are.
    pub saving: Option<(FilePageId, Vec<u8>, bool)>,
    /// The pages being saved by **Save all**, each with the bytes written.
    pub saving_all: Vec<(FilePageId, Vec<u8>)>,
}

/// What a File page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileContent {
    /// The file is being read.
    Reading,
    /// The file has a NUL byte.
    NotText,
    /// The file could not be read.
    Failed(String),
    /// The text of the file.
    Text(Box<Document>),
}

/// The box that finds in a page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FindBox {
    /// What is typed.
    pub typed: String,
    /// Its switches.
    pub options: FindOptions,
    /// Every match, as bytes of the text.
    pub marks: Vec<Range<usize>>,
    /// The match gone to.
    pub current: Option<usize>,
    /// Whether what is typed is a regular expression that is not valid.
    pub is_invalid: bool,
    /// When the box takes the keyboard.
    pub focus: u64,
}

/// A File page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePage {
    /// Its number.
    pub id: FilePageId,
    /// The folder of its repository, as found in the workspace.
    pub folder: PathBuf,
    /// The top folder of the working tree, where the path is from.
    pub top: PathBuf,
    /// The path of the file from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version it shows, read only; none for the working tree.
    pub revision: Option<CommitId>,
    /// Its title in the trail.
    pub title: String,
    /// What it shows.
    pub content: FileContent,
    /// The bytes on disk when they were last read or saved.
    pub read: Vec<u8>,
    /// The caret and the selection.
    pub cursor: Cursor,
    /// The colors of the code.
    pub colors: Colors,
    /// The line to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
    /// When the editor takes the keyboard.
    pub focus: Option<u64>,
    /// The line asked for before the file was read, from 1.
    pub pending_line: Option<usize>,
    /// A text another page gave it, to take as edits not saved once the file is read, with the line to go to, from 1.
    pub pending_text: Option<(String, usize)>,
    /// The read running.
    pub job: Option<JobId>,
    /// The read of the repository it was last read against.
    pub generation: u64,
    /// The box that finds, while it is open.
    pub find: Option<FindBox>,
    /// What is typed in **Go to line**, while it is asked.
    pub go_to_line: Option<String>,
}

impl FilePage {
    /// The document of the page, once the file is read as text.
    #[must_use]
    pub fn document(&self) -> Option<&Document> {
        match &self.content {
            FileContent::Text(document) => Some(document),
            _ => None,
        }
    }

    /// Whether it has edits not saved.
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        self.document().is_some_and(Document::is_unsaved)
    }
}

/// Opens the page of a file of the repository shown, or shows the one it has, at the line asked for.
pub fn open(state: &mut State, target: FileTarget) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, top, generation) = (read.folder.clone(), read.state.folder.clone(), read.generation);
    let found = state.file_pages.pages.iter().position(|page| {
        page.path == target.path && page.revision == target.revision && same_folder(&page.folder, &folder)
    });
    if let Some(index) = found {
        let serial = next_serial(state);
        if let Some(page) = state.file_pages.pages.get_mut(index) {
            if let Some(line) = target.line {
                go_to(page, line, serial);
            }
            let id = page.id;
            super::pages::show_page(state, Page::File(id));
        }
        return Vec::new();
    }
    let id = FilePageId(state.file_pages.next);
    state.file_pages.next += 1;
    let name = target
        .path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&target.path)
        .to_owned();
    let title = match &target.revision {
        Some(commit) => words::at_revision(&name, commit.short()),
        None => name,
    };
    state.file_pages.pages.push(FilePage {
        id,
        folder,
        top,
        path: target.path,
        revision: target.revision,
        title,
        content: FileContent::Reading,
        read: Vec::new(),
        cursor: Cursor::default(),
        colors: Colors::default(),
        reveal: None,
        focus: None,
        pending_line: target.line,
        pending_text: None,
        job: None,
        generation,
        find: None,
        go_to_line: None,
    });
    super::pages::show_page(state, Page::File(id));
    start_read(state, id).into_iter().collect()
}

/// The next number of a reveal or of a focus.
fn next_serial(state: &mut State) -> u64 {
    state.file_pages.serial += 1;
    state.file_pages.serial
}

/// Puts the cursor at the start of a line, from 1, brought to the middle; a line past the end is the last line. Before
/// the file is read, the line waits for it.
fn go_to(page: &mut FilePage, line: usize, serial: u64) {
    let Some(document) = page.document() else {
        page.pending_line = Some(line);
        return;
    };
    let buffer = document.buffer();
    let index = line.saturating_sub(1).min(buffer.line_count().saturating_sub(1));
    page.cursor = Cursor::at(buffer.line_range(index).start);
    page.reveal = Some((serial, index));
    page.focus = Some(serial);
}

/// Starts the read of the file of a page.
fn start_read(state: &mut State, id: FilePageId) -> Option<Effect> {
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let kind = JobKind::ReadFile {
        folder: page.top.clone(),
        job: FileJob {
            page: id,
            path: page.path.clone(),
            revision: page.revision.clone(),
        },
    };
    let job = super::workspace::start_job(state, kind);
    let page = page_mut(state, id)?;
    page.job = Some(job.id);
    Some(Effect::Start(job))
}

/// A page by its number.
pub(super) fn page_mut(state: &mut State, id: FilePageId) -> Option<&mut FilePage> {
    state.file_pages.pages.iter_mut().find(|page| page.id == id)
}

/// Takes the file read: the first read shows it, at the line asked for; a read again replaces the text when the page
/// has no edits not saved and the file changed.
pub fn file_read(state: &mut State, job: JobId, result: Result<Option<Vec<u8>>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(id) = state
        .file_pages
        .pages
        .iter()
        .find(|page| page.job == Some(job))
        .map(|page| page.id)
    else {
        return Vec::new();
    };
    let serial = next_serial(state);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    page.job = None;
    let is_first = page.content == FileContent::Reading;
    let bytes = match result {
        Ok(Some(bytes)) => bytes,
        Ok(None) if is_first => {
            page.content = FileContent::Failed(format!("{} is not there.", page.path));
            return Vec::new();
        }
        Err(error) if is_first => {
            page.content = FileContent::Failed(error);
            return Vec::new();
        }
        // A read again that finds the file gone, or fails, leaves the page as it was.
        Ok(None) | Err(_) => return Vec::new(),
    };
    if !is_first && (page.is_unsaved() || bytes == page.read) {
        return Vec::new();
    }
    take_bytes(page, bytes);
    if is_first {
        let line = page.pending_line.take().unwrap_or(1);
        go_to(page, line, serial);
    }
    given::take_pending(state, id)
}

/// Shows the bytes of a file: its text in a new document, keeping the cursor where it can, or that it is not text.
fn take_bytes(page: &mut FilePage, bytes: Vec<u8>) {
    match Document::from_bytes(&bytes) {
        Ok(document) => {
            let length = document.buffer().text().len();
            let boundary = |offset: usize| document.buffer().boundary_at_or_before(offset.min(length));
            page.cursor = Cursor {
                anchor: boundary(page.cursor.anchor),
                head: boundary(page.cursor.head),
                goal: None,
            };
            page.colors = Colors::of(&page.path, document.buffer());
            page.content = FileContent::Text(Box::new(document));
            edit::find_again(page);
        }
        Err(NotText::Binary) => page.content = FileContent::NotText,
    }
    page.read = bytes;
}

/// Reads again the pages of files of the working tree of the repository shown, once the repository was read again
/// since they were: a file changed in another program shows its new text, unless the page has edits not saved.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.folder.clone(), read.generation);
    let to_read: Vec<FilePageId> = state
        .file_pages
        .pages
        .iter_mut()
        .filter(|page| page.revision.is_none() && page.generation != generation && same_folder(&page.folder, &folder))
        .filter_map(|page| {
            page.generation = generation;
            let can_read = page.job.is_none() && !page.is_unsaved() && page.content != FileContent::Reading;
            can_read.then_some(page.id)
        })
        .collect();
    to_read.into_iter().filter_map(|id| start_read(state, id)).collect()
}

/// Forgets the pages closed, and cancels their reads.
pub fn prune(state: &mut State) -> Vec<Effect> {
    let open = &state.pages.open;
    let (kept, closed): (Vec<FilePage>, Vec<FilePage>) = std::mem::take(&mut state.file_pages.pages)
        .into_iter()
        .partition(|page| open.contains(&Page::File(page.id)));
    state.file_pages.pages = kept;
    closed
        .into_iter()
        .filter_map(|page| page.job)
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Closes the page shown, unless it is a File page with edits not saved, which asks first.
pub fn close_page(state: &mut State) -> Vec<Effect> {
    if let Some(Page::File(id)) = state.page_shown()
        && state
            .file_pages
            .pages
            .iter()
            .any(|page| page.id == id && page.is_unsaved())
    {
        state.file_pages.closing = Some(id);
        return Vec::new();
    }
    super::pages::close_page(state)
}

/// The answer to **Save changes to app.rs?**: **Save** saves, then closes; **Don't save** closes and forgets the edits;
/// **Cancel** keeps the page.
fn answer_close(state: &mut State, answer: CloseAnswer) -> Vec<Effect> {
    let Some(id) = state.file_pages.closing.take() else {
        return Vec::new();
    };
    match answer {
        CloseAnswer::Save => edit::save(state, id, true),
        CloseAnswer::DontSave => {
            close(state, id);
            Vec::new()
        }
        CloseAnswer::Cancel => Vec::new(),
    }
}

/// Closes a page: the page shown before it, or the timeline, shows in its place.
pub(super) fn close(state: &mut State, id: FilePageId) {
    if let Some(index) = state.pages.open.iter().position(|open| *open == Page::File(id)) {
        let _closed = state.pages.open.remove(index);
        state.pages.shown = match state.pages.shown {
            Some(shown) if shown == index => index.checked_sub(1),
            Some(shown) if shown > index => Some(shown - 1),
            other => other,
        };
    }
}

/// The text not saved of a file of a working tree, as it would be written, when a File page has edits not saved.
#[must_use]
pub fn unsaved_bytes(state: &State, top: &Path, path: &str) -> Option<Vec<u8>> {
    state
        .file_pages
        .pages
        .iter()
        .find(|page| page.revision.is_none() && page.path == path && same_folder(&page.top, top) && page.is_unsaved())
        .and_then(FilePage::document)
        .map(|document| document.buffer().to_bytes())
}

/// The paths of the files of a working tree with edits not saved, for the Changes page; a file outside it, such as the
/// user's settings file, is not one of them.
#[must_use]
pub fn unsaved_paths(state: &State, top: &Path) -> Vec<String> {
    state
        .file_pages
        .pages
        .iter()
        .filter(|page| page.revision.is_none() && same_folder(&page.top, top) && page.is_unsaved())
        .filter(|page| !Path::new(&page.path).is_absolute())
        .map(|page| page.path.clone())
        .collect()
}

/// What a page is drawn from.
#[must_use]
pub fn page_state(state: &State, id: FilePageId) -> Option<FilePageState<'_>> {
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let shown = match &page.content {
        FileContent::Reading => FileShown::Reading,
        FileContent::NotText => FileShown::NotText,
        FileContent::Failed(error) => FileShown::Failed(error),
        FileContent::Text(document) => FileShown::Text(FileText {
            buffer: document.buffer(),
            cursor: page.cursor,
            colors: &page.colors.spans,
            reveal: page.reveal,
            focus: page.focus,
        }),
    };
    let line_count = page.document().map_or(1, |document| document.buffer().line_count());
    Some(FilePageState {
        id,
        title: &page.title,
        path: &page.path,
        is_read_only: page.revision.is_some(),
        is_unsaved: page.is_unsaved(),
        shown,
        find: page.find.as_ref().map(|find| FindState {
            typed: &find.typed,
            options: find.options,
            marks: &find.marks,
            current: find.current,
            is_invalid: find.is_invalid,
            focus: find.focus,
        }),
        go_to_line: page
            .go_to_line
            .as_deref()
            .map(|typed| GoToLineState { typed, line_count }),
        tab_width: TAB_WIDTH,
        blame: views::blame::PageBlame::default(),
    })
}

/// The question asked before closing a page with edits not saved, while it waits.
#[must_use]
pub fn close_question(state: &State) -> Option<CloseQuestion<'_>> {
    let id = state.file_pages.closing?;
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let name = page.path.rsplit('/').next().unwrap_or(&page.path);
    Some(CloseQuestion { name })
}
