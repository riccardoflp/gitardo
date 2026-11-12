//! The details of a commit: what the details beside the graph, the Stashes pane and the page of a commit show of one
//! commit, and its files laid out under their directories.

mod tree;

use crate::state::FileChange;
use crate::{CommitId, Timestamp};

pub use tree::{FilePath, TreeNode, file_tree};

/// The subject the uncommitted changes have when they are shown as a commit.
pub const UNCOMMITTED_SUBJECT: &str = "Uncommitted changes";

/// Who made a commit, or committed it, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The name.
    pub name: String,
    /// The address.
    pub email: String,
    /// When.
    pub date: Timestamp,
}

/// Everything the views show of one commit, or of the uncommitted changes asked for as a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetail {
    /// Its full hash; none for the uncommitted changes.
    pub id: Option<CommitId>,
    /// Its parents, the first one first; none for a root commit and for the uncommitted changes.
    pub parents: Vec<CommitId>,
    /// Who made it; none for the uncommitted changes.
    pub author: Option<Signature>,
    /// Who committed it; none for the uncommitted changes.
    pub committer: Option<Signature>,
    /// Its message as it was written, subject, empty line and body, without the newline at its end; for the
    /// uncommitted changes, [`UNCOMMITTED_SUBJECT`].
    pub message: String,
    /// The files it changed against its first parent, every file it added for a root commit, or, for the uncommitted
    /// changes, the files of the index and of the working tree together.
    pub files: Vec<FileChange>,
    /// For a stash made with its untracked files, those files, apart from the others; otherwise none.
    pub untracked_files: Vec<FileChange>,
}

impl CommitDetail {
    /// Whether it has more than one parent, so that its files are those it changed against its first parent only.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    /// Its first parent, which its files are compared with; none for a root commit.
    #[must_use]
    pub fn first_parent(&self) -> Option<&CommitId> {
        self.parents.first()
    }
}
