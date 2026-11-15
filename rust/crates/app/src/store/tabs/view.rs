//! What a tab keeps of the repository it shows while another tab is shown (`repo.keep-open`), and which answers of the
//! workers are its own.

use std::collections::BTreeMap;

use super::super::message::{JobId, JobKind, Message};
use super::super::state::{Shown, State};

/// What the store holds of the repository shown, which a tab keeps while another is shown.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RepositoryView {
    /// The repository as last read; none while it was being read for the first time.
    pub shown: Option<Shown>,
    /// The jobs it started that had not answered, which it waits for.
    pub jobs: BTreeMap<JobId, JobKind>,
    /// The timeline.
    pub timeline: super::super::timeline::Timeline,
    /// The search and Quick search.
    pub search: super::super::search::Search,
    /// The inspector.
    pub inspector: super::super::inspector::Inspector,
    /// The pages open, and the one shown.
    pub pages: super::super::state::Pages,
    /// The pages of commits.
    pub commit_pages: super::super::commit_page::CommitPages,
    /// The Diff pages.
    pub diff_pages: super::super::diff_page::DiffPages,
    /// The Merge pages.
    pub merge_pages: super::super::merge_page::MergePages,
    /// The File pages.
    pub file_pages: super::super::file_page::FilePages,
    /// The comparisons.
    pub compares: super::super::compare::Compares,
    /// The review of the conflicts.
    pub reviews: super::super::review::Reviews,
    /// Its reads.
    pub reads: super::super::reads::Reads,
    /// The Changes page and the commit box.
    pub changes: super::super::changes::Changes,
    /// The interactive rebase page.
    pub rebase_page: Option<views::rebase::TodoPage>,
    /// The conflicts the rules resolve.
    pub conflicts: super::super::conflicts::Conflicts,
    /// The Files page.
    pub files_page: super::super::files_page::FilesPage,
    /// The page of what new worktrees get.
    pub new_worktrees: super::super::new_worktrees::NewWorktreesPage,
    /// The blame of the File pages.
    pub blames: super::super::blame::Blames,
    /// Another worktree shown in its place.
    pub worktrees: super::super::worktrees::Worktrees,
    /// The pull requests.
    pub github: super::super::github::GitHub,
    /// The branch rail.
    pub rail: super::super::rail::Rail,
}

impl RepositoryView {
    /// Whether a page of it has edits not saved: a File page, or the result of a Merge page.
    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.file_pages
            .pages
            .iter()
            .any(super::super::file_page::FilePage::is_unsaved)
            || self
                .merge_pages
                .pages
                .iter()
                .any(super::super::merge_page::MergePage::is_unsaved)
    }
}

/// What a message is the answer to: a job, or a question to the AI provider.
pub(super) enum Answered {
    Job(JobId),
    Ask(crate::work::ai::AskId),
}

impl Answered {
    /// What a message answers; none for a message that answers nothing a repository asked.
    pub(super) fn of(message: &Message) -> Option<Answered> {
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
                super::super::github::GitHubMessage::Detected { job, .. }
                | super::super::github::GitHubMessage::Read { job, .. },
            ) => *job,
            Message::AiPiece { ask, .. } | Message::AiAnswered { ask, .. } => return Some(Answered::Ask(*ask)),
            _ => return None,
        };
        Some(Answered::Job(job))
    }

    /// Whether a view kept by a tab waits for it.
    pub(super) fn is_for(&self, view: &RepositoryView) -> bool {
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
pub(super) fn take_view(state: &mut State) -> RepositoryView {
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
pub(super) fn put_view(state: &mut State, view: RepositoryView) {
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
