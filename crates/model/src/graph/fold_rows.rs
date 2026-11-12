//! The rows of the graph with the commits each folded merge brought in put away under one row.

mod brought_in;
mod fold;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::BuildHasher;

use crate::CommitId;
use crate::graph::reach::Ancestry;
use crate::graph::row::{GraphInput, Row};

pub use fold::Fold;

/// The rows as the graph shows them with some merges folded, and which merge put away each commit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FoldedRows {
    /// The rows shown, in order.
    pub rows: Vec<ShownRow>,
    /// Each commit put away, with the merge whose fold holds it.
    pub folded_by: HashMap<CommitId, CommitId>,
}

/// A row as the graph shows it with merges folded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShownRow {
    /// A row of the list.
    Row {
        /// Its place in the rows given.
        index: usize,
        /// Its parents as the lines are drawn, when one of them is put away under a fold: the fold stands for it.
        parents: Option<Vec<CommitId>>,
    },
    /// The row that stands for the commits a merge brought in.
    Fold(Fold),
}

impl FoldedRows {
    /// What the lines of the graph are laid out from for the rows shown.
    #[must_use]
    pub fn graph_inputs<'rows>(&'rows self, rows: &'rows [Row]) -> Vec<GraphInput<'rows>> {
        self.rows
            .iter()
            .filter_map(|shown| match shown {
                ShownRow::Row { index, parents } => rows.get(*index).map(|row| GraphInput {
                    parents: parents.as_deref().unwrap_or(row.parents()),
                    ..row.graph_input()
                }),
                ShownRow::Fold(fold) => Some(GraphInput {
                    id: Some(&fold.id),
                    parents: &fold.parents,
                    is_dashed: false,
                }),
            })
            .collect()
    }
}

/// The rows with each merge of `folded` folded: the commits it brought in (those loaded that its other parents reach
/// and its first parent does not) put away under one row. That row goes where the first of them was, or under the
/// lowest commit made on top of any of them when that is further down, and has as parents the commits they come from
/// that are listed. A merge that brought nothing in, or whose commits cannot have one row there without being below
/// what they come from, stays unfolded. Merges are folded from the top, and a commit put away by one is not put away
/// again by another.
///
/// Feature: graph.fold-rows
#[must_use]
pub fn fold_rows<Hasher: BuildHasher>(rows: &[Row], folded: &HashSet<CommitId, Hasher>) -> FoldedRows {
    let ancestry = Ancestry::new(rows);
    let children = children(&ancestry);
    let mut put_away: Vec<Option<usize>> = vec![None; rows.len()];
    let mut folds: Vec<(usize, Fold)> = Vec::new();
    for (merge, row) in rows.iter().enumerate() {
        let Row::Commit(commit) = row else {
            continue;
        };
        let is_wanted = commit.is_merge() && folded.contains(&commit.id);
        if !is_wanted || put_away.get(merge).copied().flatten().is_some() {
            continue;
        }
        let mut brought = brought_in::brought_in(&ancestry, commit, merge);
        brought.retain(|index| put_away.get(*index).copied().flatten().is_none());
        let Some((anchor, parents)) = placement(&ancestry, &children, &brought) else {
            continue;
        };
        for index in &brought {
            if let Some(slot) = put_away.get_mut(*index) {
                *slot = Some(folds.len());
            }
        }
        let ids = |indices: &[usize]| -> Vec<CommitId> {
            indices
                .iter()
                .filter_map(|index| rows.get(*index).and_then(Row::id).cloned())
                .collect()
        };
        folds.push((anchor, Fold::new(commit, ids(&brought), ids(&parents))));
    }
    shown_rows(rows, &put_away, folds)
}

/// Where the row of the commits brought in goes, and the rows of the commits they come from; none when it cannot go
/// above all of those, or when nothing was brought in. The row goes after the row of the place returned: the first of
/// the commits, which is put away, or the lowest commit made on top of any of them.
fn placement(ancestry: &Ancestry<'_>, children: &[Vec<usize>], brought: &[usize]) -> Option<(usize, Vec<usize>)> {
    let first = *brought.first()?;
    let is_brought = |index: &usize| brought.binary_search(index).is_ok();
    let lowest_on_top = brought
        .iter()
        .flat_map(|index| children.get(*index).map_or(&[][..], Vec::as_slice))
        .filter(|child| !is_brought(child))
        .copied()
        .max();
    let anchor = lowest_on_top.filter(|lowest| *lowest > first).unwrap_or(first);
    let mut parents: Vec<usize> = brought
        .iter()
        .flat_map(|index| ancestry.parents_of(*index))
        .filter(|parent| !is_brought(parent))
        .copied()
        .collect();
    parents.sort_unstable();
    parents.dedup();
    if parents.first().is_some_and(|parent| *parent <= anchor) {
        return None;
    }
    Some((anchor, parents))
}

/// The rows of the commits made on top of each row.
fn children(ancestry: &Ancestry<'_>) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); ancestry.len()];
    for child in 0..ancestry.len() {
        for parent in ancestry.parents_of(child) {
            if let Some(list) = children.get_mut(*parent) {
                list.push(child);
            }
        }
    }
    children
}

/// The rows shown: those not put away, with the parents put away replaced by the folds that stand for them, and each
/// fold after the row of its place.
fn shown_rows(rows: &[Row], put_away: &[Option<usize>], folds: Vec<(usize, Fold)>) -> FoldedRows {
    let mut stands_for: HashMap<&CommitId, CommitId> = HashMap::new();
    let mut folded_by = HashMap::new();
    for (row, number) in rows.iter().zip(put_away) {
        let holder = number.and_then(|number| folds.get(number)).map(|(_, fold)| fold);
        if let (Some(id), Some(holder)) = (row.id(), holder) {
            let _ = stands_for.insert(id, holder.id.clone());
            let _ = folded_by.insert(id.clone(), holder.merge.clone());
        }
    }
    let drawn = |parents: &[CommitId]| -> Option<Vec<CommitId>> {
        if !parents.iter().any(|parent| stands_for.contains_key(parent)) {
            return None;
        }
        let mut drawn: Vec<CommitId> = Vec::with_capacity(parents.len());
        for parent in parents {
            let parent = stands_for.get(parent).unwrap_or(parent);
            if !drawn.contains(parent) {
                drawn.push(parent.clone());
            }
        }
        Some(drawn)
    };
    let mut after: BTreeMap<usize, Vec<Fold>> = BTreeMap::new();
    for (anchor, mut fold) in folds {
        if let Some(parents) = drawn(&fold.parents) {
            fold.parents = parents;
        }
        after.entry(anchor).or_default().push(fold);
    }
    let mut shown = Vec::with_capacity(rows.len());
    for (index, (row, fold)) in rows.iter().zip(put_away).enumerate() {
        if fold.is_none() {
            let parents = drawn(row.parents());
            shown.push(ShownRow::Row { index, parents });
        }
        if let Some(folds) = after.remove(&index) {
            shown.extend(folds.into_iter().map(ShownRow::Fold));
        }
    }
    FoldedRows { rows: shown, folded_by }
}

#[cfg(test)]
mod tests;
