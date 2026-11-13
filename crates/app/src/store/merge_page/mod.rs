//! The Merge pages as the store keeps them (`merge-view.page`): the file in conflict of each, read and computed on a
//! worker, the page opened once the first read says it can be (a file not in conflict with a version on each side, one
//! that looks binary or one with too many differences opens nothing, a very large one asks first), the result as it is
//! being chosen, and the page read again when the repository changes.

mod choose;
mod edit;
mod lines;

use std::path::PathBuf;
use std::sync::Arc;

use model::conflicts::{SideNamesInput, side_names};
use model::merge_view::{ColumnLengths, MergeLayout, merge_rows};
use views::Notice;
use views::merge_view::{MergeIntent, MergeLabels, MergePageId, MergePageState, MergeText};
use views::page::Page;
use views::text::merge_view as words;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::merge_view::{MergeJob, MergeRead, MergeSidesText};

pub use choose::reduce_page;
pub use lines::open_at;

/// The Merge pages, open or waiting for their first read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MergePages {
    /// The pages, in the order they were asked for.
    pub pages: Vec<MergePage>,
    /// The number of the next page.
    pub next: u64,
    /// The number of the last region brought to the middle of a page.
    pub serial: u64,
    /// The page whose very large file waits for **Continue**.
    pub asking: Option<MergePageId>,
}

/// A Merge page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergePage {
    /// Its number.
    pub id: MergePageId,
    /// The folder of its repository, as found in the workspace.
    pub folder: PathBuf,
    /// The top folder of the working tree, where git runs and the path is from.
    pub top: PathBuf,
    /// The file, from the top of the working tree.
    pub path: String,
    /// Its title in the trail.
    pub title: String,
    /// The labels over its columns: the current side, the result, the incoming side.
    pub labels: [String; 3],
    /// Whether it is among the pages of the window: not until its first read says it can be shown.
    pub is_open: bool,
    /// The read running.
    pub job: Option<JobId>,
    /// The base and the two sides, once read.
    pub sides: Option<Arc<MergeSidesText>>,
    /// The texts and their regions, as last computed.
    pub text: Option<Box<MergeText>>,
    /// The number of the text of the result `text` was computed for.
    pub text_generation: u64,
    /// The rows.
    pub layout: MergeLayout,
    /// The result as it is now, with what was chosen in it.
    pub result: String,
    /// The file as it is on disk, as last read.
    pub saved: String,
    /// The number of the text of the result, one more for each edit.
    pub generation: u64,
    /// The results before each edit, the last one last: what `Ctrl+Z` brings back.
    pub undone: Vec<String>,
    /// The results `Ctrl+Z` took back, the last one last: what `Ctrl+Y` puts again.
    pub redone: Vec<String>,
    /// A choice made while the regions of the result were being computed again, made once they are.
    pub waiting: Option<views::merge_view::MergePageIntent>,
    /// The region chosen with the arrows or `F7`.
    pub current: Option<usize>,
    /// The row to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
    /// The line of the result to go to once the regions are known, from 0.
    pub pending_line: Option<usize>,
    /// The read of the repository it was last read against.
    pub read_generation: u64,
}

impl MergePage {
    /// Whether the result has changes that are not saved.
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        self.result != self.saved
    }
}

/// Something asked of the Merge pages.
pub fn reduce(state: &mut State, intent: MergeIntent) -> Vec<Effect> {
    match intent {
        MergeIntent::Open(path) => open(state, path),
        MergeIntent::OpenAt(path, line) => open_at(state, &path, Some(line)),
        MergeIntent::Page(id, intent) => reduce_page(state, id, intent),
        MergeIntent::AnswerLarge(is_continued) => answer_large(state, is_continued),
    }
}

/// Opens the Merge page of a file in conflict in the repository shown, or shows the one it has: there is one page for
/// a file. The page opens once its file is read.
pub fn open(state: &mut State, path: String) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, top, generation) = (read.folder.clone(), read.state.folder.clone(), read.generation);
    let found = state
        .merge_pages
        .pages
        .iter()
        .find(|page| page.path == path && same_folder(&page.folder, &folder));
    if let Some(page) = found {
        if page.is_open {
            super::pages::show_page(state, Page::Merge(page.id));
        }
        return Vec::new();
    }
    let id = MergePageId(state.merge_pages.next);
    state.merge_pages.next += 1;
    let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
    state.merge_pages.pages.push(MergePage {
        id,
        folder,
        top,
        title: words::title(&name),
        labels: [String::new(), words::result_label(&path), String::new()],
        path,
        is_open: false,
        job: None,
        sides: None,
        text: None,
        text_generation: 0,
        layout: MergeLayout::default(),
        result: String::new(),
        saved: String::new(),
        generation: 0,
        undone: Vec::new(),
        redone: Vec::new(),
        waiting: None,
        current: None,
        reveal: None,
        pending_line: None,
        read_generation: generation,
    });
    start_read(state, id, Reading::FromDisk).into_iter().collect()
}

