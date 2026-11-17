//! What a worker sends of the next pages of the timeline, and what the window has of it to say which pages are next.

use git::read::graph::CommitPage;
use model::graph::Row;

/// The next pages of the timeline, as a worker sends them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MorePages {
    /// The commits known are read again as they were: only the rows after them are sent.
    Appended {
        /// The rows after those known.
        rows: Vec<Row>,
        /// Whether the history has more.
        has_more: bool,
    },
    /// The history changed under the commits known: every row read.
    Replaced(CommitPage),
}

/// The commits a window has of a timeline, as a worker checks them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Known {
    /// How many rows were read, the stashes among them.
    pub count: usize,
    /// How many of them are commits.
    pub commits: usize,
    /// The last commit read: the read of the next pages starts from it again, to tell whether the history is the
    /// same above it.
    pub last: Option<model::CommitId>,
}
