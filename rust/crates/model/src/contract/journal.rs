//! What Undo needs to take back an operation of Gitardo that the reflog does not see: kept by the engine in its journal,
//! as each plan said when it ran (`undo.journal`).

use crate::CommitId;
use crate::state::Head;

use super::Text;

/// What a plan leaves for Undo, when the reflog is not enough to take it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoRecord {
    /// A branch was deleted: Undo creates it again at the same commit, without its upstream.
    BranchDeleted {
        /// Its name, without `refs/heads/`.
        name: String,
        /// The commit it was at.
        commit: CommitId,
    },
    /// A stash was dropped: Undo stores it again with its message.
    StashDropped {
        /// Its message.
        message: String,
        /// Its commit.
        commit: CommitId,
    },
    /// The conflicts of these files were resolved: Undo marks them again, staged ones included.
    ConflictsResolved {
        /// The files, relative to the top of the working tree.
        files: Vec<String>,
    },
    /// A squash merge, which leaves its changes staged and nothing committed: Undo resets them away.
    SquashMerge,
    /// A series of commits made at once (cherry-picks, a patch applied as commits, a squash, the split of an older
    /// commit): Undo takes them all back to where `HEAD` was before.
    Series {
        /// Where `HEAD` was before the series.
        head_before: CommitId,
    },
    /// The last commit was split: Undo resets to it, keeping the files it added, which are untracked now.
    SplitLast {
        /// The commit that was split.
        commit: CommitId,
    },
}

impl UndoRecord {
    /// Whether the entry is kept also when the plan stopped half way on a conflict: a squash merge and a series of
    /// commits are taken back whole, also from where they stopped.
    #[must_use]
    pub fn holds_after_a_stop(&self) -> bool {
        matches!(self, UndoRecord::SquashMerge | UndoRecord::Series { .. })
    }
}

/// An entry of the journal: what a plan of Gitardo left for Undo, and where `HEAD` was once it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// Its number, in the order the entries were made, so that the plan of Undo can say which one it takes back.
    pub number: u64,
    /// What the plan did, in words: its title.
    pub title: Text,
    /// What Undo needs.
    pub record: UndoRecord,
    /// What was checked out once the plan ran. The entry holds only while it still is.
    pub head: Head,
}
