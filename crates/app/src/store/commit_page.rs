//! The pages of commits as the store keeps them (`commit-page.open`, `commit-page.view`): the commit of each and the
//! repository it is in, its read on a worker, and what the user changed in it; and **Show in the timeline**
//! (`commit-page.show-in-graph`), which points the window back at the repository of the page and loads the history until
//! the commit is in it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use model::CommitId;
use model::commit_page::{Fold, FoldSet, PatchLines, initial_folds};
use model::details::{CommitDetail, TreeNode};
use views::commit_page::{CommitPageId, CommitPageIntent, CommitPageState, PageData, PageRead, PageTarget};
use views::details::FilesIntent;
use views::page::Page;
use views::text::details::page_title;

use super::inspector::{shape, toggle_fold, tree_of};
use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;

/// The pages of commits open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CommitPages {
    /// The pages, in the order they were opened.
    pub pages: Vec<CommitPage>,
    /// The number of the next page.
    pub next: u64,
    /// The number of the last place a page was asked to bring to its top.
    pub serial: u64,
    /// The commit **Show in the timeline** waits for, while the repository is read or the history is loaded.
    pub go_to: Option<(PathBuf, CommitId)>,
}

/// How far the commit of a page is read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PageReadState {
    /// It is read.
    #[default]
    Loading,
    /// It is read.
    Read(Box<PageData>),
    /// It is not in the repository any more.
    Missing,
    /// Git could not read it.
    Failed,
}

/// The page of a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitPage {
    /// Its number.
    pub id: CommitPageId,
    /// The folder of the repository its commit is in, which it stays of.
    pub folder: PathBuf,
    /// Its commit.
    pub commit: CommitId,
    /// Its title in the trail.
    pub title: String,
    /// Whether its commit is a stash.
    pub is_stash: bool,
    /// The read of its commit, while it runs.
    pub job: Option<JobId>,
    /// Its commit as read.
    pub read: PageReadState,
    /// Whether **Show the changes** was clicked.
    pub shows_changes: bool,
    /// Which diffs are folded.
    pub folds: FoldSet,
    /// Whether **Show all** was clicked.
    pub is_all_files_shown: bool,
    /// The directories of the tree folded.
    pub folded: BTreeSet<String>,
    /// The tree of the files shown.
    pub tree: Vec<TreeNode>,
    /// What to bring to its top, once for each number.
    pub reveal: Option<(u64, PageTarget)>,
    /// A number that changes whenever what it lists changes.
    pub version: u64,
}

/// Opens the page of a commit of a repository, or shows the one it has: there is never a second page for the same
/// commit of the same repository. What `features::commit_page::open` decides, reduced in the store as Q90 says.
pub fn open(state: &mut State, folder: PathBuf, commit: CommitId) -> Vec<Effect> {
    let open = state
        .commit_pages
        .pages
        .iter()
        .find(|page| page.commit == commit && same_folder(&page.folder, &folder))
        .map(|page| page.id);
    if let Some(id) = open {
        super::pages::show_page(state, Page::Commit(id));
        return Vec::new();
    }
    let id = CommitPageId(state.commit_pages.next);
    state.commit_pages.next += 1;
    let kind = JobKind::ReadCommitPage {
        folder: folder.clone(),
        commit: commit.clone(),
    };
    let job = super::workspace::start_job(state, kind);
    let row = state.timeline.rows.iter().find(|row| row.id() == Some(&commit));
    let subject = row
        .and_then(model::graph::Row::commit)
        .map(|row| row.subject.clone())
        .unwrap_or_default();
    let is_stash = matches!(row, Some(model::graph::Row::Stash { .. }));
    state.commit_pages.pages.push(CommitPage {
        id,
        title: page_title(commit.short(), &subject),
        is_stash,
        folder,
        commit,
        job: Some(job.id),
        read: PageReadState::Loading,
        shows_changes: false,
        folds: FoldSet::default(),
        is_all_files_shown: false,
        folded: BTreeSet::new(),
        tree: Vec::new(),
        reveal: None,
        version: 0,
    });
    super::pages::show_page(state, Page::Commit(id));
    vec![Effect::Start(job)]
}

/// Takes the commit of a page read, with its lines: the diffs folded as `commit-page.initial-folds` says, the title
/// made from its subject.
pub fn page_read(
    state: &mut State,
    job: JobId,
    result: Result<Option<Box<(CommitDetail, PatchLines)>>, String>,
) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(page) = state.commit_pages.pages.iter_mut().find(|page| page.job == Some(job)) else {
        return Vec::new();
    };
    page.job = None;
    page.version += 1;
    page.read = match result {
        Ok(Some(read)) => {
            let (detail, patch) = *read;
            let subject = detail.message.lines().next().unwrap_or_default().to_owned();
            page.title = page_title(page.commit.short(), &subject);
            page.folds = match &patch {
                PatchLines::Files(patches) => initial_folds(patches),
                PatchLines::Unmatched => FoldSet::default(),
            };
            page.tree = tree_of(&detail.files, page.is_all_files_shown);
            let shaped = shape(detail);
            PageReadState::Read(Box::new(PageData {
                detail: shaped.detail,
                title: shaped.title,
                text: shaped.text,
                patch,
            }))
        }
        Ok(None) => PageReadState::Missing,
        Err(error) => {
            state.last_error = Some(error);
            PageReadState::Failed
        }
    };
    Vec::new()
}

