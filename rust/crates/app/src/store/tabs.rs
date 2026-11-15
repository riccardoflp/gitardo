//! The tabs of the repositories open in the window (`repo.tabs`, `repo.switch-tab`, `repo.keep-open`): one for each
//! repository of the workspace, the one shown, and what each of the others keeps while it is not.
//!
//! The store holds what the repository shown shows in the fields of [`State`]; a tab that is left takes those fields
//! with it, as a [`RepositoryView`], and gives them back when it is shown again. What its reads answer meanwhile is
//! kept with it, in the order it came, and told to the store once it is shown.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use views::graph::RepositoryEntry;
use views::repo::TabEntry;

use super::message::{Effect, JobId, JobKind, Message};
use super::state::{Shown, State};
use crate::persisted::{RecentWorkspace, same_folder};

/// The tabs of the window.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Tabs {
    /// The tabs, in their order.
    pub open: Vec<Tab>,
    /// The tab shown; none while the window shows no repository.
    pub shown: Option<usize>,
}

/// A repository open in a tab.
#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// What it showed when it was left, with `repositories.keepOpen` on; none for the tab shown, for one never shown,
    /// and with the setting off.
    pub kept: Option<Box<RepositoryView>>,
    /// What its reads answered while it was not shown, in the order it came.
    pub waiting: Vec<Message>,
}

impl Tab {
    /// A tab for a repository, which keeps nothing yet.
    #[must_use]
    pub fn new(folder: PathBuf) -> Tab {
        Tab {
            folder,
            kept: None,
            waiting: Vec::new(),
        }
    }
}

/// What the store holds of the repository shown, which a tab keeps while another is shown.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RepositoryView {
    /// The repository as last read; none while it was being read for the first time.
    pub shown: Option<Shown>,
    /// The jobs it started that had not answered, which it waits for.
    pub jobs: BTreeMap<JobId, JobKind>,
    /// The timeline.
    pub timeline: super::timeline::Timeline,
    /// The search and Quick search.
    pub search: super::search::Search,
    /// The inspector.
    pub inspector: super::inspector::Inspector,
    /// The pages open, and the one shown.
    pub pages: super::state::Pages,
    /// The pages of commits.
    pub commit_pages: super::commit_page::CommitPages,
    /// The Diff pages.
    pub diff_pages: super::diff_page::DiffPages,
    /// The Merge pages.
    pub merge_pages: super::merge_page::MergePages,
    /// The File pages.
    pub file_pages: super::file_page::FilePages,
    /// The comparisons.
    pub compares: super::compare::Compares,
    /// The review of the conflicts.
    pub reviews: super::review::Reviews,
    /// Its reads.
    pub reads: super::reads::Reads,
    /// The Changes page and the commit box.
    pub changes: super::changes::Changes,
    /// The interactive rebase page.
    pub rebase_page: Option<views::rebase::TodoPage>,
    /// The conflicts the rules resolve.
    pub conflicts: super::conflicts::Conflicts,
    /// The Files page.
    pub files_page: super::files_page::FilesPage,
    /// The page of what new worktrees get.
    pub new_worktrees: super::new_worktrees::NewWorktreesPage,
    /// The blame of the File pages.
    pub blames: super::blame::Blames,
    /// Another worktree shown in its place.
    pub worktrees: super::worktrees::Worktrees,
    /// The pull requests.
    pub github: super::github::GitHub,
    /// The branch rail.
    pub rail: super::rail::Rail,
}

/// The tabs for the repositories found in a workspace, in their order: a tab of a repository still there keeps what
/// it kept; the tab shown is the one of the repository shown. What the tabs of the repositories gone waited for is
/// cancelled.
pub fn follow_repositories(state: &mut State) -> Vec<Effect> {
    let folders: Vec<PathBuf> = state.repositories().iter().map(|found| found.folder.clone()).collect();
    let mut before = std::mem::take(&mut state.tabs.open);
    state.tabs.open = folders
        .into_iter()
        .map(|folder| {
            before
                .iter()
                .position(|tab| same_folder(&tab.folder, &folder))
                .map_or_else(|| Tab::new(folder), |index| before.swap_remove(index))
        })
        .collect();
    state.tabs.shown = shown_tab(state);
    before.into_iter().flat_map(forget).collect()
}

