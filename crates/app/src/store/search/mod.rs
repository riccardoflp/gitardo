//! The search of the timeline as the store keeps it (`search.box`, `search.loaded`, `search.history`, `search.step`,
//! `search.list-mode`, `search.go-to-hash`), **Go to commit…** (`search.go-to-commit`) and Quick search.
//!
//! A search of the commits loaded is worked out at once, here, each time the text or the rows change. A search of the
//! whole history, the commit of a hash or a name, and where a commit is in the history are read on a worker
//! ([`jobs`]); each read has its job, and a result whose job is no longer the one waited for is not wanted, as the
//! reads of the timeline are not. A new search cancels the read of the one before.

mod go_to;
pub mod jobs;
mod list;
mod quick;
pub mod view;

use model::CommitId;
use model::search::{BoxMode, Direction, SearchMode, hash_prefix, loaded};
use views::search::SearchIntent;

use super::message::{Effect, JobId};
use super::state::State;

pub use go_to::{go_to, settle};
pub use jobs::{SearchJob, SearchResult};
pub use list::narrowing;
pub use view::SearchView;

/// The search of the timeline, **Go to commit** and Quick search.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Search {
    /// The mode chosen in the list before the box.
    pub mode: BoxMode,
    /// What is in the box.
    pub text: String,
    /// What the search found.
    pub found: Found,
    /// The rows shown that it found, in order.
    pub found_rows: Vec<usize>,
    /// The search of the whole history asked last, while what it found is shown.
    pub asked: Option<Asked>,
    /// The read of a search of the whole history, while it runs.
    pub matches_job: Option<JobId>,
    /// What is said under the box: a search git refused, or a hash that is no commit.
    pub failure: Option<String>,
    /// Whether the text is a hash that no loaded commit has, which `Enter` goes to (`search.go-to-hash`).
    pub is_hash_offered: bool,
    /// The read of the commit of a hash, while it runs.
    pub hash_job: Option<JobId>,
    /// The search the timeline is narrowed to, shown as a list (`search.list-mode`).
    pub list: Option<Asked>,
    /// The commit whose history alone the timeline lists: a commit gone to on no branch or tag
    /// (`search.go-to-hash`).
    pub alone: Option<CommitId>,
    /// The commit being gone to, until it is selected.
    pub going_to: Option<go_to::GoTo>,
    /// **Go to commit**, while it is open.
    pub dialog: Option<GoToCommit>,
    /// Quick search.
    pub quick: QuickSearch,
}

/// What a search found.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Found {
    /// Nothing was asked, or what was found was dropped: nothing is dimmed.
    #[default]
    Nothing,
    /// The commits found, in the order of the history, and whether the search stopped counting.
    Commits {
        /// The commits.
        commits: Vec<CommitId>,
        /// Whether there are more than those.
        has_more: bool,
    },
}

impl Found {
    /// The commits found.
    #[must_use]
    pub fn commits(&self) -> &[CommitId] {
        match self {
            Found::Nothing => &[],
            Found::Commits { commits, .. } => commits,
        }
    }
}

/// A search of the whole history, as it was asked.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Asked {
    /// How git looks.
    pub mode: SearchMode,
    /// What it looks for.
    pub text: String,
}

/// **Go to commit**, while it is open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GoToCommit {
    /// What is typed.
    pub text: String,
    /// The commit git says it stands for, once it said: none when it is no commit.
    pub verdict: Option<Option<CommitId>>,
    /// The read of what git says, while it runs.
    pub job: Option<JobId>,
}

/// Quick search.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QuickSearch {
    /// Whether its picker is open.
    pub is_open: bool,
    /// What was typed in its box in the header, which the picker starts from.
    pub typed: String,
}

/// Changes the search as the user asked.
pub fn reduce(state: &mut State, intent: SearchIntent) -> Vec<Effect> {
    match intent {
        SearchIntent::Mode(mode) => {
            state.search.mode = mode;
            changed(state)
        }
        SearchIntent::Type(text) => {
            state.search.text = text;
            changed(state)
        }
        SearchIntent::Empty => {
            state.search.text.clear();
            changed(state)
        }
        SearchIntent::Submit => submit(state),
        SearchIntent::Step(direction) => step(state, direction),
        SearchIntent::ToggleList => list::toggle(state),
        SearchIntent::ShowAllCommits => list::show_all(state),
        SearchIntent::GoToCommit
        | SearchIntent::GoToCommitTyped(_)
        | SearchIntent::GoToCommitSubmit
        | SearchIntent::GoToCommitClose => go_to::dialog(state, intent),
        SearchIntent::OpenQuick(_) | SearchIntent::ChooseQuick(_) | SearchIntent::CloseQuick => {
            quick::reduce(state, intent)
        }
    }
}

/// The text or the mode changed: what was found is dropped; the commits loaded are searched again at once.
fn changed(state: &mut State) -> Vec<Effect> {
    let effects = drop_found(state);
    after_layout(state);
    effects
}

/// Whether a search shown as a list, or the history of a commit alone, narrows the timeline: it then has no draft and
/// no stashes, and the bar of either takes the place of the bar of the branches shown.
#[must_use]
pub fn is_narrowed(state: &State) -> bool {
    state.search.list.is_some() || state.search.alone.is_some()
}

/// Drops what was found and what was said, and cancels the reads of the search.
pub fn drop_found(state: &mut State) -> Vec<Effect> {
    let search = &mut state.search;
    search.found = Found::Nothing;
    search.found_rows.clear();
    search.asked = None;
    search.failure = None;
    search.is_hash_offered = false;
    let jobs = [search.matches_job.take(), search.hash_job.take()];
    cancel(state, jobs.into_iter().flatten())
}

