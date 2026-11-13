//! The results of the queries of remotes, sync and stashes, as a feature reads them from its answers.

use super::{Answer, AnswerKey, Answers};
use crate::remote::{Overwritten, Remote};
use crate::stash::StashReading;

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
}