/// Marks the tab of the repository shown as the one shown, after a repository was chosen without the tabs.
pub(super) fn follow_shown(state: &mut State) {
    if let Some(index) = shown_tab(state) {
        state.tabs.shown = Some(index);
    }
}

/// The tabs as the island draws them: each by the name of its repository among those of the workspace, and with the
/// folder of what it shows for its tooltip, a worktree shown in place of its repository included.
pub(super) fn entries(state: &State, repositories: &[RepositoryEntry]) -> Vec<TabEntry> {
    let shown = state.shown_folder();
    state
        .tabs
        .open
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let is_shown = state.tabs.shown == Some(index);
            let name = repositories
                .iter()
                .find(|entry| same_folder(&entry.folder, &tab.folder))
                .map_or_else(|| tab.folder.display().to_string(), |entry| entry.name.clone());
            let folder = match shown {
                Some(shown) if is_shown => shown.display().to_string(),
                _ => tab.folder.display().to_string(),
            };
            TabEntry { name, folder, is_shown }
        })
        .collect()
}

/// The tab of the repository shown, or of the repository whose worktree is shown in its place.
fn shown_tab(state: &State) -> Option<usize> {
    let folder = state.worktrees.window_folder.as_ref().or(state.shown_folder())?;
    state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder))
}

/// Shows the repository of another tab (`repo.switch-tab`): nothing while an operation runs, or when it is the tab
/// shown. With `repositories.keepOpen` on, the tab left keeps what it shows, and a tab that kept something shows it at
/// once, read again behind it; otherwise the repository is read as when it is chosen (`repo.select`).
///
/// Feature: repo.switch-tab
pub fn switch_tab(state: &mut State, index: usize) -> Vec<Effect> {
    if state.is_operation_running || state.tabs.shown == Some(index) {
        return Vec::new();
    }
    let Some(folder) = state.tabs.open.get(index).map(|tab| tab.folder.clone()) else {
        return Vec::new();
    };
    let mut effects = super::workspace::cancel_reads(state);
    let keep_open = state.settings.keep_open;
    let (left, not_wanted) = kept_on_leaving(take_view(state), keep_open);
    effects.extend(not_wanted.into_iter().map(Effect::Cancel));
    if let Some(tab) = state.tabs.shown.and_then(|shown| state.tabs.open.get_mut(shown)) {
        tab.kept = left.map(Box::new);
    }
    state.dialog = None;
    let Some(tab) = state.tabs.open.get_mut(index) else {
        return effects;
    };
    let kept = tab.kept.take();
    let waiting = std::mem::take(&mut tab.waiting);
    state.tabs.shown = Some(index);
    let Some(kept) = kept else {
        effects.extend(super::workspace::select(state, &folder));
        return effects;
    };
    put_view(state, *kept);
    if let Some(workspace) = state.workspace.as_ref() {
        state.persisted.remember_workspace(RecentWorkspace {
            folders: workspace.folders.clone(),
            repository: Some(folder.clone()),
        });
        effects.push(Effect::WritePersisted(state.persisted.clone()));
    }
    effects.push(super::workspace::watch_settings(state));
    let read_folder = state.shown_folder().cloned().unwrap_or(folder);
    effects.push(super::workspace::start_read(state, read_folder));
    for message in waiting {
        effects.extend(super::reduce_message(state, message));
    }
    effects
}

/// What a tab keeps when it is left (`repo.keep-open`), and the jobs it started that nobody waits for any more: with
/// `repositories.keepOpen` on, all it shows, still waiting for its jobs; with it off, nothing but what is kept between
/// runs anyway, which the store keeps by repository itself, and none of its jobs.
///
/// Feature: repo.keep-open
#[must_use]
pub fn kept_on_leaving(view: RepositoryView, keep_open: bool) -> (Option<RepositoryView>, Vec<JobId>) {
    if keep_open {
        (Some(view), Vec::new())
    } else {
        (None, view.jobs.into_keys().collect())
    }
}

