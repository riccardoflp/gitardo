//! Who last changed each line of a file, as `git blame` says it.

use std::sync::Arc;

mod age;
mod format;
mod lines;
mod target;

pub use age::{AgeInput, OLD_AFTER_DAYS, OLD_COLOR, RECENT_COLOR, age_color};
pub use format::{DEFAULT_FORMAT, FormatInput, NOT_COMMITTED, ago, format_line};
pub use lines::{LineEdit, PinnedInput, mark_edited_lines, pick_pinned_line};
pub use target::{BlameOfFile, BlameQuery, FileToOpen, LineOfFile, ShownBlame};

use crate::{CommitId, Timestamp};

/// Who last changed a line: a commit, or nobody yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlameLine {
    /// The line is not committed yet: it was typed, or changed, since the last commit.
    NotCommitted,
    /// The commit that last changed the line, shared by every line of that commit.
    Committed(Arc<BlameCommit>),
}

/// A commit that last changed some lines, as the blame describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameCommit {
    /// Its full hash.
    pub id: CommitId,
    /// Who wrote it.
    pub author: String,
    /// When its author wrote it.
    pub author_time: Timestamp,
    /// The first line of its message.
    pub summary: String,
    /// The commit before it for this file, and the name the file had there; none for the commit that added the file.
    pub previous: Option<PreviousVersion>,
}

/// The file as it was before a commit: the commit before it, and the name the file had there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousVersion {
    /// The commit before.
    pub commit: CommitId,
    /// The name the file had in it, from the top of the working tree.
    pub path: String,
}
