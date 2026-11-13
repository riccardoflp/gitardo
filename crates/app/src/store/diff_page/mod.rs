//! The Diff pages as the store keeps them (`diff.page`): the two versions and the files of each, the read of each file
//! on a worker, the page opened once the first file is read (a file that looks binary or has too many differences
//! opens nothing, a very large one asks first), the page read again when `HEAD`, the index or the working tree it shows
//! changes, and what the user changed in it.

mod lines;
mod navigate;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use git::read::diff::{DiffRequest, FileVersion, VersionRequest};
use views::Notice;
use views::diff::{ChangePlace, DiffPageId, DiffPageState, DiffTarget, DiffVersion, FileDiff, Hint, PageLine};
use views::page::Page;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::diff::{DiffJob, DiffRead};

pub use navigate::reduce_page;

/// Something asked of the Diff pages.
pub fn reduce(state: &mut State, intent: views::diff::DiffIntent) -> Vec<Effect> {
    match intent {
        views::diff::DiffIntent::Open(target) => open(state, *target),
        views::diff::DiffIntent::Page(id, intent) => reduce_page(state, id, intent),
        views::diff::DiffIntent::AnswerLarge(is_continued) => answer_large(state, is_continued),
    }
}

/// The Diff pages, open or waiting for their first read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DiffPages {
    /// The pages, in the order they were asked for.
    pub pages: Vec<DiffPage>,
    /// The number of the next page.
    pub next: u64,
    /// The number of the last change brought to the middle of a page.
    pub serial: u64,
    /// The width of the left side of the last page dragged, which the next page opens with.
    pub last_width: Option<u32>,
    /// The page whose very large file waits for **Continue**.
    pub asking: Option<DiffPageId>,
}

/// A Diff page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffPage {
    /// Its number.
    pub id: DiffPageId,
    /// The folder of its repository, as found in the workspace.
    pub folder: PathBuf,
    /// The top folder of the working tree, where git runs and the paths are from.
    pub top: PathBuf,
    /// What it compares.
    pub target: DiffTarget,
    /// Its title in the trail.
    pub title: String,
    /// Whether it is among the pages of the window: a page of one file is not until its first read says it can be
    /// shown.
    pub is_open: bool,
    /// How far the diff of each file is.
    pub diffs: Vec<FileDiff>,
    /// The read of each file running.
    pub jobs: Vec<Option<JobId>>,
    /// The read of the repository it was last read against.
    pub generation: u64,
    /// For each file, the first rows of the folds clicked open.
    pub unfolded: Vec<BTreeSet<usize>>,
    /// Whether the unchanged rows are folded.
    pub is_collapsed: bool,
    /// The rows of the page.
    pub lines: Vec<PageLine>,
    /// Where each change is.
    pub changes: Vec<ChangePlace>,
    /// The current change.
    pub current: Option<usize>,
    /// The row to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
    /// What the bar says for a moment, and until when.
    pub hint: Option<(Hint, Duration)>,
    /// The width of the left side, as dragged.
    pub left_width: Option<u32>,
    /// The way `F7` is going from file to file, and the file it started from, while the file it went to is read.
    pub going: Option<(navigate::Direction, String)>,
}

/// Opens the Diff page of a target in the repository shown, or shows the one it has: there is never a second page for
/// the same diff. A page of several files opens at once; a page of one file opens once its file is read.
pub fn open(state: &mut State, target: DiffTarget) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, top, generation) = (read.folder.clone(), read.state.folder.clone(), read.generation);
    let found = state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.target == target && same_folder(&page.folder, &folder));
    if let Some(page) = found {
        if page.is_open {
            super::pages::show_page(state, Page::Diff(page.id));
        }
        return Vec::new();
    }
    let id = DiffPageId(state.diff_pages.next);
    state.diff_pages.next += 1;
    let count = target.files.len();
    let page = DiffPage {
        id,
        folder,
        top,
        title: target.page_title(),
        target,
        is_open: count != 1,
        diffs: vec![FileDiff::Loading; count],
        jobs: vec![None; count],
        generation,
        unfolded: vec![BTreeSet::new(); count],
        is_collapsed: state.settings.collapse_unchanged,
        lines: Vec::new(),
        changes: Vec::new(),
        current: None,
        reveal: None,
        hint: None,
        left_width: state.diff_pages.last_width,
        going: None,
    };
    state.diff_pages.pages.push(page);
    let is_open = count != 1;
    let effects: Vec<Effect> = (0..count)
        .filter_map(|file| read_file(state, id, file, is_open))
        .collect();
    if let Some(page) = page_mut(state, id) {
        lines::rebuild(page);
    }
    if is_open {
        super::pages::show_page(state, Page::Diff(id));
    }
    effects
}

