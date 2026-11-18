//! What the reads of the timeline send, a page at a time: the next pages, the commits a window has, and which message
//! ends a read.

use git::read::graph::CommitPage;
use model::graph::Row;

use super::message::{JobId, Message};

/// The job whose worker has nothing more to say once this message came: every job that answers, but a read of the
/// timeline whose page has more pages after it.
#[must_use]
pub fn job_done(message: &Message) -> Option<JobId> {
    let follows = |result: &Result<CommitPage, String>| result.as_ref().is_ok_and(|page| page.follows);
    match message {
        Message::RepositoriesFound { job, .. }
        | Message::DetailsRead { job, .. }
        | Message::CommitPageRead { job, .. } => Some(*job),
        Message::RepositoryRead { job, result, .. } => {
            let follows = result
                .as_ref()
                .is_ok_and(|read| read.1.as_ref().is_some_and(|page| page.follows));
            (!follows).then_some(*job)
        }
        Message::TimelineRead { job, result } | Message::PagesRead { job, result } => {
            (!follows(result)).then_some(*job)
        }
        Message::CommitsRead { job, result } => {
            let follows = match result {
                Ok(MorePages::Appended { follows, .. }) => *follows,
                Ok(MorePages::Replaced(page)) => page.follows,
                Err(_) => false,
            };
            (!follows).then_some(*job)
        }
        Message::FileSearch { job, answer } => answer.is_last().then_some(*job),
        // Every other message is not the end of a job the workers keep a token for.
        _ => None,
    }
}

/// The next pages of the timeline, as a worker sends them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MorePages {
    /// The commits known are read again as they were: only the rows after them are sent.
    Appended {
        /// The rows after those known.
        rows: Vec<Row>,
        /// Whether the history has more.
        has_more: bool,
        /// Whether more pages of the same read follow.
        follows: bool,
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
