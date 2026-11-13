//! The interactive rebase page (`rebase.todo-page`) and the banner of a rebase stopped to edit or split a commit
//! (`rebase.edit-stop`), in the store of the window; and what the window does after a squash or a split.

use model::CommitId;
use model::contract::{Answer, Answers, FeatureId, FollowUp, Target};
use model::graph::Row;
use model::rebase::{LineBeforeHeadInput, find_line_before_head};
use model::state::RepositoryState;
use views::feature::FeatureIntent;
use views::page::Page;
use views::rebase::{EditStopState, RebaseIntent, TodoPage, TodoPageState};

use super::message::Effect;
use super::state::State;
use crate::persisted::changes::keep_message;

/// Reduces what is asked of the interactive rebase page.
pub fn reduce(state: &mut State, intent: RebaseIntent) -> Vec<Effect> {
    match intent {
        RebaseIntent::Open(commit) => {
            if let Some(page) = new_page(state, &commit) {
                state.rebase_page = Some(page);
                super::pages::show_page(state, Page::Rebase);
            }
            Vec::new()
        }
        RebaseIntent::Cancel => close(state),
        RebaseIntent::Start => {
            let Some(page) = state.rebase_page.as_ref() else {
                return Vec::new();
            };
            if page.problem().is_some() || !page.is_changed() {
                return Vec::new();
            }
            let answers = Answers::new().with(features::rebase::TODO, Answer::Text(page.todo_rows()));
            let mut effects = close(state);
            let run = FeatureIntent::RunWith(FeatureId("rebase.interactive"), Target::Head, answers);
            effects.extend(super::feature::reduce(state, run));
            effects
        }
        other => {
            if let Some(page) = state.rebase_page.as_mut() {
                page.apply(&other);
            }
            Vec::new()
        }
    }
}

/// The page of the commits from a commit up to `HEAD`, newest first, when they are a line among the rows loaded.
fn new_page(state: &State, commit: &CommitId) -> Option<TodoPage> {
    let read = state.shown_read()?;
    let commits: Vec<model::commit::Commit> = state
        .timeline
        .read_rows()
        .iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some(commit.clone()),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        })
        .collect();
    let line = find_line_before_head(&LineBeforeHeadInput {
        commits: &commits,
        head: read.state.head.commit.as_ref(),
        commit,
    })?;
    let rows = line
        .into_iter()
        .map(|id| {
            let subject = commits
                .iter()
                .find(|each| each.id == id)
                .map(|each| each.subject.clone())
                .unwrap_or_default();
            (id, subject)
        })
        .collect();
    let branch = read.state.checked_out_branch().unwrap_or("HEAD").to_owned();
    Some(TodoPage::new(branch, rows))
}

/// Closes the page, wherever it is among the pages open.
fn close(state: &mut State) -> Vec<Effect> {
    state.rebase_page = None;
    if let Some(index) = state.pages.open.iter().position(|page| *page == Page::Rebase) {
        state.pages.shown = Some(index);
        return super::pages::close_page(state);
    }
    Vec::new()
}

/// What the page shows, while it is open.
#[must_use]
pub fn page_state(state: &State) -> Option<TodoPageState<'_>> {
    state.rebase_page.as_ref().map(|page| TodoPageState { page })
}

/// The banner of a rebase stopped to edit or split a commit, when the repository shown is.
#[must_use]
pub fn stop_state(repository: &RepositoryState) -> Option<EditStopState<'_>> {
    repository.rebase_stop.as_ref().map(|stop| EditStopState {
        stop,
        has_conflicts: !repository.working_tree.conflicted.is_empty(),
    })
}

/// After a squash or a split: the commit as many before `HEAD` as the follow-up says is selected once the repository
/// is read again, and the message of the commit split goes to the commit box.
pub fn ended(state: &mut State, after: &[FollowUp]) {
    for follow_up in after {
        match follow_up {
            FollowUp::FillCommitMessage(message) => {
                if let Some(folder) = state.shown_folder().cloned() {
                    keep_message(&mut state.persisted.commit_messages, &folder, message);
                }
            }
            _ => {}
        }
    }
}