/// Starts the read of a file of a page, the very large file asked about already when `is_confirmed`.
pub(super) fn read_file(state: &mut State, id: DiffPageId, file: usize, is_confirmed: bool) -> Option<Effect> {
    let head = state.shown_read().and_then(|read| read.state.head.commit.clone());
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let paths = page.target.files.get(file)?;
    let version = |side: &DiffVersion, path: &str| match side {
        DiffVersion::Head => head.clone().map_or(FileVersion::Absent, FileVersion::Commit),
        DiffVersion::Index => FileVersion::Index,
        DiffVersion::WorkingTree => FileVersion::WorkingTree {
            unsaved: super::file_page::unsaved_bytes(state, &page.top, path),
        },
        DiffVersion::Commit { commit, .. } => FileVersion::Commit(commit.clone()),
        DiffVersion::Empty => FileVersion::Absent,
        DiffVersion::Base => FileVersion::Base,
        DiffVersion::Resolved(files) => {
            files
                .iter()
                .find(|(path, _)| *path == paths.right)
                .map_or(FileVersion::Absent, |(_, text)| FileVersion::WorkingTree {
                    unsaved: Some(text.clone().into_bytes()),
                })
        }
    };
    let request = DiffRequest {
        left: VersionRequest {
            version: version(&page.target.left, &paths.left),
            path: paths.left.clone(),
        },
        right: VersionRequest {
            version: version(&page.target.right, &paths.right),
            path: paths.right.clone(),
        },
    };
    let kind = JobKind::ReadDiff {
        folder: page.top.clone(),
        job: DiffJob {
            page: id,
            file,
            request,
            reads_index: page.target.left == DiffVersion::Head && page.target.right == DiffVersion::WorkingTree,
            is_confirmed,
        },
    };
    let job = super::workspace::start_job(state, kind);
    let page = page_mut(state, id)?;
    if let Some(running) = page.jobs.get_mut(file) {
        *running = Some(job.id);
    }
    Some(Effect::Start(job))
}

/// Reads again the files of the open pages that show a file of a working tree, with the text a File page has not
/// saved yet (`file-view.edit`).
pub(super) fn read_file_again(state: &mut State, top: &std::path::Path, path: &str) -> Vec<Effect> {
    let to_read: Vec<(DiffPageId, usize)> = state
        .diff_pages
        .pages
        .iter()
        .filter(|page| page.is_open && same_folder(&page.top, top))
        .flat_map(|page| {
            let shows_working_tree = (
                page.target.left == DiffVersion::WorkingTree,
                page.target.right == DiffVersion::WorkingTree,
            );
            page.target
                .files
                .iter()
                .enumerate()
                .filter(move |(_, file)| {
                    (shows_working_tree.0 && file.left == path) || (shows_working_tree.1 && file.right == path)
                })
                .map(move |(index, _)| (page.id, index))
        })
        .collect();
    to_read
        .into_iter()
        .filter_map(|(id, file)| read_file(state, id, file, true))
        .collect()
}

/// A page by its number.
pub(super) fn page_mut(state: &mut State, id: DiffPageId) -> Option<&mut DiffPage> {
    state.diff_pages.pages.iter_mut().find(|page| page.id == id)
}

/// Takes the diff of a file read: a page waiting for its first read opens, refuses or asks; an open page shows it,
/// unless it is the same as before, and `F7` going from file to file passes over a file that cannot be shown.
pub fn diff_read(state: &mut State, job: JobId, result: DiffRead) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some((id, file)) = state.diff_pages.pages.iter().find_map(|page| {
        let file = page.jobs.iter().position(|running| *running == Some(job))?;
        Some((page.id, file))
    }) else {
        return Vec::new();
    };
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    if let Some(running) = page.jobs.get_mut(file) {
        *running = None;
    }
    if page.is_open {
        return navigate::shown(state, (id, file), result);
    }
    match result {
        DiffRead::Text(text) => {
            page.is_open = true;
            if let Some(diff) = page.diffs.get_mut(file) {
                *diff = FileDiff::Text(text);
            }
            lines::rebuild(page);
            super::pages::show_page(state, Page::Diff(id));
        }
        DiffRead::Large => state.diff_pages.asking = Some(id),
        DiffRead::Binary => refuse(state, id, Some(Notice::DiffLooksBinary)),
        DiffRead::TooComplex => refuse(state, id, Some(Notice::DiffTooComplex)),
        DiffRead::Failed(error) => {
            state.last_error = Some(error);
            refuse(state, id, None);
        }
    }
    Vec::new()
}

