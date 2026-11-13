//! What a feature says it needs next, or what it has decided to do.

use super::{AnswerKey, Answers, Context, Plan, Question, Text};
use crate::CommitId;

/// The identifier of a feature, as its card is headed: `branch.create`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(pub &'static str);

/// An operation of the catalog: called again and again with what is known so far, it says what it needs next. It is
/// pure (rule R2.2): the same context and answers always give the same outcome.
pub type Operation = fn(&Context, &Answers) -> Outcome;

/// What an operation needs next, or what it has decided to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Ask the user something; the answer comes back under the question's key.
    Ask(Question),
    /// Read something more from the repository; the result comes back under the key given.
    Read(AnswerKey, Query),
    /// Do not run: why, in words for the user.
    Refuse(Refusal),
    /// Run this.
    Run(Plan),
}

/// Something a feature needs read from the repository before it can decide. The engine reads it and gives the result
/// back as an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// The last entries of the reflog of a reference, newest first: `HEAD` or `refs/heads/main`. The answer is
    /// [`super::Answer::Reflog`].
    Reflog {
        /// The reference, in full, or `HEAD`.
        reference: String,
        /// How many entries, at most.
        count: usize,
    },
    /// Whether the files of the working tree are as a commit has them (`git diff --quiet <commit>`). The answer is
    /// [`super::Answer::Flag`].
    FilesMatch {
        /// The commit.
        commit: CommitId,
    },
    /// The commit that a hash, a branch, a tag or anything else git understands stands for
    /// (`search.read-revision`). The answer is [`super::Answer::Commit`].
    Revision {
        /// The name, as typed.
        revision: String,
    },
    /// The newest entry of the engine's journal that still holds for this repository. The answer is
    /// [`super::Answer::Journal`].
    Journal,
    /// How many commits each of two revisions has that the other has not
    /// (`git rev-list --left-right --count <left>...<right> --`). The answer is [`super::Answer::Counts`].
    Counts {
        /// The revision counted on the left: `refs/heads/feature`.
        left: String,
        /// The revision counted on the right: `origin/feature`.
        right: String,
    },
    /// Whether a revision is an ancestor of another, or the same commit
    /// (`git merge-base --is-ancestor <ancestor> <descendant>`). The answer is [`super::Answer::Flag`].
    IsAncestor {
        /// The revision that may be the ancestor: `refs/heads/feature`.
        ancestor: String,
        /// The revision that may have it: `HEAD`.
        descendant: String,
    },
    /// The local branches merged into a revision, by their short names
    /// (`git branch --merged <into> --format=%(refname:short)`). The answer is [`super::Answer::Names`].
    MergedBranches {
        /// The revision: `HEAD`.
        into: String,
    },
    /// Whether a commit has a file (`git cat-file -e <revision>:<path>`): `.gitmodules`, to know whether it has
    /// submodules. The answer is [`super::Answer::Flag`].
    HasFile {
        /// The commit, by any name git understands: `HEAD`, `feature`.
        revision: String,
        /// The path of the file, from the top of the tree.
        path: String,
    },
    /// The commits of an upstream that `HEAD` does not have, as of the last fetch (`sync.read-overwritten`). The answer
    /// is [`super::Answer::Overwritten`].
    Overwritten {
        /// The upstream, in full: `refs/remotes/origin/main`.
        upstream: String,
    },
    /// The remotes with their URLs (`remote.read-remotes`). The answer is [`super::Answer::Remotes`].
    Remotes,
    /// The files of a stash, those of its untracked files that are there again, and the conflicts applying it onto
    /// `HEAD` would make (`stash.read-files`, `stash.preview`). The answer is [`super::Answer::Stash`].
    Stash {
        /// Its place in the list: 0 for `stash@{0}`.
        index: usize,
    },
    /// Who commits are made as, and which `user.*` keys the repository sets for itself (`identity.read`). The answer
    /// is [`super::Answer::Identity`].
    Identity,
    /// The files a merge of `theirs` into `ours` would leave in conflict (`merge.read-predicted-conflicts`). The answer
    /// is [`super::Answer::Conflicts`].
    PredictedConflicts {
        /// What the merge goes into; `HEAD` when none is given.
        ours: Option<String>,
        /// What is merged.
        theirs: String,
        /// The base to merge from, for a cherry-pick: the parent of the commit picked; none lets git find it.
        base: Option<String>,
    },
    /// Commits, each with its parents and its subject, in the order given
    /// (`git log --no-walk=unsorted <revision>… --`). The answer is [`super::Answer::Commits`].
    Commits {
        /// The commits, by any name git understands.
        revisions: Vec<String>,
    },
    /// The commits after a commit up to `HEAD`, newest first, each with its parents
    /// (`git log --topo-order <commit>..HEAD --`). The answer is [`super::Answer::Commits`].
    LineAfter {
        /// The commit the line starts after.
        commit: CommitId,
    },
    /// The whole message of a commit (`git log -1 --format=%B <commit> --`). The answer is
    /// [`super::Answer::Message`].
    Message {
        /// The commit.
        commit: CommitId,
    },
    /// A file in a commit, in the index and on disk (`diff.read-sides`), to check that a change is still where a Diff
    /// page showed it. The answer is [`super::Answer::FileVersions`].
    FileVersions {
        /// The path of the file from the top of the working tree, with `/`.
        path: String,
        /// The commit to read it in too: `HEAD`, or the one a page compares with; none for no commit.
        commit: Option<CommitId>,
    },
    /// The remote branches that have a commit (`rebase.read-pushed`). The answer is [`super::Answer::Names`].
    Pushed {
        /// The commit.
        commit: CommitId,
    },
    /// The commit a cherry-pick or a revert stopped on with nothing left to do (`cherry-pick.skip-empty`). The answer
    /// is [`super::Answer::EmptyStop`].
    EmptyStop,
    /// The parents of a commit, and the line of commits from it up to `HEAD` with their messages, when it is a single
    /// line without merges: what a rewrite makes again. The answer is [`super::Answer::Line`].
    Line {
        /// The commit.
        commit: CommitId,
    },
    /// The whole messages of commits (`git log -1 --format=%B <hash>`). The answer is [`super::Answer::Messages`], in
    /// the same order.
    Messages {
        /// The commits.
        commits: Vec<CommitId>,
    },
    /// The files a commit changes, both names of one renamed
    /// (`git diff-tree -r --no-commit-id --name-only --no-renames -z --root <hash>`). The answer is
    /// [`super::Answer::Names`].
    CommitFiles {
        /// The commit.
        commit: CommitId,
    },
}

/// Why a feature does not run, in words for the user, with what may be done instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The sentence: "This repository has no remote."
    pub message: Text,
    /// A feature offered in its place, with the words of its button: **Add remote…**.
    pub offer: Option<(FeatureId, Text)>,
}

impl Refusal {
    /// A refusal that offers nothing else.
    #[must_use]
    pub fn saying(message: &str) -> Self {
        Refusal {
            message: Text::from(message),
            offer: None,
        }
    }
}