/// Cancels jobs that are not wanted any more.
pub fn cancel(state: &mut State, jobs: impl IntoIterator<Item = JobId>) -> Vec<Effect> {
    jobs.into_iter()
        .map(|job| {
            let _forgotten = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// The rows were laid out again, or the text changed: the commits loaded are searched again with **Loaded**, and the
/// rows found are found again among those shown.
pub fn after_layout(state: &mut State) {
    let search = &mut state.search;
    let timeline = &state.timeline;
    if search.mode == BoxMode::Loaded {
        let commits = loaded(&timeline.rows, &search.text);
        search.is_hash_offered = hash_prefix(&search.text).is_some_and(|prefix| {
            !timeline
                .rows
                .iter()
                .filter_map(model::graph::Row::id)
                .any(|id| id.as_str().starts_with(&prefix))
        });
        search.found = if search.text.trim().is_empty() {
            Found::Nothing
        } else {
            Found::Commits {
                commits,
                has_more: false,
            }
        };
    }
    let layout = &timeline.layout;
    let mut rows: Vec<usize> = search
        .found
        .commits()
        .iter()
        .filter(|commit| !layout.folded.folded_by.contains_key(*commit))
        .filter_map(|commit| layout.shown_of.get(commit).copied())
        .collect();
    rows.sort_unstable();
    rows.dedup();
    search.found_rows = rows;
}

/// `Enter` in the box: with **Loaded**, goes to the hash typed or steps; in the other modes, asks git, or steps once
/// it found, and does nothing more while git looks.
fn submit(state: &mut State) -> Vec<Effect> {
    let Some(mode) = state.search.mode.history() else {
        if state.search.is_hash_offered {
            return go_to::hash(state);
        }
        return step(state, Direction::Next);
    };
    let asked = Asked {
        mode,
        text: state.search.text.trim().to_owned(),
    };
    if asked.text.is_empty() {
        return Vec::new();
    }
    if state.search.asked.as_ref() == Some(&asked) {
        if state.search.matches_job.is_some() || state.search.failure.is_some() {
            return Vec::new();
        }
        return step(state, Direction::Next);
    }
    if state.persisted.search_as_list {
        return list::show(state, asked);
    }
    ask(state, asked)
}

/// Asks git for the commits a search of the whole history finds, in the history the timeline lists.
pub fn ask(state: &mut State, asked: Asked) -> Vec<Effect> {
    let mut effects = drop_found(state);
    let Some(request) = jobs::matches_request(state, &asked) else {
        return effects;
    };
    state.search.asked = Some(asked);
    let job = jobs::start(state, request);
    state.search.matches_job = Some(job.id);
    effects.push(Effect::Start(job));
    effects
}

/// Steps to the next or the previous match, and goes to it (`search.step`).
fn step(state: &mut State, direction: Direction) -> Vec<Effect> {
    let selected = match &state.timeline.selected {
        Some(views::graph::Selected::Commit(commit)) => Some(commit.clone()),
        _ => None,
    };
    match model::search::step(state.search.found.commits(), selected.as_ref(), direction) {
        Some(commit) => go_to(state, commit),
        None => Vec::new(),
    }
}

/// The result of a read of the search.
pub fn searched(state: &mut State, job: JobId, result: SearchResult) -> Vec<Effect> {
    if state.jobs.remove(&job).is_none() {
        return Vec::new();
    }
    match result {
        SearchResult::Matches(matches) => matches_read(state, job, matches),
        SearchResult::Revision(commit) => go_to::revision_read(state, job, commit),
        SearchResult::Position(position) => go_to::position_read(state, job, position),
    }
}

/// What a search of the whole history found: the first of them is selected and shown, unless the commit selected is
/// one of them or the search is shown as a list; a search git refused says why and dims nothing.
fn matches_read(state: &mut State, job: JobId, result: Result<model::search::Matches, String>) -> Vec<Effect> {
    if state.search.matches_job != Some(job) {
        return Vec::new();
    }
    state.search.matches_job = None;
    let matches = match result {
        Ok(matches) => matches,
        Err(error) => {
            state.search.failure = Some(views::text::search::search_failed(&error));
            state.search.found = Found::Commits {
                commits: Vec::new(),
                has_more: false,
            };
            after_layout(state);
            return Vec::new();
        }
    };
    let was_shown = !matches!(state.search.found, Found::Nothing);
    let first = matches.commits.first().cloned();
    let is_selected_found = matches!(&state.timeline.selected,
        Some(views::graph::Selected::Commit(commit)) if matches.commits.contains(commit));
    state.search.found = Found::Commits {
        commits: matches.commits,
        has_more: matches.has_more,
    };
    after_layout(state);
    match first {
        Some(first) if !was_shown && !is_selected_found && state.search.list.is_none() => go_to(state, first),
        _ => Vec::new(),
    }
}

/// The repository was read again: a search of the whole history whose commits are shown is asked again, so that a
/// commit made or fetched meanwhile is counted too; one git refused is not asked again.
pub fn repository_read(state: &mut State) -> Vec<Effect> {
    let search = &state.search;
    let is_shown = !matches!(search.found, Found::Nothing) && search.failure.is_none();
    let Some(asked) = search
        .asked
        .clone()
        .filter(|_| is_shown && search.matches_job.is_none())
    else {
        return Vec::new();
    };
    let Some(request) = jobs::matches_request(state, &asked) else {
        return Vec::new();
    };
    let job = jobs::start(state, request);
    state.search.matches_job = Some(job.id);
    vec![Effect::Start(job)]
}