/// Shows the tab of a repository of the workspace: chosen in the picker of `repo.select`, or the one a file is in,
/// shown before the file's history or blame (`repo.owner-of-file`).
pub fn show_repository(state: &mut State, folder: &Path) -> Vec<Effect> {
    let Some(index) = state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder)) else {
        return super::workspace::select(state, folder);
    };
    // The tab shown, showing a worktree or a submodule in its place, comes back to its own repository.
    if state.tabs.shown == Some(index) {
        return super::workspace::select(state, folder);
    }
    switch_tab(state, index)
}

/// The tab that waits for what a message answers, when it is not the tab shown.
#[must_use]
pub fn waiting_for(state: &State, message: &Message) -> Option<usize> {
    let answered = Answered::of(message)?;
    state
        .tabs
        .open
        .iter()
        .position(|tab| tab.kept.as_ref().is_some_and(|view| answered.is_for(view)))
}

/// Keeps a message for the tab that waits for it, until it is shown.
pub fn keep_for(state: &mut State, index: usize, message: Message) {
    if let Some(tab) = state.tabs.open.get_mut(index) {
        tab.waiting.push(message);
    }
}

/// With `repositories.keepOpen` turned off, the tabs not shown forget what they kept, and what they waited for is
/// cancelled.
pub fn settings_changed(state: &mut State) -> Vec<Effect> {
    if state.settings.keep_open {
        return Vec::new();
    }
    let mut effects = Vec::new();
    for tab in &mut state.tabs.open {
        tab.waiting.clear();
        if let Some(kept) = tab.kept.take() {
            effects.extend(kept.jobs.into_keys().map(Effect::Cancel));
        }
    }
    effects
}

/// Every job the tabs not shown wait for, which closing the window cancels.
pub fn cancel_all(state: &mut State) -> Vec<Effect> {
    std::mem::take(&mut state.tabs.open)
        .into_iter()
        .flat_map(forget)
        .collect()
}

/// What a tab closed waited for, cancelled.
fn forget(tab: Tab) -> Vec<Effect> {
    tab.kept
        .map(|kept| kept.jobs.into_keys().map(Effect::Cancel).collect())
        .unwrap_or_default()
}

/// What a message is the answer to: a job, or a question to the AI provider.
enum Answered {
    Job(JobId),
    Ask(crate::work::ai::AskId),
}

impl Answered {
    /// What a message answers; none for a message that answers nothing a repository asked.
    fn of(message: &Message) -> Option<Answered> {
        let job = match message {
            Message::RepositoryRead { job, .. }
            | Message::TimelineRead { job, .. }
            | Message::CommitsRead { job, .. }
            | Message::Searched { job, .. }
            | Message::DetailsRead { job, .. }
            | Message::CommitPageRead { job, .. }
            | Message::DiffRead { job, .. }
            | Message::MergeRead { job, .. }
            | Message::FileRead { job, .. }
            | Message::ComparisonRead { job, .. }
            | Message::InPlaceRead { job, .. }
            | Message::ConflictsCounted { job, .. }
            | Message::ReviewRead { job, .. }
            | Message::IgnoredRead { job, .. }
            | Message::FilesRead { job, .. }
            | Message::NewWorktreesRead { job, .. }
            | Message::BlameRead { job, .. }
            | Message::GitHub(
                super::github::GitHubMessage::Detected { job, .. } | super::github::GitHubMessage::Read { job, .. },
            ) => *job,
            Message::AiPiece { ask, .. } | Message::AiAnswered { ask, .. } => return Some(Answered::Ask(*ask)),
            _ => return None,
        };
        Some(Answered::Job(job))
    }

