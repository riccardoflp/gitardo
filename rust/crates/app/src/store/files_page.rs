//! The Files page as the store keeps it: the files of the working tree of the repository shown, listed on a worker when
//! the page opens and again each time the repository is read while it is open, as a tree whose folders are folded
//! until they are opened; and what is done with a file or a folder picked in it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use model::contract::{FeatureId, Target};
use model::details::{FilePath, TreeNode, file_tree};
use views::feature::FeatureIntent;
use views::file_view::{FilePageId, FileTarget, FileViewIntent};
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
    /// The File page of the file shown beside the tree (`file-view.beside-files`).
    pub shown_file: Option<FilePageId>,
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
        FilesPageIntent::OpenFile(path) => open_beside(state, &path),
        FilesPageIntent::ShowTree => {
            leave_file(state);
            super::pages::show_page(state, Page::Files);
            Vec::new()
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
        leave_file(state);
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
        marked: page
            .shown_file
            .and_then(|id| state.file_pages.pages.iter().find(|file| file.id == id))
            .map(|file| file.path.as_str()),
    }
}

/// Opens a file of the working tree beside the tree of the Files page, in place of the one shown there: that one is
/// forgotten, unless it has edits not saved, which keep it open in its own page (`file-view.beside-files`). A file
/// that has its own page already keeps it too.
fn open_beside(state: &mut State, path: &str) -> Vec<Effect> {
    let listed = state.pages.open.clone();
    let effects = super::file_page::reduce(state, FileViewIntent::Open(FileTarget::working_tree(path)));
    let Some(Page::File(id)) = state.page_shown() else {
        return effects;
    };
    if !listed.contains(&Page::File(id)) {
        state.pages.open.retain(|page| *page != Page::File(id));
    }
    super::pages::show_page(state, Page::Files);
    if let Some(previous) = state
        .files_page
        .shown_file
        .replace(id)
        .filter(|previous| *previous != id)
    {
        keep_if_unsaved(state, previous);
    }
    effects
}

/// Takes the file away from beside the tree, keeping it open in its own page when it has edits not saved.
fn leave_file(state: &mut State) {
    if let Some(id) = state.files_page.shown_file.take() {
        keep_if_unsaved(state, id);
    }
}

/// Lists a File page among the pages open, after the others, when it has edits not saved and is not listed yet.
fn keep_if_unsaved(state: &mut State, id: FilePageId) {
    let is_unsaved = state
        .file_pages
        .pages
        .iter()
        .any(|page| page.id == id && page.is_unsaved());
    if is_unsaved && !state.pages.open.contains(&Page::File(id)) {
        state.pages.open.push(Page::File(id));
    }
}

/// The Files page with the file shown beside its tree, when one is, as the window draws it.
#[must_use]
pub fn beside_state<'a>(
    state: &'a State,
    file: Option<views::file_view::FilePageState<'a>>,
    tree_width: Option<f32>,
) -> views::page::PageState<'a> {
    let tree_width = tree_width.map(whole_pixels);
    let tree = page_state(state);
    match file {
        Some(file) => {
            views::page::PageState::FilesBeside(Box::new(views::file_view::BesideFilesState { tree, file, tree_width }))
        }
        None => views::page::PageState::Files(tree),
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

/// A width in whole logical pixels.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the width is kept between zero and the most a u16 counts first"
)]
pub(crate) fn whole_pixels(width: f32) -> u16 {
    width.round().clamp(0.0, f32::from(u16::MAX)) as u16
}