/// Closes a page that is open, and forgets it: the page shown before it, or the timeline, shows in its place.
pub(super) fn close(state: &mut State, id: DiffPageId) {
    if let Some(index) = state.pages.open.iter().position(|open| *open == Page::Diff(id)) {
        let _closed = state.pages.open.remove(index);
        state.pages.shown = match state.pages.shown {
            Some(shown) if shown == index => index.checked_sub(1),
            Some(shown) if shown > index => Some(shown - 1),
            other => other,
        };
    }
    state.diff_pages.pages.retain(|page| page.id != id);
}

/// Forgets a page that was not opened, saying why.
fn refuse(state: &mut State, id: DiffPageId, notice: Option<Notice>) {
    state.diff_pages.pages.retain(|page| page.id != id);
    if notice.is_some() {
        state.notice = notice;
    }
}

/// The answer to **This file is very large and the diff may be slow. Continue?**: **Continue** computes it, anything
/// else opens nothing.
pub fn answer_large(state: &mut State, is_continued: bool) -> Vec<Effect> {
    let Some(id) = state.diff_pages.asking.take() else {
        return Vec::new();
    };
    if is_continued {
        read_file(state, id, 0, true).into_iter().collect()
    } else {
        refuse(state, id, None);
        Vec::new()
    }
}

/// Forgets the pages closed, and cancels the reads of their files.
pub fn prune(state: &mut State) -> Vec<Effect> {
    let open = &state.pages.open;
    let (kept, closed): (Vec<DiffPage>, Vec<DiffPage>) = std::mem::take(&mut state.diff_pages.pages)
        .into_iter()
        .partition(|page| !page.is_open || open.contains(&Page::Diff(page.id)));
    state.diff_pages.pages = kept;
    closed
        .into_iter()
        .flat_map(|page| page.jobs.into_iter().flatten())
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Reads again the open pages of the repository shown that show `HEAD`, the index or the working tree, once the
/// repository was read again since they were.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.folder.clone(), read.generation);
    let to_read: Vec<(DiffPageId, usize)> = state
        .diff_pages
        .pages
        .iter_mut()
        .filter(|page| page.is_open && page.generation != generation && same_folder(&page.folder, &folder))
        .filter(|page| page.target.left.is_live() || page.target.right.is_live())
        .flat_map(|page| {
            page.generation = generation;
            let id = page.id;
            page.jobs
                .iter()
                .enumerate()
                .filter(|(_, running)| running.is_none())
                .map(move |(file, _)| (id, file))
                .collect::<Vec<_>>()
        })
        .collect();
    to_read
        .into_iter()
        .filter_map(|(id, file)| read_file(state, id, file, true))
        .collect()
}

/// What a page is drawn from.
#[must_use]
pub fn page_state(state: &State, id: DiffPageId) -> Option<DiffPageState<'_>> {
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let single_text = match page.diffs.as_slice() {
        [FileDiff::Text(text)] => Some(text.sides),
        _ => None,
    };
    let (left, right) = single_text.unwrap_or((true, true));
    Some(DiffPageState {
        id,
        title: &page.title,
        names: (side_name(&page.target.left, left), side_name(&page.target.right, right)),
        versions: (&page.target.left, &page.target.right),
        files: &page.target.files,
        diffs: &page.diffs,
        lines: &page.lines,
        changes: &page.changes,
        current: page.current,
        is_collapsed: page.is_collapsed,
        left_width: page.left_width,
        reveal: page.reveal,
        hint: page.hint,
        is_unsaved: page.target.files.iter().any(|file| {
            let shows = |version: &DiffVersion, path: &str| {
                *version == DiffVersion::WorkingTree
                    && super::file_page::unsaved_bytes(state, &page.top, path).is_some()
            };
            shows(&page.target.left, &file.left) || shows(&page.target.right, &file.right)
        }),
    })
}

/// The name over the column of a version, **Empty** when the file is not there.
fn side_name(version: &DiffVersion, is_there: bool) -> &str {
    if is_there {
        version.name()
    } else {
        views::text::diff::EMPTY
    }
}

/// The question asked before a very large diff, while it waits.
#[must_use]
pub fn large_question(state: &State) -> Option<views::diff::LargeDiffQuestion<'_>> {
    let id = state.diff_pages.asking?;
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    Some(views::diff::LargeDiffQuestion { title: &page.title })
}
