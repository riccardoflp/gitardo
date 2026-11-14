//! The blame of the File pages as the store keeps it (`blame.current-line`, `blame.file`): for each page, who last
//! changed the line under its cursor and each of its lines, read on a worker when what they depend on changes (the
//! line, the text, the commit checked out, `blame.ignoreWhitespaceAndMoves`); the lines typed marked not committed at
//! once; and the operations of the blame and of the history run on a page, with what they ask of the window after.

mod edits;
mod follow;
mod view;

use std::collections::BTreeMap;
use std::path::PathBuf;

use model::CommitId;
use model::blame::{BlameLine, BlameOfFile, FileToOpen, LineOfFile, ShownBlame};
use model::contract::{FeatureId, FollowUp, Target};
use model::file_history::{LinesOfFile, selected_lines};
use views::blame::BlameIntent;
use views::feature::FeatureIntent;
use views::file_view::{FilePageId, FileTarget};

use super::file_page::FilePage;
use super::message::{Effect, JobId};
use super::state::State;
use crate::work::blame::BlameAnswer;

pub use edits::{LineChange, line_changes, mark_edits};
pub use follow::follow;
pub use view::BlameData;

/// The blame of the File pages.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Blames {
    /// The blame of each page, by its number.
    pub pages: BTreeMap<FilePageId, PageBlame>,
    /// The page and the line, from 0, whose hover is open.
    pub hovered: Option<(FilePageId, usize)>,
}

/// Why a file shows its blame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Wanted {
    /// It does not.
    #[default]
    No,
    /// It was asked for, on this page (`blame.toggle-file`).
    Asked,
    /// Every file shows it (`fileBlame.everyFile`).
    EveryFile,
}

/// What a read of the blame of a page depends on: when one of them changes, it is read again.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Inputs {
    /// The commit checked out.
    pub head: Option<CommitId>,
    /// Whether whitespace and moved lines are ignored.
    pub ignores_whitespace: bool,
    /// A fingerprint of the text of the page.
    pub text: u64,
}

/// The blame of a File page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PageBlame {
    /// Who last changed each line, while the file shows its blame and it is read.
    pub lines: Option<Vec<Option<BlameLine>>>,
    /// Whether the file shows its blame, and why.
    pub wanted: Wanted,
    /// Whether its blame was hidden while every file shows one: it stays hidden until the page is closed.
    pub is_hidden_until_closed: bool,
    /// What the last read of the blame of the file asked was read with.
    pub file_asked: Option<Inputs>,
    /// The read of the blame of the file running.
    pub file_job: Option<JobId>,
    /// The commit checked out when git had no blame for the file, which every file showing its blame asks again
    /// only once it changes.
    pub has_none_at: Option<Option<CommitId>>,
    /// The line of the cursor, from 0, and who last changed it, once read; none when git said nothing of it.
    pub line: Option<(usize, Option<BlameLine>)>,
    /// The line, from 0, and the fingerprint of the text the last read of the line of the cursor asked was read with.
    pub line_asked: Option<(usize, u64)>,
    /// The read of the line of the cursor running.
    pub line_job: Option<JobId>,
}

/// What an operation of the blame asks of the window after it: the blame of a file shown or hidden, or a file opened
/// with its blame.
pub fn carry_out(state: &mut State, follow_up: FollowUp) -> Vec<Effect> {
    match follow_up {
        FollowUp::ShowBlame(blame) => show(state, *blame),
        FollowUp::HideBlame(path, revision) => hide(state, &path, revision.as_ref()),
        FollowUp::OpenFile(file) => open_file(state, *file),
        _ => Vec::new(),
    }
}

/// Something asked of the blame of a page.
pub fn reduce(state: &mut State, intent: &BlameIntent) -> Vec<Effect> {
    match *intent {
        BlameIntent::Hover(id, line) => {
            state.blames.hovered = line.map(|line| (id, line));
            Vec::new()
        }
        BlameIntent::Run(id, feature, line) => {
            state.blames.hovered = None;
            let target = target_of(state, id, feature, line).unwrap_or(Target::Head);
            super::feature::reduce(state, FeatureIntent::Run(feature, target))
        }
    }
}

