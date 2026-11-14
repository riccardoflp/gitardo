//! What dropping a commit or a branch on a row of the timeline offers.

use crate::CommitId;
use crate::contract::FeatureId;

/// What is dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dragged<'a> {
    /// A commit.
    Commit(&'a CommitId),
    /// A branch, local or remote.
    Branch {
        /// Its short name: `feature`, `origin/feature`.
        name: &'a str,
        /// The commit it points to.
        tip: &'a CommitId,
        /// Whether it is a remote branch.
        is_remote: bool,
        /// Whether another worktree has it checked out.
        is_checked_out_elsewhere: bool,
    },
}

/// The row it is dropped on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropRow<'a> {
    /// The uncommitted changes at the top of the timeline.
    Draft,
    /// A commit.
    Commit(&'a CommitId),
}

/// What [`list_drop_entries`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropInput<'a> {
    /// What is dragged.
    pub dragged: Dragged<'a>,
    /// Where it is dropped.
    pub row: DropRow<'a>,
    /// The commit checked out; none before the first commit.
    pub head: Option<&'a CommitId>,
    /// The branch checked out; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// Whether a merge, rebase, cherry-pick, revert or patch is under way.
    pub is_operation_under_way: bool,
}

/// An entry a drop offers: the operation, and its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropEntry {
    /// The operation it runs.
    pub feature: FeatureId,
    /// What the entry says.
    pub label: String,
}

impl DropEntry {
    /// An entry.
    fn new(feature: &'static str, label: String) -> DropEntry {
        DropEntry {
            feature: FeatureId(feature),
            label,
        }
    }
}

