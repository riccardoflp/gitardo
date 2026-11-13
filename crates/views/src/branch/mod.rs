//! The operations of branches, tags and commits where the window offers them: the entries of the menus of the branch
//! rail and of the timeline, and what a double click on a branch does (`branch.open`). The same reference has the same
//! entries wherever it is listed or labels its commit.

mod history;
mod menu;
#[cfg(test)]
mod tests;

use model::contract::OpenIn;
use model::state::RepositoryState;

pub(crate) use history::{compare_entries, history_entries};
pub(crate) use menu::{
    OperationEntry, branch_entries, chosen, commit_entries, menu_items, open_intent, remote_entries, tag_entries,
};

/// What the menus of the references and of the commits are made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceMenus<'a> {
    /// The repository shown, as last read: what is checked out, the worktrees, the upstreams and the remotes.
    pub repository: &'a RepositoryState,
    /// `branches.openIn`: what a double click on a branch does, and which entry comes first in its menu.
    pub open_in: OpenIn,
}