/// Forgets the pages closed, and cancels the reads of their commits.
pub fn prune(state: &mut State) -> Vec<Effect> {
    let open = &state.pages.open;
    let (kept, closed): (Vec<CommitPage>, Vec<CommitPage>) = std::mem::take(&mut state.commit_pages.pages)
        .into_iter()
        .partition(|page| open.contains(&Page::Commit(page.id)));
    state.commit_pages.pages = kept;
    closed
        .into_iter()
        .filter_map(|page| page.job)
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Changes a page as the user asked.
pub fn reduce_page(state: &mut State, id: CommitPageId, intent: CommitPageIntent) -> Vec<Effect> {
    let serial = state.commit_pages.serial + 1;
    let Some(page) = state.commit_pages.pages.iter_mut().find(|page| page.id == id) else {
        return Vec::new();
    };
    page.version += 1;
    let count = match &page.read {
        PageReadState::Read(data) => data.detail.files.len(),
        _ => 0,
    };
    match intent {
        CommitPageIntent::Files(files) => return reduce_files(state, id, files),
        CommitPageIntent::OpenParent(parent) => {
            let folder = page.folder.clone();
            return open(state, folder, parent);
        }
        CommitPageIntent::ShowInTimeline => return show_in_timeline(state, id),
        CommitPageIntent::ShowChanges(shows) => page.shows_changes = shows,
        CommitPageIntent::Fold(index, fold) => page.folds.choose(index, fold),
        CommitPageIntent::FoldAll(fold) => page.folds.choose_all(count, fold),
        CommitPageIntent::ShowFile(index) => {
            page.shows_changes = true;
            page.folds.choose(index, Fold::Unfolded);
            page.reveal = Some((serial, PageTarget::Diff(index)));
            state.commit_pages.serial = serial;
        }
        CommitPageIntent::BackToTop => {
            page.reveal = Some((serial, PageTarget::Top));
            state.commit_pages.serial = serial;
        }
    }
    Vec::new()
}

/// Changes the files of a page as the user asked: the layout chosen is the one the next pages open with.
fn reduce_files(state: &mut State, id: CommitPageId, intent: FilesIntent) -> Vec<Effect> {
    let Some(page) = state.commit_pages.pages.iter_mut().find(|page| page.id == id) else {
        return Vec::new();
    };
    match intent {
        FilesIntent::ShowAll => {
            page.is_all_files_shown = true;
            if let PageReadState::Read(data) = &page.read {
                page.tree = tree_of(&data.detail.files, true);
            }
            Vec::new()
        }
        FilesIntent::Layout(layout) => {
            state.persisted.file_layouts.page = layout;
            vec![Effect::WritePersisted(state.persisted.clone())]
        }
        FilesIntent::Fold(path) => {
            toggle_fold(&mut page.folded, path);
            Vec::new()
        }
    }
}

/// **Show in the timeline**: the timeline in place of the page, pointed back at the repository of the page first when
/// the window shows another, and the commit selected once it is among the rows loaded.
fn show_in_timeline(state: &mut State, id: CommitPageId) -> Vec<Effect> {
    let Some(page) = state.commit_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let (folder, commit) = (page.folder.clone(), page.commit.clone());
    let mut effects = super::pages::show_timeline(state);
    let is_shown = state.shown_folder().is_some_and(|shown| same_folder(shown, &folder));
    state.commit_pages.go_to = Some((folder.clone(), commit));
    if !is_shown {
        effects.extend(super::workspace::select(state, &folder));
    }
    effects.extend(follow(state));
    effects
}

/// Follows the commit **Show in the timeline** waits for: once the timeline of its repository is read, goes to it as
/// `graph.show-in-graph` does, loading the history up to it or taking away what narrows the timeline.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some((folder, commit)) = state.commit_pages.go_to.clone() else {
        return Vec::new();
    };
    let is_read = state
        .shown_read()
        .is_some_and(|read| same_folder(&read.folder, &folder))
        && state
            .timeline
            .folder
            .as_deref()
            .is_some_and(|shown| same_folder(shown, &folder));
    if !is_read {
        if state.shown_folder().is_none_or(|shown| !same_folder(shown, &folder)) {
            state.commit_pages.go_to = None;
        }
        return Vec::new();
    }
    if state.timeline.more_job.is_some() || state.timeline.is_reading_again {
        return Vec::new();
    }
    state.commit_pages.go_to = None;
    super::search::go_to(state, commit)
}

/// What a page is drawn from: the labels and the color of the line of its commit come from the timeline while the
/// window shows its repository.
#[must_use]
pub fn page_state(state: &State, id: CommitPageId) -> Option<CommitPageState<'_>> {
    let page = state.commit_pages.pages.iter().find(|page| page.id == id)?;
    let timeline = &state.timeline;
    let is_shown = timeline
        .folder
        .as_deref()
        .is_some_and(|shown| same_folder(shown, &page.folder));
    let layout = &timeline.layout;
    let (labels, lane) = if is_shown {
        let labels = layout.labels.get(&page.commit).map_or(&[][..], Vec::as_slice);
        let lane = layout
            .shown_of
            .get(&page.commit)
            .and_then(|shown| layout.cells.get(*shown))
            .map_or(0, |cell| cell.color);
        (labels, lane)
    } else {
        (&[][..], 0)
    };
    Some(CommitPageState {
        id,
        title: &page.title,
        is_stash: page.is_stash,
        read: match &page.read {
            PageReadState::Loading => PageRead::Loading,
            PageReadState::Read(data) => PageRead::Read(data),
            PageReadState::Missing => PageRead::Missing,
            PageReadState::Failed => PageRead::Failed,
        },
        shows_changes: page.shows_changes,
        folds: &page.folds,
        layout: state.persisted.file_layouts.page,
        tree: &page.tree,
        folded: &page.folded,
        is_all_files_shown: page.is_all_files_shown,
        labels,
        lane,
        reveal: page.reveal,
        version: page.version,
    })
}
