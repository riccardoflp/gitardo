//! The rows of the graph, and what the lines of the graph are laid out from.

use crate::CommitId;
use crate::commit::Commit;

/// One row of the graph, in the order listed, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A commit of the history listed.
    Commit(Commit),
    /// A stash, with only its first parent: the commit it was made on. Its other parents, the index and the untracked
    /// files it took, are left out, since the stash stands for them.
    Stash {
        /// The commit git keeps the stash as.
        commit: Commit,
        /// Its place in the list of stashes: 0 for `stash@{0}`.
        index: usize,
    },
    /// The changes not committed, above the commit checked out.
    Uncommitted {
        /// The commit checked out; none before the first commit.
        parent: Option<CommitId>,
        /// How many files have a change of any kind.
        files: usize,
    },
}

impl Row {
    /// The commit of the row; none for the uncommitted changes.
    #[must_use]
    pub fn commit(&self) -> Option<&Commit> {
        match self {
            Row::Commit(commit) | Row::Stash { commit, .. } => Some(commit),
            Row::Uncommitted { .. } => None,
        }
    }

    /// The hash of the row's commit; none for the uncommitted changes.
    #[must_use]
    pub fn id(&self) -> Option<&CommitId> {
        self.commit().map(|commit| &commit.id)
    }

    /// The commits the row comes from, the first parent first.
    #[must_use]
    pub fn parents(&self) -> &[CommitId] {
        match self {
            Row::Commit(commit) | Row::Stash { commit, .. } => &commit.parents,
            Row::Uncommitted { parent, .. } => parent.as_slice(),
        }
    }

    /// Whether the row's line is dashed: the uncommitted changes and the stashes are not commits of the history.
    #[must_use]
    pub fn is_dashed(&self) -> bool {
        match self {
            Row::Commit(_) => false,
            Row::Stash { .. } | Row::Uncommitted { .. } => true,
        }
    }

    /// What the lines of the graph are laid out from for this row.
    #[must_use]
    pub fn graph_input(&self) -> GraphInput<'_> {
        GraphInput {
            id: self.id(),
            parents: self.parents(),
            is_dashed: self.is_dashed(),
        }
    }
}

/// A row as the lines of the graph see it: its hash, the commits it comes from, and whether its line is dashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphInput<'rows> {
    /// The hash of its commit; none for a row no other row comes from, such as the uncommitted changes.
    pub id: Option<&'rows CommitId>,
    /// Its parents, the first one first. A row given without parents has no line down.
    pub parents: &'rows [CommitId],
    /// Whether its line is dashed.
    pub is_dashed: bool,
}
