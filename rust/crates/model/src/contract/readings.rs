//! The results of the queries of patches, files and comparisons, as a feature reads them from its answers.

use std::path::Path;

use super::{Answer, AnswerKey, Answers};
use crate::compare::RecentComparison;
use crate::patch::PatchCheck;

impl Answers {
    /// The patch made by `git format-patch`, read under a key.
    #[must_use]
    pub fn patch(&self, key: AnswerKey) -> Option<&str> {
        match self.get(key)? {
            Answer::Patch(patch) => Some(patch),
            _ => None, // Any other kind of answer is no patch.
        }
    }

    /// What applying a patch would do, read under a key.
    #[must_use]
    pub fn patch_check(&self, key: AnswerKey) -> Option<&PatchCheck> {
        match self.get(key)? {
            Answer::PatchCheck(check) => Some(check),
            _ => None, // Any other kind of answer is no check of a patch.
        }
    }

    /// The bytes read under a key.
    #[must_use]
    pub fn contents(&self, key: AnswerKey) -> Option<&[u8]> {
        match self.get(key)? {
            Answer::Contents(contents) => Some(contents),
            _ => None, // Any other kind of answer is no contents.
        }
    }

    /// The bytes of files read under a key, each none when it is not there.
    #[must_use]
    pub fn files(&self, key: AnswerKey) -> Option<&[Option<Vec<u8>>]> {
        match self.get(key)? {
            Answer::Files(files) => Some(files),
            _ => None, // Any other kind of answer is no files.
        }
    }

    /// The text of the clipboard read under a key: `None` when it was not read yet, `Some(None)` when it holds no text.
    #[must_use]
    pub fn clipboard(&self, key: AnswerKey) -> Option<Option<&str>> {
        match self.get(key)? {
            Answer::Clipboard(text) => Some(text.as_deref()),
            _ => None, // Any other kind of answer is no clipboard.
        }
    }

    /// The path chosen or given under a key.
    #[must_use]
    pub fn path(&self, key: AnswerKey) -> Option<&Path> {
        match self.get(key)? {
            Answer::Path(path) => Some(path),
            _ => None, // Any other kind of answer is no path.
        }
    }

    /// The positions of a reference in its reflog, read under a key.
    #[must_use]
    pub fn positions(&self, key: AnswerKey) -> Option<&[crate::reflog::ReflogPosition]> {
        match self.get(key)? {
            Answer::Positions(positions) => Some(positions),
            _ => None, // Any other kind of answer is no list of positions.
        }
    }

    /// Where some lines are in the commit checked out, read under a key.
    #[must_use]
    pub fn committed_lines(&self, key: AnswerKey) -> Option<&crate::file_history::CommittedLines> {
        match self.get(key)? {
            Answer::CommittedLines(lines) => Some(lines),
            _ => None, // Any other kind of answer is no lines.
        }
    }

    /// The blame of a file read under a key: `None` when it was not read yet, `Some(None)` when git has no blame for
    /// the file.
    #[must_use]
    pub fn blame(&self, key: AnswerKey) -> Option<Option<&[Option<crate::blame::BlameLine>]>> {
        match self.get(key)? {
            Answer::Blame(lines) => Some(lines.as_deref()),
            _ => None, // Any other kind of answer is no blame.
        }
    }

    /// The comparisons opened before, given under a key.
    #[must_use]
    pub fn recent_comparisons(&self, key: AnswerKey) -> Option<&[RecentComparison]> {
        match self.get(key)? {
            Answer::Recent(recent) => Some(recent),
            _ => None, // Any other kind of answer is no list of comparisons.
        }
    }
}
