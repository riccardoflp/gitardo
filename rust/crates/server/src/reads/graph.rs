//! The reads of the graph, from stage 7: `graph.read-commits`, `graph.read-position` and `search.read-matches`, each
//! over the timeline the client shows, turned into git's arguments by `engine::timeline` as the desktop's are.

use engine::timeline::{TimelineRequest, commits_request};
use git::read::graph::{CommitsRequest, PositionRequest, read_commits, read_position};
use git::read::repo::{TimelineParts, read_timeline_parts};
use git::read::search::{MatchesRequest, read_matches};
use model::filter::BranchesShown;
use protocol::reads::{
    CommitPageJson, CommitsParams, MatchesJson, MatchesParams, PositionParams, PositionResult, TimelineJson, commit_of,
};
use protocol::rpc::ResponseError;
use serde_json::Value;

use super::{Reading, answered, invalid};
use crate::session::git_failure;

/// The rows of the pages of the timeline asked for, after those the client has (`graph.read-commits`).
pub fn commits(reading: &Reading<'_>, params: &CommitsParams) -> Result<Value, ResponseError> {
    let request = CommitsRequest {
        skip: params.timeline.skip,
        ..commits_of(reading, &params.timeline)?
    };
    answered(read_commits(&reading.git, &request), |page| CommitPageJson::from(page))
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

/// What a read of the timeline asks of git, against what the repository has now: what is checked out always, the
/// references only for the branches shown that need them, and the stashes only when they are placed among the commits.
fn commits_of(reading: &Reading<'_>, timeline: &TimelineJson) -> Result<CommitsRequest, ResponseError> {
    let request = TimelineRequest::try_from(timeline).map_err(invalid)?;
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
