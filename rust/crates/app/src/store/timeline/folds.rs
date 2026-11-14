//! The folds of the timeline (`graph.fold-merge`, `graph.fold-all`), kept for each repository between runs.

use std::path::Path;

use model::CommitId;

use super::super::message::Effect;
use super::super::state::State;
use super::relayout;
use crate::persisted::RepositoryFolds;
use crate::persisted::same_folder;
use crate::persisted::timeline::{MOST_FOLDS, folds_of, remember};

/// Takes the folds kept for a repository, when it is shown again.
pub fn restore(state: &mut State, folder: &Path) {
    let Some(kept) = folds_of(&state.persisted.folds, folder) else {
        return;
    };
    let hashes = |list: &[String]| list.iter().filter_map(|hash| CommitId::parse(hash)).collect();
    state.timeline.folded = hashes(&kept.folded);
    state.timeline.unfolded = hashes(&kept.unfolded);
    state.timeline.is_all_folded = kept.is_all_folded;
}

/// While every merge is folded, folds the merges loaded since, but those unfolded by hand.
pub fn fold_new_merges(state: &mut State) {
    let timeline = &mut state.timeline;
    if !timeline.is_all_folded {
        return;
    }
    let merges: Vec<CommitId> = model::graph::foldable_merges(&timeline.rows)
        .into_iter()
        .filter(|merge| !timeline.unfolded.contains(merge))
        .collect();
    timeline.folded.extend(merges);
}

/// Folds a merge.
pub fn fold(state: &mut State, merge: CommitId) -> Vec<Effect> {
    let _was_unfolded = state.timeline.unfolded.remove(&merge);
    let _new = state.timeline.folded.insert(merge);
    changed(state)
}

/// Shows the commits of a folded merge again; while all are folded, it stays unfolded after the next page.
pub fn unfold(state: &mut State, merge: &CommitId) -> Vec<Effect> {
    let _was_folded = state.timeline.folded.remove(merge);
    if state.timeline.is_all_folded {
        let _new = state.timeline.unfolded.insert(merge.clone());
    }
    changed(state)
}

/// **Fold all merges**: every merge loaded, and those loaded later; checked again, it unfolds them all.
pub fn fold_all(state: &mut State) -> Vec<Effect> {
    if state.timeline.is_all_folded {
        return unfold_all(state);
    }
    state.timeline.is_all_folded = true;
    state.timeline.unfolded.clear();
    fold_new_merges(state);
    changed(state)
}

/// **Unfold all merges**: every commit shown again, and every fold forgotten.
pub fn unfold_all(state: &mut State) -> Vec<Effect> {
    state.timeline.is_all_folded = false;
    state.timeline.folded.clear();
    state.timeline.unfolded.clear();
    changed(state)
}

/// Once the whole history is loaded, forgets the merges folded that are no longer in it, after a rebase or a reset.
pub fn forget_gone(state: &mut State) -> Vec<Effect> {
    let timeline = &mut state.timeline;
    let before = timeline.folded.len();
    let listed: std::collections::HashSet<&CommitId> = timeline.rows.iter().filter_map(model::graph::Row::id).collect();
    timeline.folded.retain(|merge| listed.contains(merge));
    if timeline.folded.len() == before {
        return Vec::new();
    }
    vec![remembered(state)]
}

/// Lays the timeline out again after its folds changed, and keeps them.
fn changed(state: &mut State) -> Vec<Effect> {
    relayout(state);
    vec![remembered(state)]
}

/// Keeps the folds of the repository shown with what is kept between runs, the last ones first.
fn remembered(state: &mut State) -> Effect {
    if let Some(folder) = state.timeline.folder.clone() {
        let sorted = |set: &std::collections::HashSet<CommitId>| {
            let mut hashes: Vec<String> = set.iter().map(|id| id.as_str().to_owned()).collect();
            hashes.sort_unstable();
            hashes.truncate(MOST_FOLDS);
            hashes
        };
        let folds = RepositoryFolds {
            folder: folder.clone(),
            folded: sorted(&state.timeline.folded),
            unfolded: sorted(&state.timeline.unfolded),
            is_all_folded: state.timeline.is_all_folded,
        };
        remember(&mut state.persisted.folds, folds, |kept| {
            same_folder(&kept.folder, &folder)
        });
    }
    Effect::WritePersisted(state.persisted.clone())
}
