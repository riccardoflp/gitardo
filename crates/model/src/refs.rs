//! The references as the lists show them: in which order, and the branches checked out last.

mod recent;
mod sort;

use crate::state::{Branch, RemoteBranch, RepositoryState, Tag};

pub use recent::recent;
pub use sort::{SortOrder, order_in_force, sort};

/// The branches, the remote branches and the tags of a repository, each list in the order it is shown in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct References {
    /// The local branches.
    pub branches: Vec<Branch>,
    /// The remote branches.
    pub remote_branches: Vec<RemoteBranch>,
    /// The tags.
    pub tags: Vec<Tag>,
}

impl References {
    /// The references of a repository as read, in the order git lists them.
    #[must_use]
    pub fn of(state: &RepositoryState) -> Self {
        References {
            branches: state.branches.clone(),
            remote_branches: state.remote_branches.clone(),
            tags: state.tags.clone(),
        }
    }
}
