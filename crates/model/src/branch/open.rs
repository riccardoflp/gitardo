//! `branch.open`: what a double click on a local or a remote branch does, wherever it is listed or labels its commit.

use crate::contract::{FeatureId, OpenIn, Target};
use crate::state::RepositoryState;

use super::{local_name, other_worktree};

/// What a double click was on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenBranchInput<'a> {
    /// The repository, as last read: what is checked out, the worktrees and the remotes.
    pub state: &'a RepositoryState,
    /// The branch.
    pub reference: OpenedReference<'a>,
    /// `branches.openIn`.
    pub open_in: OpenIn,
}

/// The branch a double click was on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenedReference<'a> {
    /// A local branch, by its short name.
    Branch(&'a str),
    /// A remote branch, by its short name: `origin/feature`.
    RemoteBranch(&'a str),
}

/// The operation a double click on a branch runs, and what it runs on; nothing when it does nothing.
///
/// A local branch checked out here does nothing. One that another worktree has opens that worktree, since git checks
/// it out nowhere else, or does nothing when that worktree's folder is missing. Otherwise `branches.openIn` decides:
/// `checkout` checks it out, `worktree` opens it in a worktree of its own. A remote branch goes by the local branch of
/// its name the same way, and is checked out as that local branch or opened in a worktree on one that tracks it.
///
/// Feature: branch.open
#[must_use]
pub fn open(input: &OpenBranchInput<'_>) -> Option<(FeatureId, Target)> {
    let (local, target, checkout) = match input.reference {
        OpenedReference::Branch(name) => {
            if input.state.checked_out_branch() == Some(name) {
                return None;
            }
            (name, Target::Branch(name.to_owned()), "branch.checkout")
        }
        OpenedReference::RemoteBranch(name) => (
            local_name(name, &input.state.remotes),
            Target::RemoteBranch(name.to_owned()),
            "branch.checkout-remote",
        ),
    };
    if let Some(worktree) = other_worktree(input.state, local) {
        return (!worktree.is_missing).then(|| (FeatureId("worktree.open"), Target::Branch(local.to_owned())));
    }
    let feature = match input.open_in {
        OpenIn::Checkout => checkout,
        OpenIn::Worktree => "worktree.open-in-worktree",
    };
    Some((FeatureId(feature), target))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::{OpenBranchInput, OpenedReference, open};
    use crate::CommitId;
    use crate::contract::{FeatureId, OpenIn, Target};
    use crate::state::{Head, RepositoryState, Worktree};

    fn state() -> RepositoryState {
        RepositoryState {
            head: Head {
                commit: CommitId::parse(&"a".repeat(40)),
                branch: Some(String::from("refs/heads/main")),
            },
            remotes: vec![String::from("origin")],
            ..RepositoryState::default()
        }
    }

    fn with_worktree(mut state: RepositoryState, branch: &str, is_missing: bool) -> RepositoryState {
        state.worktrees.push(Worktree {
            folder: PathBuf::from("app-feature"),
            commit: None,
            branch: Some(format!("refs/heads/{branch}")),
            is_main: false,
            is_current: false,
            is_missing,
            uncommitted: None,
        });
        state
    }

    fn opened(state: &RepositoryState, reference: OpenedReference<'_>, open_in: OpenIn) -> Option<(FeatureId, Target)> {
        open(&OpenBranchInput {
            state,
            reference,
            open_in,
        })
    }

    #[test]
    fn branch_open__does_nothing_on_the_branch_checked_out() {
        assert_eq!(
            opened(&state(), OpenedReference::Branch("main"), OpenIn::Checkout),
            None
        );
        assert_eq!(
            opened(&state(), OpenedReference::Branch("feature"), OpenIn::Checkout),
            Some((FeatureId("branch.checkout"), Target::Branch(String::from("feature"))))
        );
    }

    #[test]
    fn branch_open__opens_a_worktree_for_the_branch_when_set_to_worktree() {
        assert_eq!(
            opened(&state(), OpenedReference::Branch("feature"), OpenIn::Worktree),
            Some((
                FeatureId("worktree.open-in-worktree"),
                Target::Branch(String::from("feature"))
            ))
        );
    }

    #[test]
    fn branch_open__opens_the_worktree_that_has_the_branch_of_a_remote_branch() {
        let state = with_worktree(state(), "feature", false);
        for open_in in [OpenIn::Checkout, OpenIn::Worktree] {
            assert_eq!(
                opened(&state, OpenedReference::RemoteBranch("origin/feature"), open_in),
                Some((FeatureId("worktree.open"), Target::Branch(String::from("feature"))))
            );
        }
        assert_eq!(
            opened(&state, OpenedReference::RemoteBranch("origin/other"), OpenIn::Checkout),
            Some((
                FeatureId("branch.checkout-remote"),
                Target::RemoteBranch(String::from("origin/other"))
            ))
        );
    }

    #[test]
    fn branch_open__does_nothing_when_the_worktree_of_the_branch_is_missing() {
        let state = with_worktree(state(), "feature", true);
        assert_eq!(
            opened(&state, OpenedReference::Branch("feature"), OpenIn::Checkout),
            None
        );
        assert_eq!(
            opened(
                &state,
                OpenedReference::RemoteBranch("origin/feature"),
                OpenIn::Worktree
            ),
            None
        );
    }
}
