//! What the two sides of a conflict are (`conflicts.side-names`): the words that say what **Current** and **Incoming**
//! are in the operation that stopped, the branch or commit each one is, and the subject of its commit.

#[cfg(test)]
mod tests;

use crate::CommitId;
use crate::state::Operation;

/// A commit as the reads find it: its hash, the references that point at it, and its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamedCommit<'a> {
    /// Its hash.
    pub id: &'a CommitId,
    /// The branches and remote branches at it, by full name, as `git for-each-ref --points-at=<hash>
    /// --format=%(refname) refs/heads refs/remotes` lists them: `refs/heads/main`, `refs/remotes/origin/main`.
    pub references: &'a [String],
    /// The subject of its message, `git log -1 --format=%s`.
    pub subject: Option<&'a str>,
}

/// What the reads found of the operation that stopped, or of the stash applied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideNamesInput<'a> {
    /// The operation in progress; none when the conflicts come from a stash applied.
    pub operation: Option<Operation>,
    /// The branch checked out, by its short name (`git symbolic-ref --short -q HEAD`); none when `HEAD` is detached.
    pub checked_out: Option<&'a str>,
    /// What `HEAD` is.
    pub head: Option<NamedCommit<'a>>,
    /// The commit the operation brings in: `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` or `REBASE_HEAD`.
    pub brought_in: Option<NamedCommit<'a>>,
    /// In a rebase, the commit of `onto` (`.git/rebase-merge/onto` or `.git/rebase-apply/onto`).
    pub onto: Option<NamedCommit<'a>>,
    /// In a rebase, the branch being rebased, as `head-name` says it: `refs/heads/topic`, or `detached HEAD`.
    pub rebased_branch: Option<&'a str>,
    /// In `git am`, the first line of the message of the patch (`.git/rebase-apply/final-commit`).
    pub patch_subject: Option<&'a str>,
}

/// What a side is in the operation that stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideRole {
    /// The branch checked out: the current side of a merge, a cherry-pick, a revert and `git am`.
    BranchCheckedOut,
    /// The branch being merged.
    BranchBeingMerged,
    /// What the branch is being rebased onto: the current side of a rebase.
    RebasedOnto,
    /// Your commit being replayed: the incoming side of a rebase.
    CommitBeingReplayed,
    /// The commit being picked.
    CommitBeingPicked,
    /// The state before the reverted commit.
    StateBeforeReverted,
    /// The patch being applied by `git am`.
    PatchBeingApplied,
    /// Your files: the current side when a stash is applied.
    YourFiles,
    /// The changes being applied from a stash.
    ChangesBeingApplied,
}

impl SideRole {
    /// The words that say it, in sentence case.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            SideRole::BranchCheckedOut => "The branch checked out",
            SideRole::BranchBeingMerged => "The branch being merged",
            SideRole::RebasedOnto => "What the branch is being rebased onto",
            SideRole::CommitBeingReplayed => "Your commit being replayed",
            SideRole::CommitBeingPicked => "The commit being picked",
            SideRole::StateBeforeReverted => "The state before the reverted commit",
            SideRole::PatchBeingApplied => "The patch being applied",
            SideRole::YourFiles => "Your files",
            SideRole::ChangesBeingApplied => "The changes being applied",
        }
    }
}

/// One side of the conflicts: what it is, the branch or commit it is, and the subject of its commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    /// What it is in the operation.
    pub role: SideRole,
    /// The branch, or the short hash of the commit; none for a patch and for the sides of a stash.
    pub name: Option<String>,
    /// The subject of its commit, or the first line of the message of a patch.
    pub subject: Option<String>,
}

/// What the two sides of the conflicts are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideNames {
    /// The current side.
    pub current: Side,
    /// The incoming side.
    pub incoming: Side,
}

/// What the two sides of the conflicts are in the operation that stopped.
///
/// The current side is the branch checked out, else a branch at `HEAD`, else the short hash of `HEAD`; in a rebase it
/// is the branch at the commit of `onto`, else its short hash. The incoming side of a merge is a branch at
/// `MERGE_HEAD`, else its short hash; of a cherry-pick or a revert, the short hash of the commit brought in; of a rebase,
/// the branch being rebased, else the name of `REBASE_HEAD`. A patch and a stash have no name. A branch at a commit is
/// the first one listed that does not end in `/HEAD`, by its short name.
///
/// Feature: conflicts.side-names
#[must_use]
pub fn side_names(input: &SideNamesInput<'_>) -> SideNames {
    let subject = |commit: Option<NamedCommit<'_>>| commit.and_then(|commit| commit.subject).map(str::to_owned);
    let checked_out = || {
        input
            .checked_out
            .map(str::to_owned)
            .or_else(|| input.head.as_ref().map(name))
    };
    let side = |role, name: Option<String>, subject: Option<String>| Side { role, name, subject };
    let current = |role| side(role, checked_out(), subject(input.head));
    let brought_in = |role, name: fn(&NamedCommit<'_>) -> String| {
        side(role, input.brought_in.as_ref().map(name), subject(input.brought_in))
    };
    let (current, incoming) = match input.operation {
        Some(Operation::Merge) => (
            current(SideRole::BranchCheckedOut),
            brought_in(SideRole::BranchBeingMerged, name),
        ),
        Some(Operation::CherryPick) => (
            current(SideRole::BranchCheckedOut),
            brought_in(SideRole::CommitBeingPicked, short_hash),
        ),
        Some(Operation::Revert) => (
            current(SideRole::BranchCheckedOut),
            brought_in(SideRole::StateBeforeReverted, short_hash),
        ),
        Some(Operation::Patch) => (
            current(SideRole::BranchCheckedOut),
            side(
                SideRole::PatchBeingApplied,
                None,
                input.patch_subject.map(str::to_owned),
            ),
        ),
        Some(Operation::Rebase) => {
            let rebased = input
                .rebased_branch
                .and_then(|branch| branch.strip_prefix("refs/heads/"))
                .map(str::to_owned)
                .or_else(|| input.brought_in.as_ref().map(name));
            (
                side(
                    SideRole::RebasedOnto,
                    input.onto.as_ref().map(name),
                    subject(input.head),
                ),
                side(SideRole::CommitBeingReplayed, rebased, subject(input.brought_in)),
            )
        }
        None => (
            side(SideRole::YourFiles, None, None),
            side(SideRole::ChangesBeingApplied, None, None),
        ),
    };
    SideNames { current, incoming }
}

/// A commit by a branch at it, else by its short hash.
fn name(commit: &NamedCommit<'_>) -> String {
    commit
        .references
        .iter()
        .filter(|reference| !reference.ends_with("/HEAD"))
        .find_map(|reference| {
            reference
                .strip_prefix("refs/heads/")
                .or_else(|| reference.strip_prefix("refs/remotes/"))
        })
        .map_or_else(|| short_hash(commit), str::to_owned)
}

/// A commit by its short hash, 7 characters.
fn short_hash(commit: &NamedCommit<'_>) -> String {
    commit.id.short().to_owned()
}
