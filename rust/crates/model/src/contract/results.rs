//! The results of the queries of remotes, sync and stashes, as a feature reads them from its answers.

use super::{Answer, AnswerKey, Answers};
use crate::CommitId;
use crate::commit::Commit;
use crate::rebase::LineToHead;
use crate::remote::{Overwritten, Remote};
use crate::stash::StashReading;
use crate::state::Operation;

/// A cherry-pick or a revert stopped on a commit whose changes are already there: no file is left in conflict and the
/// tracked files are as `HEAD` has them, so that skipping it loses nothing (`cherry-pick.skip-empty`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyStop {
    /// `Operation::CherryPick` or `Operation::Revert`.
    pub operation: Operation,
    /// The commit it stopped on.
    pub commit: CommitId,
}

impl Answers {
    /// The commits a force push would throw away, read under a key.
    #[must_use]
    pub fn overwritten(&self, key: AnswerKey) -> Option<&Overwritten> {
        match self.get(key)? {
            Answer::Overwritten(overwritten) => Some(overwritten),
            _ => None, // Any other kind of answer is no list of commits.
        }
    }

    /// The remotes read under a key.
    #[must_use]
    pub fn remotes(&self, key: AnswerKey) -> Option<&[Remote]> {
        match self.get(key)? {
            Answer::Remotes(remotes) => Some(remotes),
            _ => None, // Any other kind of answer is no list of remotes.
        }
    }

    /// The versions of a file read under a key.
    #[must_use]
    pub fn file_versions(&self, key: AnswerKey) -> Option<&crate::diff::FileVersions> {
        match self.get(key)? {
            Answer::FileVersions(versions) => Some(versions),
            _ => None, // Any other kind of answer is no versions of a file.
        }
    }

    /// What was read of a stash under a key.
    #[must_use]
    pub fn stash(&self, key: AnswerKey) -> Option<&StashReading> {
        match self.get(key)? {
            Answer::Stash(reading) => Some(reading),
            _ => None, // Any other kind of answer is no reading of a stash.
        }
    }

    /// The files a merge would leave in conflict, read under a key: `None` when it was not read yet, `Some(None)` when
    /// git cannot tell.
    #[must_use]
    pub fn conflicts(&self, key: AnswerKey) -> Option<Option<&[String]>> {
        match self.get(key)? {
            Answer::Conflicts(paths) => Some(paths.as_deref()),
            _ => None, // Any other kind of answer is no prediction.
        }
    }

    /// The commits read under a key.
    #[must_use]
    pub fn commits(&self, key: AnswerKey) -> Option<&[Commit]> {
        match self.get(key)? {
            Answer::Commits(commits) => Some(commits),
            _ => None, // Any other kind of answer is no list of commits.
        }
    }

    /// The message of a commit read under a key.
    #[must_use]
    pub fn message(&self, key: AnswerKey) -> Option<&str> {
        match self.get(key)? {
            Answer::Message(message) => Some(message),
            _ => None, // Any other kind of answer is no message.
        }
    }

    /// The stop with nothing left to do read under a key: `None` when it was not read yet, `Some(None)` when there is
    /// none.
    #[must_use]
    pub fn empty_stop(&self, key: AnswerKey) -> Option<Option<&EmptyStop>> {
        match self.get(key)? {
            Answer::EmptyStop(stop) => Some(stop.as_ref()),
            _ => None, // Any other kind of answer is no stop.
        }
    }

    /// The line of a commit up to `HEAD`, read under a key.
    #[must_use]
    pub fn line(&self, key: AnswerKey) -> Option<&LineToHead> {
        match self.get(key)? {
            Answer::Line(line) => Some(line),
            _ => None, // Any other kind of answer is no line.
        }
    }

    /// The version of git read under a key: `Some(None)` when git said something of another form, which a feature
    /// takes as a git too old for it.
    #[must_use]
    pub fn git_version(&self, key: AnswerKey) -> Option<Option<&crate::rebase::GitVersion>> {
        match self.get(key)? {
            Answer::GitVersion(version) => Some(version.as_ref()),
            _ => None, // Any other kind of answer is no version.
        }
    }

    /// The branches of a stack read under a key.
    #[must_use]
    pub fn stacked(&self, key: AnswerKey) -> Option<&crate::rebase::StackedBranches> {
        match self.get(key)? {
            Answer::Stacked(stacked) => Some(stacked),
            _ => None, // Any other kind of answer is no stack.
        }
    }

    /// The messages read under a key.
    #[must_use]
    pub fn messages(&self, key: AnswerKey) -> Option<&[String]> {
        match self.get(key)? {
            Answer::Messages(messages) => Some(messages),
            _ => None, // Any other kind of answer is no messages.
        }
    }
}