    /// Whether a view kept by a tab waits for it.
    fn is_for(&self, view: &RepositoryView) -> bool {
        match self {
            Answered::Job(job) => {
                view.jobs.contains_key(job)
                    || view.rail.stashes.jobs.contains_key(job)
                    || view.github.detecting == Some(*job)
                    || view.github.reading.as_ref().is_some_and(|(reading, _)| reading == job)
            }
            Answered::Ask(ask) => {
                view.reviews.questions.asked.contains_key(ask)
                    || view
                        .changes
                        .drafting
                        .as_ref()
                        .is_some_and(|drafting| drafting.ask == *ask)
            }
        }
    }
}

/// Takes out of the store what the repository shown shows, leaving what a repository not read yet shows. What is the
/// window's stays: which folder is watched, how the lists of the rail are turned, and the number of the next question
/// to the AI provider.
fn take_view(state: &mut State) -> RepositoryView {
    let shown = state
        .workspace
        .as_mut()
        .map(|workspace| std::mem::replace(&mut workspace.shown, Shown::Looking));
    let opening = state.opening.as_ref().map(|opening| opening.job);
    let jobs: Vec<JobId> = state.jobs.keys().copied().filter(|job| Some(*job) != opening).collect();
    let jobs = jobs
        .into_iter()
        .filter_map(|job| state.jobs.remove(&job).map(|kind| (job, kind)))
        .collect();
    let view = RepositoryView {
        shown,
        jobs,
        timeline: std::mem::take(&mut state.timeline),
        search: std::mem::take(&mut state.search),
        inspector: std::mem::take(&mut state.inspector),
        pages: std::mem::take(&mut state.pages),
        commit_pages: std::mem::take(&mut state.commit_pages),
        diff_pages: std::mem::take(&mut state.diff_pages),
        merge_pages: std::mem::take(&mut state.merge_pages),
        file_pages: std::mem::take(&mut state.file_pages),
        compares: std::mem::take(&mut state.compares),
        reviews: std::mem::take(&mut state.reviews),
        reads: std::mem::take(&mut state.reads),
        changes: std::mem::take(&mut state.changes),
        rebase_page: state.rebase_page.take(),
        conflicts: std::mem::take(&mut state.conflicts),
        files_page: std::mem::take(&mut state.files_page),
        new_worktrees: std::mem::take(&mut state.new_worktrees),
        blames: std::mem::take(&mut state.blames),
        worktrees: std::mem::take(&mut state.worktrees),
        github: std::mem::take(&mut state.github),
        rail: std::mem::take(&mut state.rail),
    };
    state.reads.watched.clone_from(&view.reads.watched);
    state.rail.lists = view.rail.lists.clone();
    state.reviews.questions.next = view.reviews.questions.next;
    view
}

/// Puts back in the store what a tab kept, keeping what is the window's.
fn put_view(state: &mut State, view: RepositoryView) {
    let watched = state.reads.watched.take();
    let lists = std::mem::take(&mut state.rail.lists);
    let next_ask = state.reviews.questions.next;
    if let (Some(workspace), Some(shown)) = (state.workspace.as_mut(), view.shown) {
        workspace.shown = shown;
    }
    state.jobs.extend(view.jobs);
    state.timeline = view.timeline;
    state.search = view.search;
    state.inspector = view.inspector;
    state.pages = view.pages;
    state.commit_pages = view.commit_pages;
    state.diff_pages = view.diff_pages;
    state.merge_pages = view.merge_pages;
    state.file_pages = view.file_pages;
    state.compares = view.compares;
    state.reviews = view.reviews;
    state.reads = view.reads;
    state.changes = view.changes;
    state.rebase_page = view.rebase_page;
    state.conflicts = view.conflicts;
    state.files_page = view.files_page;
    state.new_worktrees = view.new_worktrees;
    state.blames = view.blames;
    state.worktrees = view.worktrees;
    state.github = view.github;
    state.rail = view.rail;
    state.reads.watched = watched;
    state.rail.lists = lists;
    state.reviews.questions.next = state.reviews.questions.next.max(next_ask);
}