/// What a read of a page reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    /// The versions and the file on disk, the very large file not asked about yet.
    FromDisk,
    /// The versions and the file on disk, the very large file asked about already.
    FromDiskConfirmed,
    /// The versions again, with the result the page holds when it has changes that are not saved.
    Again,
    /// Only the regions of the result the page holds, with the versions read before.
    Regions,
}

/// Starts a read of a page, cancelling the one running.
pub(super) fn start_read(state: &mut State, id: MergePageId, reading: Reading) -> Option<Effect> {
    let auto_resolve = state.settings.features.conflicts_auto_resolve;
    let page = page_mut(state, id)?;
    let unsaved = page.is_unsaved().then(|| page.result.clone());
    let (result, sides) = match reading {
        Reading::FromDisk | Reading::FromDiskConfirmed => (None, None),
        Reading::Again => (unsaved, None),
        Reading::Regions => (Some(page.result.clone()), page.sides.clone()),
    };
    let job = MergeJob {
        page: id,
        file: page.path.clone(),
        result,
        sides,
        auto_resolve,
        is_confirmed: reading != Reading::FromDisk,
        generation: page.generation,
    };
    let kind = JobKind::ReadMerge {
        folder: page.top.clone(),
        job,
    };
    let cancelled = page.job.take();
    if let Some(cancelled) = cancelled {
        let _cancelled = state.jobs.remove(&cancelled);
    }
    let job = super::workspace::start_job(state, kind);
    let page = page_mut(state, id)?;
    page.job = Some(job.id);
    Some(Effect::Start(job))
}

/// A page by its number.
pub(super) fn page_mut(state: &mut State, id: MergePageId) -> Option<&mut MergePage> {
    state.merge_pages.pages.iter_mut().find(|page| page.id == id)
}

/// Takes what the read of a page came to: a page waiting for its first read opens, refuses or asks; an open page shows
/// the regions of the text it holds now, and makes the choice that waited for them.
pub fn merge_read(state: &mut State, job: JobId, result: MergeRead) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(page) = state.merge_pages.pages.iter_mut().find(|page| page.job == Some(job)) else {
        return Vec::new();
    };
    page.job = None;
    let id = page.id;
    match result {
        MergeRead::Text {
            text,
            sides,
            is_from_disk,
            generation,
        } => return shown(state, id, (text, sides), (is_from_disk, generation)),
        MergeRead::Large => state.merge_pages.asking = Some(id),
        MergeRead::NotInConflict => refuse(state, id, Some(Notice::MergeNotInConflict)),
        MergeRead::Binary => refuse(state, id, Some(Notice::DiffLooksBinary)),
        MergeRead::TooComplex => refuse(state, id, Some(Notice::DiffTooComplex)),
        MergeRead::Failed(error) => {
            state.last_error = Some(error);
            refuse(state, id, None);
        }
    }
    Vec::new()
}

/// Shows the texts of a read, unless the result changed since it started; the first read opens the page.
fn shown(
    state: &mut State,
    id: MergePageId,
    (text, sides): (Box<MergeText>, Arc<MergeSidesText>),
    (is_from_disk, generation): (bool, u64),
) -> Vec<Effect> {
    let shows_base = state.persisted.merge_shows_base;
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    if generation != page.generation {
        return Vec::new();
    }
    if is_from_disk {
        if page.is_open && text.result != page.result {
            page.undone.push(std::mem::take(&mut page.result));
            page.redone.clear();
            page.generation += 1;
        }
        page.result.clone_from(&text.result);
        page.saved.clone_from(&text.result);
    }
    let roles = side_names(&SideNamesInput {
        operation: sides.operation,
        ..SideNamesInput::default()
    });
    page.labels[0] = words::side_label(words::CURRENT, roles.current.role.words());
    page.labels[2] = words::side_label(words::INCOMING, roles.incoming.role.words());
    page.sides = Some(sides);
    page.text_generation = page.generation;
    page.text = Some(text);
    page.current = page
        .current
        .filter(|current| page.text.as_ref().is_some_and(|text| *current < text.regions.len()));
    rebuild(page, shows_base);
    let is_first = !page.is_open;
    page.is_open = true;
    if is_first {
        super::pages::show_page(state, Page::Merge(id));
    }
    lines::go_to_pending(state, id);
    choose::after_regions(state, id)
}

