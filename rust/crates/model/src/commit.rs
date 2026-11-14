//! A commit as the lists show it: the graph, a search, the history of a file; and the message of a new commit drafted
//! by the language model (`commit.draft-message`).

mod draft;

pub use draft::{DraftInput, clean_draft, draft_message};

use crate::{CommitId, Timestamp};

/// A commit as a row of a list has it: who made it, when, and the first line of its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// Its full hash.
    pub id: CommitId,
    /// Its parents, the first one first; none for a root commit.
    pub parents: Vec<CommitId>,
    /// The name of its author.
    pub author_name: String,
    /// The address of its author.
    pub author_email: String,
    /// When its author made it.
    pub author_date: Timestamp,
    /// The first line of its message.
    pub subject: String,
}

impl Commit {
    /// Whether it has more than one parent.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}
