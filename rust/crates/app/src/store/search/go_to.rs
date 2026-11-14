//! Going to a commit, as `graph.show-in-graph` does, for a match stepped to (`search.step`), the first match of a
//! search of the whole history (`search.history`), a hash typed (`search.go-to-hash`) and **Go to commit…**
//! (`search.go-to-commit`): a commit loaded is selected, and the merge folded over it unfolded; one further back is
//! looked for in the history, which is loaded up to it; one the timeline does not list takes away what narrows it, and
//! one on no branch or tag is shown with its history alone.

use model::CommitId;
use model::search::hash_prefix;
use views::graph::GraphIntent;
use views::search::{GoToCommitPurpose, SearchIntent};

use super::super::commit_page;
use super::super::message::{Effect, JobId};
use super::super::pages;
use super::super::state::State;
use super::super::timeline::{load_up_to, reduce_graph, select_in_middle, show as show_branches};
use super::{GoToCommit, cancel, jobs, list};

/// A commit being gone to, and how far it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoTo {
    /// The commit.
    pub commit: CommitId,
    /// How far it is.
    pub stage: Stage,
}

/// How far going to a commit that is not loaded is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Where it is in the history is read, by this job.
    Position(JobId),
    /// The history is loaded up to it.
    Pages,
    /// The timeline is read again with what narrowed it taken away, or with the history of the commit alone.
    AfterList,
}

/// Goes to a commit: selects it when it is loaded, unfolding the merge folded over it, or loads the history up to it,
/// taking away what narrows the timeline when that leaves it out; a commit no branch or tag reaches is shown with its
/// history alone. Every way to a commit comes here: **Show in the timeline** and a click in the branch rail, a step of
/// a search, **Go to commit…**, the reflog, the page of a commit and the bisect banner.
///
/// Feature: graph.show-in-graph
pub fn go_to(state: &mut State, commit: CommitId) -> Vec<Effect> {
    let mut effects = forget(state);
    if let Some(shown) = show(state, &commit) {
        effects.extend(shown);
        return effects;
    }
    if state.timeline.has_more {
        if let Some(job) = jobs::position_request(state, &commit) {
            let job = jobs::start(state, job);
            state.search.going_to = Some(GoTo {
                commit,
                stage: Stage::Position(job.id),
            });
            effects.push(Effect::Start(job));
        }
        return effects;
    }
    effects.extend(widen(state, commit));
    effects
}

/// Forgets the commit being gone to, another one taking its place.
fn forget(state: &mut State) -> Vec<Effect> {
    match state.search.going_to.take() {
        Some(GoTo {
            stage: Stage::Position(job),
            ..
        }) => cancel(state, [job]),
        _ => Vec::new(),
    }
}

/// Selects a commit loaded, unfolding the merge folded over it; none when it is not loaded.
fn show(state: &mut State, commit: &CommitId) -> Option<Vec<Effect>> {
    let layout = &state.timeline.layout;
    let merge = layout.folded.folded_by.get(commit).cloned();
    if merge.is_none() && !layout.shown_of.contains_key(commit) {
        return None;
    }
    let mut effects = Vec::new();
    if let Some(merge) = merge {
        effects.extend(reduce_graph(state, GraphIntent::Unfold(merge)));
    }
    select_in_middle(state, commit.clone());
    Some(effects)
}

/// A commit that is not in the history listed: what narrows the timeline is taken away, one thing at a time, and the
/// commit gone to again once the timeline is read: a search shown as a list or the history of another commit alone,
/// then the branches shown; a commit still not listed, on no branch or tag, is shown with its history alone.
fn widen(state: &mut State, commit: CommitId) -> Vec<Effect> {
    if state.search.alone.as_ref() == Some(&commit) {
        return Vec::new();
    }
    let effects = if super::is_narrowed(state) {
        list::show_all(state)
    } else if state.timeline.shown.is_some() {
        show_branches(state, None)
    } else {
        list::alone(state, commit.clone())
    };
    if !effects.is_empty() {
        state.search.going_to = Some(GoTo {
            commit,
            stage: Stage::AfterList,
        });
    }
    effects
}

/// Where the commit being gone to is in the history: the pages up to it are loaded.
pub fn position_read(state: &mut State, job: JobId, result: Result<Option<usize>, String>) -> Vec<Effect> {
    let Some(going) = state.search.going_to.take() else {
        return Vec::new();
    };
    if going.stage != Stage::Position(job) {
        state.search.going_to = Some(going);
        return Vec::new();
    }
    match result {
        Ok(Some(position)) => {
            let pages = position / state.settings.page_size.get() + 1;
            let effects = load_up_to(state, pages);
            if !effects.is_empty() {
                state.search.going_to = Some(GoTo {
                    commit: going.commit,
                    stage: Stage::Pages,
                });
            }
            effects
        }
        Ok(None) => widen(state, going.commit),
        Err(error) => {
            state.last_error = Some(error);
            Vec::new()
        }
    }
}

