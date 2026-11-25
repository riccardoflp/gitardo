//! The history the worktrees of a repository share (`repo.share-history`): one shown for the first time takes the
//! rows another of them has read, when the history asked is the same.

use std::path::Path;

use super::group::same_folder;
use crate::filter::BranchesShown;
use crate::graph::Row;
use crate::state::{RepositoryState, Worktree};

/// The history a worktree shows, as it was read, and the worktree that asks for it.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRead<'a, Asked> {
    /// What its rows were asked with: the branches shown, the search and the order.
    pub asked: &'a Asked,
    /// The branches shown among what was asked; none is everything.
    pub shown: Option<&'a BranchesShown>,
    /// The rows read, without the row of the uncommitted changes.
    pub rows: &'a [Row],
    /// Whether the history has more commits than those rows.
    pub has_more: bool,
    /// The state of the worktree they were read in.
    pub state: &'a RepositoryState,
    /// The folder of the worktree that is shown in its place.
    pub chosen: &'a Path,
}

/// What a worktree takes from another of its repository, to show at once.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedHistory {
    /// The rows.
    pub rows: Vec<Row>,
    /// Whether the history has more commits than those rows.
    pub has_more: bool,
    /// The state of the worktree left, with its references, in the folder of the worktree chosen and with that
    /// worktree marked as the one shown: what is checked out, what is not committed and the rest are those of the
    /// worktree left until the worktree chosen is read (Q1743).
    pub state: RepositoryState,
}

/// The rows and the references a worktree may take from another of its repository that has its commits read: none
/// when the history asked differs, by the branches shown, the search or the order; none when the timeline follows
/// what is checked out (**Current**, **Upstream**), which is not the same in two worktrees; none when the folder
/// chosen is not a worktree of that repository whose folder is there, as a submodule, another repository.
///
/// Feature: repo.share-history
#[must_use]
pub fn shared_history<Asked: PartialEq>(read: &HistoryRead<'_, Asked>, asked: &Asked) -> Option<SharedHistory> {
    if read.asked != asked || matches!(read.shown, Some(BranchesShown::Current | BranchesShown::Upstream)) {
        return None;
    }
    let is_chosen = |worktree: &Worktree| same_folder(&worktree.folder, read.chosen);
    let chosen = read.state.worktrees.iter().find(|worktree| is_chosen(worktree))?;
    if chosen.is_missing || chosen.is_current {
        return None;
    }
    let worktrees = read.state.worktrees.iter().map(|worktree| Worktree {
        is_current: is_chosen(worktree),
        ..worktree.clone()
    });
    Some(SharedHistory {
        rows: read.rows.to_vec(),
        has_more: read.has_more,
        state: RepositoryState {
            folder: chosen.folder.clone(),
            worktrees: worktrees.collect(),
            ..read.state.clone()
        },
    })
}

/// Whether the commit checked out is one a reference the timeline lists is at, so that listing it adds no commit to
/// the history of the references: a branch, a remote branch or a tag among the kinds shown. With references ticked
/// one by one what is checked out is not listed at all; with **Current** and **Upstream** it is the history itself.
#[must_use]
pub fn is_on_reference(state: &RepositoryState, shown: Option<&BranchesShown>) -> bool {
    let (local, remote, tags) = match shown {
        None => (true, true, true),
        Some(BranchesShown::Kinds(kinds)) => (kinds.local, kinds.remote, kinds.tags),
        Some(BranchesShown::References(_)) => return true,
        Some(BranchesShown::Current | BranchesShown::Upstream) => return false,
    };
    let Some(head) = &state.head.commit else {
        return false;
    };
    (local && state.branches.iter().any(|branch| &branch.commit == head))
        || (remote && state.remote_branches.iter().any(|branch| &branch.commit == head))
        || (tags && state.tags.iter().any(|tag| &tag.commit == head))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{HistoryRead, shared_history};
    use crate::filter::BranchesShown;
    use crate::state::{RepositoryState, Worktree};

    /// `api`, shown, with its worktree `api-fix` and a worktree whose folder is gone.
    fn api() -> RepositoryState {
        let worktree = |folder: &str, is_current: bool, is_missing: bool| Worktree {
            folder: PathBuf::from(folder),
            commit: None,
            branch: None,
            is_main: is_current,
            is_current,
            is_missing,
            locked: None,
            uncommitted: None,
        };
        RepositoryState {
            folder: PathBuf::from("/work/api"),
            worktrees: vec![
                worktree("/work/api", true, false),
                worktree("/work/api-fix", false, false),
                worktree("/work/api-gone", false, true),
            ],
            ..RepositoryState::default()
        }
    }

    /// What `api` gives a folder chosen, its rows asked with `left` and the folder asking `asked`.
    fn offered(chosen: &str, left: &str, asked: &str, shown: Option<&BranchesShown>) -> Option<PathBuf> {
        let state = api();
        let read = HistoryRead {
            asked: &left,
            shown,
            rows: &[],
            has_more: false,
            state: &state,
            chosen: Path::new(chosen),
        };
        let shared = shared_history(&read, &asked)?;
        let current: Vec<&Worktree> = shared
            .state
            .worktrees
            .iter()
            .filter(|worktree| worktree.is_current)
            .collect();
        assert_eq!(current.len(), 1, "one worktree is the one shown");
        assert_eq!(
            current.first().map(|worktree| &worktree.folder),
            Some(&shared.state.folder)
        );
        Some(shared.state.folder)
    }

    #[test]
    fn repo_share_history__a_worktree_of_the_repository_takes_its_history_when_it_asks_the_same() {
        let taken = offered("/work/api-fix", "every branch", "every branch", None);
        assert_eq!(taken, Some(PathBuf::from("/work/api-fix")));
        assert_eq!(
            offered("/work/api-fix", "main", "every branch", None),
            None,
            "another history"
        );
    }

    #[test]
    fn repo_share_history__a_timeline_that_follows_what_is_checked_out_is_not_shared() {
        for shown in [BranchesShown::Current, BranchesShown::Upstream] {
            assert_eq!(offered("/work/api-fix", "same", "same", Some(&shown)), None);
        }
        let ticked = BranchesShown::References(vec![String::from("refs/heads/main")]);
        assert!(offered("/work/api-fix", "same", "same", Some(&ticked)).is_some());
    }

    #[test]
    fn repo_share_history__a_folder_that_is_no_worktree_of_the_repository_takes_nothing() {
        assert_eq!(offered("/work/api/lib/core", "same", "same", None), None, "a submodule");
        assert_eq!(
            offered("/work/api-gone", "same", "same", None),
            None,
            "a worktree whose folder is gone"
        );
        assert_eq!(
            offered("/work/api", "same", "same", None),
            None,
            "the worktree shown itself"
        );
    }
}
