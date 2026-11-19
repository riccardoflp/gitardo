//! Whether the history of a repository is shallow, and where it ends (`repo.read-shallow`).

use crate::CommitId;

/// Where the history of a shallow clone ends: the commits whose parents were not fetched, which git lists with no
/// parent, as first commits. None in a repository that is not shallow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Shallow {
    /// The commits whose parents were not fetched, as the file `shallow` lists them.
    pub ends: Vec<CommitId>,
}

impl Shallow {
    /// Whether the repository is a shallow clone.
    #[must_use]
    pub fn is_shallow(&self) -> bool {
        !self.ends.is_empty()
    }

    /// Whether the parents of a commit were not fetched: the history of a shallow clone ends at it.
    #[must_use]
    pub fn ends_at(&self, commit: &CommitId) -> bool {
        self.ends.contains(commit)
    }
}