/// The timeline was read: the commit being gone to is selected once it is loaded.
pub fn settle(state: &mut State) -> Vec<Effect> {
    let Some(going) = state.search.going_to.take() else {
        return Vec::new();
    };
    match going.stage {
        Stage::Position(_) => {
            state.search.going_to = Some(going);
            Vec::new()
        }
        _ if state.timeline.is_reading_again || state.timeline.more_job.is_some() => {
            state.search.going_to = Some(going);
            Vec::new()
        }
        Stage::Pages => show(state, &going.commit).unwrap_or_default(),
        Stage::AfterList => show(state, &going.commit).unwrap_or_else(|| go_to(state, going.commit)),
    }
}

/// `Enter` on a hash no loaded commit has: git is asked for the commit it starts, in lower case.
pub fn hash(state: &mut State) -> Vec<Effect> {
    let Some(prefix) = hash_prefix(&state.search.text) else {
        return Vec::new();
    };
    let before = state.search.hash_job.take();
    let mut effects = cancel(state, before);
    state.search.failure = None;
    if let Some(job) = jobs::revision_request(state, &prefix) {
        let job = jobs::start(state, job);
        state.search.hash_job = Some(job.id);
        effects.push(Effect::Start(job));
    }
    effects
}

/// The commit a hash or a name stands for: the hash typed in the box is gone to, or refused; what is typed in **Go to
/// commit** is known or not.
pub fn revision_read(state: &mut State, job: JobId, result: Result<Option<CommitId>, String>) -> Vec<Effect> {
    if state.search.hash_job == Some(job) {
        state.search.hash_job = None;
        return match result {
            Ok(Some(commit)) => go_to(state, commit),
            Ok(None) => {
                state.search.failure = Some(views::text::search::not_a_commit(state.search.text.trim()));
                Vec::new()
            }
            Err(error) => {
                state.search.failure = Some(views::text::search::search_failed(&error));
                Vec::new()
            }
        };
    }
    if let Some(dialog) = state.search.dialog.as_mut().filter(|dialog| dialog.job == Some(job)) {
        dialog.job = None;
        dialog.verdict = Some(result.unwrap_or_default());
    }
    Vec::new()
}

/// **Go to commit…** (`search.go-to-commit`): what is typed is asked of git as it is typed; **Go to commit** goes to
/// the commit once git knows it, and does nothing before. **Open Commit as a Page…** (`commit-page.open`) asks the
/// same way, and opens the page of the commit.
pub fn dialog(state: &mut State, intent: SearchIntent) -> Vec<Effect> {
    match intent {
        SearchIntent::GoToCommit | SearchIntent::OpenCommit => {
            if state.shown_read().is_some() {
                let purpose = if intent == SearchIntent::OpenCommit {
                    GoToCommitPurpose::OpenPage
                } else {
                    GoToCommitPurpose::GoTo
                };
                state.search.dialog = Some(GoToCommit {
                    purpose,
                    ..GoToCommit::default()
                });
            }
            Vec::new()
        }
        SearchIntent::GoToCommitTyped(text) => typed(state, text),
        SearchIntent::GoToCommitSubmit => {
            let Some(dialog) = state.search.dialog.as_ref() else {
                return Vec::new();
            };
            let (Some(Some(commit)), purpose) = (dialog.verdict.clone(), dialog.purpose) else {
                return Vec::new();
            };
            state.search.dialog = None;
            match purpose {
                GoToCommitPurpose::GoTo => {
                    let mut effects = pages::show_timeline(state);
                    effects.extend(go_to(state, commit));
                    effects
                }
                GoToCommitPurpose::OpenPage => match state.shown_read().map(|read| read.folder.clone()) {
                    Some(folder) => commit_page::open(state, folder, commit),
                    None => Vec::new(),
                },
            }
        }
        _ => {
            let job = state.search.dialog.take().and_then(|dialog| dialog.job);
            cancel(state, job)
        }
    }
}

/// What is typed in **Go to commit**: the read of what was typed before is cancelled and the new text asked of git.
fn typed(state: &mut State, text: String) -> Vec<Effect> {
    let Some(dialog) = state.search.dialog.as_mut() else {
        return Vec::new();
    };
    if dialog.text == text {
        return Vec::new();
    }
    let before = dialog.job.take();
    dialog.verdict = None;
    let revision = text.trim().to_owned();
    dialog.text = text;
    let mut effects = cancel(state, before);
    if revision.is_empty() {
        return effects;
    }
    if let Some(job) = jobs::revision_request(state, &revision) {
        let job = jobs::start(state, job);
        if let Some(dialog) = state.search.dialog.as_mut() {
            dialog.job = Some(job.id);
        }
        effects.push(Effect::Start(job));
    }
    effects
}
