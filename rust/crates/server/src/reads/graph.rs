//! The reads of the graph, from stage 7: `graph.read-commits`, `graph.read-position` and `search.read-matches`, each
//! over the timeline the client shows, turned into git's arguments by `engine::timeline` as the desktop's are. From
//! version 1.8 the pages of the commits are sent as they are read when the client asks for them so (D54).

use engine::timeline::{TimelineRequest, commits_request};
use git::read::graph::{
    CommitPage, CommitsRequest, PositionRequest, READ_LIMIT, read_commits, read_commits_at_once, read_position,
};
use git::read::repo::{TimelineParts, read_timeline_parts};
use git::read::search::{MatchesRequest, read_matches};
use model::filter::BranchesShown;
use protocol::reads::{
    CommitPageJson, CommitsParams, GraphRowsParams, MatchesJson, MatchesParams, PositionParams, PositionResult,
    RowJson, TimelineJson, commit_of, methods,
};
use protocol::rpc::{ResponseError, notification_message};
use serde_json::Value;

use super::{Reading, answered, invalid};
use crate::session::{git_failure, json};

/// The rows of the pages of the timeline asked for, after those the client has (`graph.read-commits`). Asked with
/// `stream` (1.8), the read goes on to the end of the history or to 100,000 commits after those, and each page is sent
/// as it is read.
pub fn commits(reading: &Reading<'_>, params: &CommitsParams) -> Result<Value, ResponseError> {
    let pages = TimelineRequest::try_from(&params.timeline).map_err(invalid)?.pages;
    let request = CommitsRequest {
        skip: params.timeline.skip,
        ..commits_of(reading, &params.timeline)?
    };
    if params.stream {
        return streamed(
            reading,
            &CommitsRequest {
                count: READ_LIMIT,
                ..request
            },
        );
    }
    answered(read_commits_at_once(&reading.git, &request.with_pages(pages)), |page| {
        CommitPageJson::from(page)
    })
}

/// Reads the commits as git prints them, sends each page that has rows in a notification `read/graph.rows` named by
/// the id of the request, and answers with no rows once the read ends, saying whether the history has more. The
/// notifications go out before the answer, on the same output.
fn streamed(reading: &Reading<'_>, request: &CommitsRequest) -> Result<Value, ResponseError> {
    let mut send = |page: CommitPage| {
        if page.rows.is_empty() {
            return;
        }
        let params = GraphRowsParams {
            read: reading.id.clone(),
            rows: page.rows.iter().map(RowJson::from).collect(),
        };
        if let Ok(params) = json(&params) {
            reading.session.send(notification_message(methods::GRAPH_ROWS, params));
        }
    };
    let end = read_commits(&reading.git, request, &mut send).map_err(|error| git_failure(&error))?;
    json(&CommitPageJson {
        rows: Vec::new(),
        has_more: end.has_more,
    })
}

/// How many commits come before a commit in the timeline; none when it is not there (`graph.read-position`).
pub fn position(reading: &Reading<'_>, params: &PositionParams) -> Result<Value, ResponseError> {
    let request = PositionRequest {
        commit: commit_of(&params.commit).map_err(invalid)?,
        history: commits_of(reading, &params.timeline)?.history,
    };
    answered(read_position(&reading.git, &request), |index| PositionResult {
        index: *index,
    })
}

/// The commits of the history of the timeline that a search finds, newest first (`search.read-matches`).
pub fn matches(reading: &Reading<'_>, params: &MatchesParams) -> Result<Value, ResponseError> {
    // The search looks in the history the graph lists, which its first parents and its narrowing are part of.
    let listed = commits_of(reading, &params.timeline)?.history;
    let request = MatchesRequest {
        mode: params.mode.into(),
        text: params.text.clone(),
        selection: listed.arguments(),
    };
    answered(read_matches(&reading.git, &request), |matches| {
        MatchesJson::from(matches)
    })
}

/// What a read of the timeline asks of git, against what the repository has: as the last read of its state kept it,
/// with nothing asked of git again; with no state kept, as before the first read or after an operation, what is
/// checked out always, the references only for the branches shown that need them, and the stashes only when they are
/// placed among the commits.
fn commits_of(reading: &Reading<'_>, timeline: &TimelineJson) -> Result<CommitsRequest, ResponseError> {
    let request = TimelineRequest::try_from(timeline).map_err(invalid)?;
    if let Some(state) = &reading.kept_state {
        return Ok(commits_request(state, &request));
    }
    let parts = TimelineParts {
        references: matches!(
            request.shown,
            Some(BranchesShown::Upstream | BranchesShown::References(_))
        ),
        stashes: request.shows_stashes,
    };
    let state = read_timeline_parts(&reading.git, parts).map_err(|error| git_failure(&error))?;
    Ok(commits_request(&state, &request))
}