/// The rows of a page, with the base or without it.
pub(super) fn rebuild(page: &mut MergePage, shows_base: bool) {
    let Some(text) = &page.text else {
        page.layout = MergeLayout::default();
        return;
    };
    let lengths = ColumnLengths {
        base: shows_base.then_some(text.base_lines.len()),
        current: text.current_lines.len(),
        result: text.result_lines.len(),
        incoming: text.incoming_lines.len(),
    };
    page.layout = merge_rows(&text.regions, lengths);
}

/// Forgets a page that was not opened, saying why; an open page whose file is no longer in conflict closes.
fn refuse(state: &mut State, id: MergePageId, notice: Option<Notice>) {
    let is_open = state.merge_pages.pages.iter().any(|page| page.id == id && page.is_open);
    if is_open {
        close(state, id);
        return;
    }
    state.merge_pages.pages.retain(|page| page.id != id);
    if notice.is_some() {
        state.notice = notice;
    }
}

/// The answer to **This file is very large and the merge may be slow to show. Continue?**: **Continue** shows it,
/// anything else opens nothing.
pub fn answer_large(state: &mut State, is_continued: bool) -> Vec<Effect> {
    let Some(id) = state.merge_pages.asking.take() else {
        return Vec::new();
    };
    if is_continued {
        start_read(state, id, Reading::FromDiskConfirmed).into_iter().collect()
    } else {
        refuse(state, id, None);
        Vec::new()
    }
}

/// Closes a page that is open, and forgets it: the page shown before it, or the timeline, shows in its place.
pub(super) fn close(state: &mut State, id: MergePageId) {
    if let Some(index) = state.pages.open.iter().position(|open| *open == Page::Merge(id)) {
        let _closed = state.pages.open.remove(index);
        state.pages.shown = match state.pages.shown {
            Some(shown) if shown == index => index.checked_sub(1),
            Some(shown) if shown > index => Some(shown - 1),
            other => other,
        };
    }
    state.merge_pages.pages.retain(|page| page.id != id);
}

/// Closes the Merge page of a file of the repository shown, once it is marked as resolved.
pub fn close_file(state: &mut State, path: &str) {
    let Some(folder) = state.shown_folder().cloned() else {
        return;
    };
    let found = state
        .merge_pages
        .pages
        .iter()
        .find(|page| page.path == path && same_folder(&page.folder, &folder))
        .map(|page| page.id);
    if let Some(id) = found {
        close(state, id);
    }
}

/// Forgets the pages closed, and cancels their reads.
pub fn prune(state: &mut State) -> Vec<Effect> {
    let open = &state.pages.open;
    let (kept, closed): (Vec<MergePage>, Vec<MergePage>) = std::mem::take(&mut state.merge_pages.pages)
        .into_iter()
        .partition(|page| !page.is_open || open.contains(&Page::Merge(page.id)));
    state.merge_pages.pages = kept;
    closed
        .into_iter()
        .filter_map(|page| page.job)
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Reads again the open pages of the repository shown once it was read again since they were: the file on disk, or
/// the result the page holds when it has changes that are not saved.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.folder.clone(), read.generation);
    let to_read: Vec<MergePageId> = state
        .merge_pages
        .pages
        .iter_mut()
        .filter(|page| page.is_open && page.read_generation != generation && same_folder(&page.folder, &folder))
        .filter(|page| page.job.is_none())
        .map(|page| {
            page.read_generation = generation;
            page.id
        })
        .collect();
    to_read
        .into_iter()
        .filter_map(|id| start_read(state, id, Reading::Again))
        .collect()
}

/// What a page is drawn from.
#[must_use]
pub fn page_state(state: &State, id: MergePageId) -> Option<(MergePageState<'_>, &str)> {
    let page = state.merge_pages.pages.iter().find(|page| page.id == id)?;
    let [current, result, incoming] = &page.labels;
    let shown = MergePageState {
        id,
        labels: MergeLabels {
            current,
            result,
            incoming,
        },
        text: page.text.as_deref(),
        layout: &page.layout,
        is_base_shown: state.persisted.merge_shows_base,
        current: page.current,
        reveal: page.reveal,
    };
    Some((shown, &page.title))
}

/// The question asked before a very large file is shown, while it waits.
#[must_use]
pub fn large_question(state: &State) -> Option<views::merge_view::LargeMergeQuestion<'_>> {
    let id = state.merge_pages.asking?;
    let page = state.merge_pages.pages.iter().find(|page| page.id == id)?;
    Some(views::merge_view::LargeMergeQuestion { title: &page.title })
}
