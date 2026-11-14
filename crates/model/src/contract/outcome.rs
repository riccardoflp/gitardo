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
    /// The last positions of a reference in its reflog, newest first, with their selectors and times
    /// (`reflog.read`). The answer is [`super::Answer::Positions`].
    ReflogPositions {
        /// The reference, in full, or `HEAD`.
        reference: String,
        /// How many positions, at most.
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
    /// The local and remote branches that have a commit, by their full names, without the `HEAD` of a remote
    /// (`branch.name-from-format`). The answer is [`super::Answer::Names`].
    Containing {
        /// The commit.
        commit: CommitId,
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
    /// Commits as one patch, oldest first, as `git am` takes it (`patch.read-format-patch`). The answer is
    /// [`super::Answer::Patch`].
    FormatPatch {
        /// The commits, in any order.
        commits: Vec<CommitId>,
    },
    /// What `git apply --check --3way` says of a patch (`patch.check`). The answer is [`super::Answer::PatchCheck`].
    PatchCheck {
        /// The patch, in a file or as its text.
        patch: crate::patch::PatchInput,
    },
    /// The bytes of a file. The answer is [`super::Answer::Contents`].
    FileContents {
        /// The file.
        path: std::path::PathBuf,
    },
    /// The text on the clipboard of the system, which the window reads, not git. The answer is
    /// [`super::Answer::Clipboard`].
    Clipboard,
    /// A file in the temporary folder of the system, with a name no other file has, not made yet. The answer is
    /// [`super::Answer::Path`].
    TemporaryFile {
        /// The extension of its name, without the dot: `patch`.
        extension: String,
    },
    /// The changes of a comparison as a patch that `git apply` takes, binary files included (`compare.read-patch`).
    /// The answer is [`super::Answer::Contents`].
    ComparePatch(Box<crate::compare::ComparisonTarget>),
    /// Which versions the index keeps of a file in conflict (`conflicts.read-files`). The answer is
    /// [`super::Answer::Stages`].
    ConflictStages {
        /// The path of the file from the top of the working tree.
        path: String,
    },
    /// The files in conflict, the first 200, each on disk and in the three versions of the index, for the rules that
    /// resolve conflicts (`conflicts.resolve-by-rules`). The answer is [`super::Answer::ConflictTexts`].
    ConflictTexts,
    /// The bytes of files, each none when it is not there (`review.write`). The answer is
    /// [`super::Answer::Files`], in the same order.
    FileTexts {
        /// The files.
        paths: Vec<std::path::PathBuf>,
    },
    /// The git folder of the repository of a folder, in full (`git -C <folder> rev-parse --absolute-git-dir`), where
    /// the clones of its submodules are kept (`submodule.remove`). The answer is [`super::Answer::Path`].
    GitFolder {
        /// The top folder of the repository.
        folder: std::path::PathBuf,
    },
    /// Whether something is at a path on disk: a file, or a folder with something in it (`submodule.add`). The answer
    /// is [`super::Answer::Flag`].
    HasFiles {
        /// The path.
        path: std::path::PathBuf,
    },
    /// What link and copy patterns take from a checkout, for worktrees each of a commit
    /// (`worktree-setup.read-what-patterns-take`). The answer is [`super::Answer::SetupTakes`], in the same order.
    SetupTakes(Vec<crate::worktree_setup::TakeRequest>),
    /// Whether each path is a folder, whether git tracks it or files in it, and how much of it git ignores
    /// (`worktree-setup.explain-change`). The answer is [`super::Answer::PathStatuses`], in the same order.
    PathStatuses {
        /// The paths from the top of the working tree, with `/`.
        paths: Vec<String>,
    },
    /// Every name a file had in the history of what is checked out, the name given first
    /// (`file-history.read-names`). The answer is [`super::Answer::Names`].
    FileNames {
        /// The name now, from the top of the working tree.
        path: String,
    },
    /// Where some lines of a file as the File page has it are in the commit checked out
    /// (`file-history.read-committed-lines`). The answer is [`super::Answer::CommittedLines`].
    CommittedLines(Box<crate::file_history::CommittedLinesQuery>),
    /// Who last changed each line of a file (`blame.read-file`). The answer is [`super::Answer::Blame`].
    Blame(Box<crate::blame::BlameQuery>),
    /// Whether something is at a path on disk, a file, a folder or a link (`worktree.create`). The answer is
    /// [`super::Answer::Flag`].
    PathExists {
        /// The path.
        path: std::path::PathBuf,
    },
    /// The GitHub token: the one kept in the credential store of the system, or, when allowed, the one git's credential
    /// helper knows for `github.com` (`github.token`). The answer is [`super::Answer::Token`].
    GitHubToken {
        /// Whether `git credential fill` may run.
        helper: crate::github::HelperAsked,
    },
    /// The models the AI provider lists, asked with the key given, else the one kept in the credential store of the
    /// system (`ai.choose-model`). The answer is [`super::Answer::AiModels`].
    AiModels {
        /// The settings of the provider.
        settings: crate::ai::AiSettings,
        /// The key just typed, when one was.
        key: Option<crate::ai::Key>,
    },
    /// The links in worktrees to folders outside them, which git on Windows would empty when it removes a worktree
    /// (`worktree.read-folder-links`), for every worktree named. The answer is [`super::Answer::Paths`].
    FolderLinks {
        /// The top folders of the worktrees, one for `worktree.remove`, those picked for `branch.clean-up`.
        worktrees: Vec<std::path::PathBuf>,
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
