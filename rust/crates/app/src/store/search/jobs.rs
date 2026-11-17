//! The reads of the search, as the store asks for them: what a search of the whole history finds
//! (`search.read-matches`), the commit of a hash or a name (`search.read-revision`) and where a commit is in the history
//! the timeline lists (`graph.read-position`). A worker runs them ([`crate::work::jobs`]).

use std::path::PathBuf;

use git::read::graph::PositionRequest;
use git::read::search::{MatchesRequest, RevisionRequest};
use model::CommitId;
use model::search::Matches;

use super::super::message::{Job, JobId, JobKind};
use super::super::state::State;
use super::super::timeline::{commits_request, read_request};
use super::Asked;

/// A read of the search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchJob {
    /// The commits a search of the whole history finds.
    Matches {
        /// The folder of the repository.
        folder: PathBuf,
        /// The search.
        request: MatchesRequest,
    },
    /// The commit a hash or a name stands for.
    Revision {
        /// The folder of the repository.
        folder: PathBuf,
        /// The name.
        request: RevisionRequest,
    },
    /// How many commits come before a commit in the history the timeline lists.
    Position {
        /// The folder of the repository.
        folder: PathBuf,
        /// The history and the commit.
        request: PositionRequest,
    },
}

impl SearchJob {
    /// The folder of the repository read.
    #[must_use]
    pub fn folder(&self) -> &PathBuf {
        match self {
            SearchJob::Matches { folder, .. }
            | SearchJob::Revision { folder, .. }
            | SearchJob::Position { folder, .. } => folder,
        }
    }
}

/// What a read of the search gives, or what git said when it failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchResult {
    /// The commits found.
    Matches(Result<Matches, String>),
    /// The commit of the name: none when git knows none.
    Revision(Result<Option<CommitId>, String>),
    /// How many commits come before the commit: none when it is not in the history listed.
    Position(Result<Option<usize>, String>),
}

/// The search of the whole history asked, over the history the timeline lists: the branches shown, or the history of
/// a commit alone.
#[must_use]
pub fn matches_request(state: &State, asked: &Asked) -> Option<SearchJob> {
    let read = state.shown_read()?;
    let listed = commits_request(&read.state, &read_request(state)).history;
    Some(SearchJob::Matches {
        folder: read.state.folder.clone(),
        request: MatchesRequest {
            mode: asked.mode,
            text: asked.text.clone(),
            selection: listed.selection,
        },
    })
}

/// The commit a hash or a name stands for, in the repository shown.
#[must_use]
pub fn revision_request(state: &State, revision: &str) -> Option<SearchJob> {
    let read = state.shown_read()?;
    Some(SearchJob::Revision {
        folder: read.state.folder.clone(),
        request: RevisionRequest {
            revision: revision.to_owned(),
        },
    })
}

/// Where a commit is in the history the timeline lists now, a search shown as a list included.
#[must_use]
pub fn position_request(state: &State, commit: &CommitId) -> Option<SearchJob> {
    let read = state.shown_read()?;
    let request = read_request(state);
    Some(SearchJob::Position {
        folder: read.state.folder.clone(),
        request: PositionRequest {
            history: commits_request(&read.state, &request).history,
            commit: commit.clone(),
        },
    })
}

/// Notes a read of the search as started and gives it its number.
pub fn start(state: &mut State, job: SearchJob) -> Job {
    let id = JobId(state.next_job);
    state.next_job += 1;
    let kind = JobKind::Search(job);
    let _new = state.jobs.insert(id, kind.clone());
    Job { id, kind }
}