/// What dropping something on a row offers; an empty list when the row does not accept it.
///
/// A commit onto the row checked out (its commit, or the draft) offers to cherry-pick it; a branch onto it offers to
/// merge it or rebase onto it; the branch checked out onto another commit offers to rebase or reset it there; another
/// local branch onto another commit offers to rebase it there. Nothing is accepted before the first commit or while an
/// operation is under way.
///
/// Feature: drag.list-drop-entries
#[must_use]
pub fn list_drop_entries(input: &DropInput<'_>) -> Vec<DropEntry> {
    let Some(head) = input.head.filter(|_| !input.is_operation_under_way) else {
        return Vec::new();
    };
    let current = input.branch.unwrap_or("HEAD");
    let on_checked_out = match input.row {
        DropRow::Draft => true,
        DropRow::Commit(commit) => commit == head,
    };
    match input.dragged {
        Dragged::Commit(commit) if on_checked_out && commit != head => vec![DropEntry::new(
            "cherry-pick.commit",
            format!("Cherry-pick `{}` onto `{current}`", commit.short()),
        )],
        Dragged::Commit(_) => Vec::new(),
        Dragged::Branch { name, tip, .. } if on_checked_out => {
            if Some(name) == input.branch || tip == head {
                return Vec::new();
            }
            vec![
                DropEntry::new("merge.into-current", format!("Merge `{name}` into `{current}`")),
                DropEntry::new("merge.choose-way", format!("Merge `{name}` into `{current}`…")),
                DropEntry::new("rebase.onto", format!("Rebase `{current}` onto `{name}`")),
            ]
        }
        Dragged::Branch {
            name,
            tip,
            is_remote,
            is_checked_out_elsewhere,
        } => {
            let DropRow::Commit(target) = input.row else {
                return Vec::new();
            };
            if is_remote || target == tip {
                Vec::new()
            } else if Some(name) == input.branch {
                vec![
                    DropEntry::new("rebase.onto", format!("Rebase `{name}` onto `{}`", target.short())),
                    DropEntry::new("reset.to-commit", format!("Reset `{name}` to `{}`…", target.short())),
                ]
            } else if is_checked_out_elsewhere {
                Vec::new()
            } else {
                vec![DropEntry::new(
                    "rebase.branch-onto",
                    format!("Rebase `{name}` onto `{}`", target.short()),
                )]
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Dragged, DropInput, DropRow, list_drop_entries};
    use crate::CommitId;

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
    }

    /// `main` checked out at `1…`; `feature` at `2…`; another commit `3…`.
    fn input<'a>(dragged: Dragged<'a>, row: DropRow<'a>, head: &'a CommitId) -> DropInput<'a> {
        DropInput {
            dragged,
            row,
            head: Some(head),
            branch: Some("main"),
            is_operation_under_way: false,
        }
    }

    fn labels(input: &DropInput<'_>) -> Vec<String> {
        list_drop_entries(input).into_iter().map(|entry| entry.label).collect()
    }

    fn branch<'a>(name: &'a str, tip: &'a CommitId) -> Dragged<'a> {
        Dragged::Branch {
            name,
            tip,
            is_remote: false,
            is_checked_out_elsewhere: false,
        }
    }

    #[test]
    fn drag_list_drop_entries__a_commit_over_a_row_not_checked_out_gives_nothing() {
        let (head, other, third) = (id('1'), id('2'), id('3'));
        assert_eq!(
            labels(&input(Dragged::Commit(&other), DropRow::Commit(&third), &head)),
            Vec::<String>::new()
        );
        assert_eq!(
            labels(&input(Dragged::Commit(&other), DropRow::Draft, &head)),
            ["Cherry-pick `2222222` onto `main`"]
        );
        assert_eq!(
            labels(&input(Dragged::Commit(&head), DropRow::Draft, &head)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn drag_list_drop_entries__a_branch_onto_the_commit_checked_out_gives_the_merges_then_the_rebase() {
        let (head, tip) = (id('1'), id('2'));
        assert_eq!(
            labels(&input(branch("feature", &tip), DropRow::Commit(&head), &head)),
            [
                "Merge `feature` into `main`",
                "Merge `feature` into `main`…",
                "Rebase `main` onto `feature`"
            ]
        );
        let remote = Dragged::Branch {
            name: "origin/feature",
            tip: &tip,
            is_remote: true,
            is_checked_out_elsewhere: false,
        };
        assert_eq!(labels(&input(remote, DropRow::Commit(&head), &head)).len(), 3);
        let other = id('3');
        assert_eq!(
            labels(&input(remote, DropRow::Commit(&other), &head)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn drag_list_drop_entries__the_branch_checked_out_onto_a_commit_gives_the_rebase_and_the_reset() {
        let (head, target, tip) = (id('1'), id('a'), id('2'));
        assert_eq!(
            labels(&input(branch("main", &head), DropRow::Commit(&target), &head)),
            ["Rebase `main` onto `aaaaaaa`", "Reset `main` to `aaaaaaa`…"]
        );
        assert_eq!(
            labels(&input(branch("feature", &tip), DropRow::Commit(&target), &head)),
            ["Rebase `feature` onto `aaaaaaa`"]
        );
        assert_eq!(
            labels(&input(branch("feature", &tip), DropRow::Commit(&tip), &head)),
            Vec::<String>::new()
        );
        let elsewhere = Dragged::Branch {
            name: "feature",
            tip: &tip,
            is_remote: false,
            is_checked_out_elsewhere: true,
        };
        assert_eq!(
            labels(&input(elsewhere, DropRow::Commit(&target), &head)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn drag_list_drop_entries__any_drag_while_a_rebase_is_stopped_gives_nothing() {
        let (head, tip) = (id('1'), id('2'));
        let stopped = DropInput {
            is_operation_under_way: true,
            ..input(branch("feature", &tip), DropRow::Commit(&head), &head)
        };
        assert_eq!(labels(&stopped), Vec::<String>::new());
        let empty = DropInput {
            head: None,
            ..input(Dragged::Commit(&tip), DropRow::Draft, &head)
        };
        assert_eq!(labels(&empty), Vec::<String>::new());
    }
}
