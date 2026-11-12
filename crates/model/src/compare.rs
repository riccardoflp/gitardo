//! A comparison of two branches, tags, commits, or one of them and the working tree, as git reads it: how far apart
//! they are, what merging them would do, the commits only in each and the files that differ.

use crate::commit_page::PatchLines;
use crate::state::FileChange;
use crate::{CommitId, Timestamp};

/// The most commits a comparison lists, and the most for which it says which are also in the other side.
pub const COMMIT_LIMIT: usize = 2_000;

/// A side of a comparison, as git reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    /// A commit, which a branch, a tag or a revision stood for when the comparison was read.
    Commit(CommitId),
    /// The files as they are now, with the changes not committed; its commits are those of `HEAD`.
    WorkingTree,
}

/// How the two sides are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// **Since they diverged**: what the second side changed since the commit where the two diverged, as a pull request
    /// shows it.
    SinceDiverged,
    /// **Everything that differs**: the second side against the first, directly.
    Everything,
}

/// Which side of a comparison a commit is only in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSide {
    /// Only in the first side.
    First,
    /// Only in the second side.
    Second,
}

/// A commit only in one side of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparedCommit {
    /// The side it is only in.
    pub side: CommitSide,
    /// Its full hash.
    pub id: CommitId,
    /// The name of its author.
    pub author_name: String,
    /// When its author made it.
    pub author_date: Timestamp,
    /// The first line of its message.
    pub subject: String,
    /// Whether the other side has the same change in a commit of its own, as a commit cherry-picked: **also in**.
    pub is_also_in_other_side: bool,
}

/// What merging the two sides would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutlook {
    /// Not asked: a side is the working tree, or one side has every commit of the other.
    NotAsked,
    /// The merge would have no conflict.
    Clean,
    /// The merge would leave these files in conflict.
    Conflicts(Vec<String>),
    /// It cannot be told: git is older than 2.40, or the histories never meet.
    Unknown,
}

/// Everything the comparison page shows of two sides but the lines of the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareView {
    /// Where the two diverged; none when their histories never meet.
    pub merge_base: Option<CommitId>,
    /// How many commits the first side has that the second has not.
    pub only_in_first: usize,
    /// How many commits the second side has that the first has not.
    pub only_in_second: usize,
    /// The files that differ, as the mode says.
    pub files: Vec<FileChange>,
    /// The commits only in each side, newest first, at most [`COMMIT_LIMIT`].
    pub commits: Vec<ComparedCommit>,
    /// What merging the two would do.
    pub merge: MergeOutlook,
}

/// The changes of a comparison, as lines for its page or as a patch to copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparePatch {
    /// The lines of each file, for the page.
    Lines(PatchLines),
    /// A patch that `git apply` takes, binary files included.
    Text(Vec<u8>),
}

/// The files that differ between two commits, for the comparison in place of the details.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comparison {
    /// The files, renames with both names.
    pub files: Vec<FileChange>,
}
