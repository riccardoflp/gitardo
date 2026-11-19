//! The other branches of a stack that a rewrite moved, and how one Undo puts them back with the branch rewritten
//! (`undo.moved-branches`).

use std::fmt::Write as _;

use crate::CommitId;
use crate::state::RepositoryState;

/// A branch a rewrite moved with the branch it rewrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedBranch {
    /// Its name, without `refs/heads/`.
    pub name: String,
    /// The commit it was on before the rewrite.
    pub before: CommitId,
    /// The commit the rewrite left it on, once it ran; none before.
    pub after: Option<CommitId>,
}

/// What Undo does with the branches a rewrite moved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MovedBranchesUndo {
    /// What `git update-ref --stdin` reads: a line `update refs/heads/<branch> <before> <after>` for each branch put
    /// back, which git refuses unless the branch is still at `<after>`. Empty when no branch goes back.
    pub lines: String,
    /// The branches put back.
    pub back: Vec<String>,
    /// The branches that moved since the rewrite, which stay where they are.
    pub moved_since: Vec<String>,
}

/// The branches a rewrite moved that one Undo puts back, each still where the rewrite left it; and those moved since,
/// by a commit or anything else, which stay where they are. A branch the rewrite did not move, or that is gone, is
/// left out.
///
/// Feature: undo.moved-branches
#[must_use]
pub fn moved_branches(moved: &[MovedBranch], state: &RepositoryState) -> MovedBranchesUndo {
    let mut undo = MovedBranchesUndo::default();
    for branch in moved {
        let Some(after) = branch.after.as_ref().filter(|after| **after != branch.before) else {
            continue;
        };
        let Some(now) = state.branches.iter().find(|each| each.name == branch.name) else {
            continue;
        };
        if now.commit == *after {
            let _ = writeln!(
                undo.lines,
                "update refs/heads/{} {} {after}",
                branch.name, branch.before
            );
            undo.back.push(branch.name.clone());
        } else {
            undo.moved_since.push(branch.name.clone());
        }
    }
    undo
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MovedBranch, moved_branches};
    use crate::CommitId;
    use crate::state::{Branch, RepositoryState};

    fn commit(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap()
    }

    fn at(branches: &[(&str, char)]) -> RepositoryState {
        RepositoryState {
            branches: branches
                .iter()
                .map(|(name, digit)| Branch {
                    name: (*name).to_owned(),
                    commit: commit(*digit),
                    upstream: None,
                    is_checked_out: false,
                    date: crate::Timestamp::default(),
                })
                .collect(),
            ..RepositoryState::default()
        }
    }

    fn moved(name: &str, before: char, after: char) -> MovedBranch {
        MovedBranch {
            name: name.to_owned(),
            before: commit(before),
            after: Some(commit(after)),
        }
    }

    #[test]
    fn undo_moved_branches__each_branch_still_where_the_rewrite_left_it_goes_back_in_one_line() {
        let undo = moved_branches(
            &[moved("p1", '1', 'a'), moved("p2", '2', 'b')],
            &at(&[("p1", 'a'), ("p2", 'b')]),
        );
        assert_eq!(
            undo.lines,
            format!(
                "update refs/heads/p1 {} {}\nupdate refs/heads/p2 {} {}\n",
                commit('1'),
                commit('a'),
                commit('2'),
                commit('b')
            )
        );
        assert_eq!(undo.back, ["p1", "p2"]);
        assert_eq!(undo.moved_since, Vec::<String>::new());
    }

    #[test]
    fn undo_moved_branches__a_branch_moved_since_stays_and_is_named() {
        let undo = moved_branches(
            &[moved("p1", '1', 'a'), moved("p2", '2', 'b')],
            &at(&[("p1", 'c'), ("p2", 'b')]),
        );
        assert_eq!(undo.back, ["p2"]);
        assert_eq!(undo.moved_since, ["p1"]);
        assert!(!undo.lines.contains("p1"));
    }

    #[test]
    fn undo_moved_branches__nothing_when_no_branch_moved() {
        let not_run = MovedBranch {
            after: None,
            ..moved("p1", '1', '1')
        };
        let undo = moved_branches(&[not_run, moved("p2", '2', '2')], &at(&[("p1", '1'), ("p2", '2')]));
        assert_eq!(undo, super::MovedBranchesUndo::default());
    }
}
