//! Which rows a commit reaches through the parents listed, and which reach it.

use std::collections::HashMap;

use crate::CommitId;
use crate::graph::row::Row;

/// The rows of the graph linked to the rows of their parents, as far as those are listed.
#[derive(Debug)]
pub(crate) struct Ancestry<'rows> {
    /// The row of each commit.
    rows_of: HashMap<&'rows CommitId, usize>,
    /// The rows of the parents of each row, the first parent first; a parent not listed has none.
    parents: Vec<Vec<usize>>,
}

impl<'rows> Ancestry<'rows> {
    /// The links between rows.
    pub(crate) fn new(rows: &'rows [Row]) -> Self {
        let rows_of: HashMap<&CommitId, usize> = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.id().map(|id| (id, index)))
            .collect();
        let parents = rows
            .iter()
            .map(|row| {
                row.parents()
                    .iter()
                    .filter_map(|parent| rows_of.get(parent).copied())
                    .collect()
            })
            .collect();
        Ancestry { rows_of, parents }
    }

    /// How many rows there are.
    pub(crate) fn len(&self) -> usize {
        self.parents.len()
    }

    /// The row of a commit, when it is listed.
    pub(crate) fn row_of(&self, commit: &CommitId) -> Option<usize> {
        self.rows_of.get(commit).copied()
    }

    /// The rows of the parents of a row that are listed.
    pub(crate) fn parents_of(&self, row: usize) -> &[usize] {
        self.parents.get(row).map_or(&[], Vec::as_slice)
    }

    /// For each row, whether one of the rows given reaches it: the rows themselves, their parents, and theirs.
    pub(crate) fn reached_from(&self, starts: &[usize]) -> Vec<bool> {
        let mut reached = vec![false; self.len()];
        let mut waiting: Vec<usize> = starts.to_vec();
        while let Some(row) = waiting.pop() {
            match reached.get_mut(row) {
                Some(seen) if !*seen => *seen = true,
                _ => continue,
            }
            waiting.extend_from_slice(self.parents_of(row));
        }
        reached
    }

    /// For each row, whether it reaches a row: the row itself and the commits made on top of it, as far as they are
    /// listed. Rows are listed before their parents, so the rows that reach it are all above it.
    pub(crate) fn reaching(&self, row: usize) -> Vec<bool> {
        let mut reaching = vec![false; self.len()];
        if let Some(seen) = reaching.get_mut(row) {
            *seen = true;
        }
        for index in (0..row.min(self.len())).rev() {
            let reaches = self
                .parents_of(index)
                .iter()
                .any(|parent| reaching.get(*parent).copied().unwrap_or(false));
            if let Some(seen) = reaching.get_mut(index) {
                *seen = reaches;
            }
        }
        reaching
    }
}
