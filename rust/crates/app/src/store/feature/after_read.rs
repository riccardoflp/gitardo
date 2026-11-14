//! What is done once the repository is read again after an operation: the warning of changes a pull with
//! `--autostash` left in a stash, the first bad commit of a bisect, and the commit the operation said to select.

use model::CommitId;

use crate::store::{Effect, State, search};

/// Once the repository is read again after a pull with `--autostash`, warns when the changes it set aside were left in
/// a stash, with **Show Stashes** beside the warning.
fn autostash_left(state: &mut State) {
    let Some(before) = state.operations.autostash_before.take() else {
        return;
    };
    let notice = state.shown_read().and_then(|read| {
        model::stash::autostash_left(&model::stash::PullOutcome {
            had_autostash: true,
            succeeded: true,
            stashes_before: &before,
            stashes_after: &read.state.stashes,
        })
    });
    if let (Some(notice), Some(said)) = (notice, state.operations.said.as_mut()) {
        notice.message.as_str().clone_into(&mut said.text);
        said.is_error = false;
        said.offers_stashes = true;
    }
}

/// Once the repository is read again after an answer to a bisect, says the first bad commit when it is found.
fn bisect_found(state: &mut State) {
    if !std::mem::take(&mut state.operations.check_bisect) {
        return;
    }
    let found = state.shown_read().and_then(|read| {
        let bisect = read
            .state
            .bisect
            .as_ref()
            .filter(|bisect| bisect.remaining == Some(1))?;
        let bad = bisect.bad.clone()?;
        let subject = state
            .timeline
            .read_rows()
            .iter()
            .find_map(|row| row.commit().filter(|commit| commit.id == bad))
            .map_or_default(|commit| commit.subject.clone());
        Some((bad, subject))
    });
    if let (Some((bad, subject)), Some(said)) = (found, state.operations.said.as_mut()) {
        said.text = views::text::bisect::first_bad(bad.short(), &subject);
        said.can_undo = false;
        said.found = Some(bad);
    }
}

/// Once the repository is read again after an operation, the commit it said to select is gone to.
pub fn after_read(state: &mut State) -> Vec<Effect> {
    autostash_left(state);
    bisect_found(state);
    if let Some(effects) = run_pending(state) {
        return effects;
    }
    let Some(selected) = state.operations.select_after_read.take() else {
        return Vec::new();
    };
    let back = std::mem::take(&mut state.operations.select_before_head);
    let commit = selected.or_else(|| {
        let head = state.shown_read().and_then(|read| read.state.head.commit.clone())?;
        Some(before(&state.timeline.rows, head, back))
    });
    commit.map_or_default(|commit| search::go_to(state, commit))
}

/// The operation on a file of another repository, once the window shows that repository (`compare.path-with`); it is
/// forgotten when another repository was shown in its place.
fn run_pending(state: &mut State) -> Option<Vec<Effect>> {
    let (folder, feature, target) = state.operations.run_after_read.take()?;
    let read = state.shown_read()?;
    if !crate::persisted::same_folder(&read.folder, &folder) {
        return None;
    }
    let repository = (read.folder.clone(), std::sync::Arc::clone(&read.state));
    Some(super::run_in(
        state,
        repository,
        (feature, target),
        model::contract::Answers::new(),
    ))
}

/// The commit a number of commits before another, following first parents through the rows read; as far as they go.
fn before(rows: &[model::graph::Row], mut commit: CommitId, count: usize) -> CommitId {
    for _ in 0..count {
        let parent = rows
            .iter()
            .find(|row| row.id() == Some(&commit))
            .and_then(|row| row.parents().first().cloned());
        match parent {
            Some(parent) => commit = parent,
            None => break,
        }
    }
    commit
}