/// What an operation run on a page is given: the file on disk for its history, its lines for the history of some
/// lines, and the line of the cursor, or the line given, from 1, for the operations of the blame.
#[must_use]
pub fn target_of(state: &State, id: FilePageId, feature: FeatureId, line: Option<usize>) -> Option<Target> {
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let document = page.document()?;
    let buffer = document.buffer();
    match feature.0 {
        "file-history.file" => Some(Target::Path(on_disk(page))),
        "file-history.lines" if page.revision.is_none() => {
            let range = page.cursor.range();
            let (start, end) = (buffer.position(range.start), buffer.position(range.end));
            let lines = selected_lines(buffer.text(), ((start.line, start.column), (end.line, end.column)));
            Some(Target::Lines(Box::new(LinesOfFile {
                file: on_disk(page),
                text: buffer.text().to_owned(),
                lines,
            })))
        }
        "file-history.lines" => None,
        _ => {
            let line = line.unwrap_or_else(|| buffer.position(page.cursor.head).line + 1);
            let blame = state.blames.pages.get(&id);
            let shown = match blame.and_then(|blame| blame.lines.as_ref()) {
                Some(lines) => ShownBlame::Line(lines.get(line.saturating_sub(1)).cloned().flatten()),
                None => ShownBlame::Hidden,
            };
            Some(Target::LineOfFile(Box::new(LineOfFile {
                path: page.path.clone(),
                revision: page.revision.clone(),
                line,
                unsaved: page.is_unsaved().then(|| buffer.to_bytes()),
                shown,
            })))
        }
    }
}

/// Where the file of a page is on disk.
fn on_disk(page: &FilePage) -> PathBuf {
    let mut file = page.top.clone();
    file.extend(page.path.split('/').filter(|name| !name.is_empty()));
    file
}

/// The page of a file of the repository shown, at a revision.
fn page_of(state: &State, path: &str, revision: Option<&CommitId>) -> Option<FilePageId> {
    let top = &state.shown_read()?.state.folder;
    state
        .file_pages
        .pages
        .iter()
        .find(|page| {
            page.path == path && page.revision.as_ref() == revision && crate::persisted::same_folder(&page.top, top)
        })
        .map(|page| page.id)
}

/// `blame.toggle-file` showed the blame of a file: its page shows it, as read.
pub fn show(state: &mut State, blame: BlameOfFile) -> Vec<Effect> {
    let Some(id) = page_of(state, &blame.path, blame.revision.as_ref()) else {
        return Vec::new();
    };
    let inputs = follow::inputs_of(state, id);
    let page = state.blames.pages.entry(id).or_default();
    page.lines = Some(blame.lines);
    page.wanted = Wanted::Asked;
    page.is_hidden_until_closed = false;
    page.has_none_at = None;
    page.file_asked = inputs;
    follow::cancel_file(state, id)
}

/// `blame.toggle-file` hid the blame of a file: while every file shows its blame, it stays hidden until its page is
/// closed.
pub fn hide(state: &mut State, path: &str, revision: Option<&CommitId>) -> Vec<Effect> {
    let Some(id) = page_of(state, path, revision) else {
        return Vec::new();
    };
    let is_on_every_file = state.settings.features.blame.is_on_every_file;
    let page = state.blames.pages.entry(id).or_default();
    page.lines = None;
    page.wanted = Wanted::No;
    page.file_asked = None;
    page.is_hidden_until_closed = is_on_every_file;
    follow::cancel_file(state, id)
}

/// `blame.previous` opened a file as it was before a commit: its page opens at the line, showing the blame read.
pub fn open_file(state: &mut State, file: FileToOpen) -> Vec<Effect> {
    let target = FileTarget {
        path: file.blame.path.clone(),
        revision: file.blame.revision.clone(),
        line: Some(file.line),
    };
    let mut effects = super::file_page::open(state, target);
    effects.extend(show(state, file.blame));
    effects
}

/// Takes a read of the blame: the line of the cursor, or every line of the file, of the page that asked for it while
/// it still wants it; a read cancelled, or that failed, changes nothing.
pub fn blame_read(state: &mut State, job: JobId, result: Result<BlameAnswer, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let head = state.shown_read().and_then(|read| read.state.head.commit.clone());
    let Some(page) = state
        .blames
        .pages
        .values_mut()
        .find(|page| page.file_job == Some(job) || page.line_job == Some(job))
    else {
        return Vec::new();
    };
    match result {
        Ok(BlameAnswer::Line(line)) => {
            page.line_job = None;
            page.line = page.line_asked.map(|(asked, _)| (asked, line));
        }
        Ok(BlameAnswer::File(lines)) => {
            page.file_job = None;
            if lines.is_none() {
                page.has_none_at = Some(head);
            }
            page.lines = lines;
        }
        Err(_) => {
            if page.file_job == Some(job) {
                page.file_job = None;
            } else {
                page.line_job = None;
            }
        }
    }
    Vec::new()
}

/// Whether a page shows the blame of its file, so that what is typed in it is marked not committed.
#[must_use]
pub fn shows_file(state: &State, id: FilePageId) -> bool {
    state.blames.pages.get(&id).is_some_and(|page| page.lines.is_some())
}
