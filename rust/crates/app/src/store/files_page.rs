//! The Files page as the store keeps it: the files of the working tree of the repository shown, listed on a worker when
//! the page opens and again each time the repository is read while it is open, as a tree whose folders are folded
//! until they are opened; and what is done with a file or a folder picked in it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use model::contract::{FeatureId, Target};
use model::details::{FilePath, TreeNode, file_tree};
use views::feature::FeatureIntent;
use views::file_view::{FileTarget, FileViewIntent};
use views::files::{FilesPageIntent, FilesPageState, FilesShown};
use views::page::Page;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;

/// The Files page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FilesPage {
    /// The top folder of the working tree the files are of.
    pub folder: Option<PathBuf>,
    /// The read of the repository they were listed after.
    pub generation: Option<u64>,
    /// The tree of the files listed; none before the first list.
    pub tree: Option<Vec<TreeNode>>,
    /// What git said when it could not list them.
    pub failed: Option<String>,
    /// The folders opened, by their paths.
    pub opened: BTreeSet<String>,
    /// The list running.
    pub job: Option<JobId>,
}

/// Something asked of the Files page.
pub fn reduce(state: &mut State, intent: FilesPageIntent) -> Vec<Effect> {
    match intent {
        FilesPageIntent::Open => {
            super::pages::show_page(state, Page::Files);
            state.files_page.generation = None;
            follow(state)
        }
        FilesPageIntent::Fold(path) => {
            let opened = &mut state.files_page.opened;
            if !opened.remove(&path) {
                let _new = opened.insert(path);
            }
            Vec::new()
        }
        FilesPageIntent::OpenFile(path) => {
            super::file_page::reduce(state, FileViewIntent::Open(FileTarget::working_tree(&path)))
        }
        FilesPageIntent::Run(feature, path) => run(state, feature, &path),
    }
}

/// Runs an operation on a file or a folder of the working tree, which it is given as a path on disk.
fn run(state: &mut State, feature: FeatureId, path: &str) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let mut on_disk = read.state.folder.clone();
    on_disk.extend(path.split('/').filter(|name| !name.is_empty()));
    let run = FeatureIntent::Run(feature, Target::Path(on_disk));
    super::feature::reduce(state, run)
}

/// Lists the files again while the page is open and the repository shown was read since they were listed, or is
/// another one; the folders opened stay opened in the same working tree.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if !state.pages.open.contains(&Page::Files) {
        return cancel(state);
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.state.folder.clone(), read.generation);
    let page = &mut state.files_page;
    let is_same_folder = page.folder.as_deref().is_some_and(|known| same_folder(known, &folder));
    if is_same_folder && (page.generation == Some(generation) || page.job.is_some()) {
        return Vec::new();
    }
    if !is_same_folder {
        page.tree = None;
        page.opened.clear();
    }
    page.folder = Some(folder.clone());
    page.generation = Some(generation);
    let job = super::workspace::start_job(state, JobKind::ReadFiles { folder });
    state.files_page.job = Some(job.id);
    vec![Effect::Start(job)]
}

/// Cancels the list running once the page is closed.
fn cancel(state: &mut State) -> Vec<Effect> {
    let Some(job) = state.files_page.job.take() else {
        return Vec::new();
    };
    state.files_page.generation = None;
    let _cancelled = state.jobs.remove(&job);
    vec![Effect::Cancel(job)]
}

/// Takes what a read of the Files page, or of the page of new worktrees, gives.
pub(super) fn read(state: &mut State, message: super::message::Message) -> Vec<Effect> {
    match message {
        super::message::Message::FilesRead { job, result } => files_read(state, job, result),
        super::message::Message::NewWorktreesRead { job, result } => {
            super::new_worktrees::setup_read(state, job, *result)
        }
        _ => Vec::new(),
    }
}

/// Takes the files listed.
pub fn files_read(state: &mut State, job: JobId, result: Result<Vec<String>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let page = &mut state.files_page;
    if page.job != Some(job) {
        return Vec::new();
    }
    page.job = None;
    match result {
        Ok(paths) => {
            let paths: Vec<FilePath> = paths.iter().map(|path| FilePath::from(path.as_str())).collect();
            page.tree = Some(file_tree(&paths));
            page.failed = None;
        }
        Err(error) => page.failed = Some(error),
    }
    Vec::new()
}

/// What the page is drawn from.
#[must_use]
pub fn page_state(state: &State) -> FilesPageState<'_> {
    let page = &state.files_page;
    let name = page
        .folder
        .as_deref()
        .and_then(|folder| folder.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let shown = match (&page.tree, &page.failed) {
        (_, Some(error)) => FilesShown::Failed(error),
        (Some(tree), None) => FilesShown::Listed(tree),
        (None, None) => FilesShown::Reading,
    };
    FilesPageState {
        name,
        shown,
        opened: &page.opened,
    }
}

/// An intent of the File pages, of the Files page or of the blame.
pub(super) fn reduce_file_intent(state: &mut State, intent: views::Intent) -> Vec<Effect> {
    match intent {
        views::Intent::FileView(intent) => super::file_page::reduce(state, intent),
        views::Intent::Files(intent) => reduce(state, intent),
        views::Intent::Blame(intent) => super::blame::reduce(state, &intent),
        _ => Vec::new(),
    }
}
