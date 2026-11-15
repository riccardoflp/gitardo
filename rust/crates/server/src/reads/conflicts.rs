//! The reads of the conflicts: the versions of a file in conflict, the files in conflict with their stages, what the
//! two sides are, and what the rules propose for each conflict.

use std::time::{Duration, Instant};

use git::read::conflicts::{ConflictFilesRequest, read_files};
use git::read::merge_view::{MergeSidesRequest, read_sides};
use git::read::review::{SideNamesRequest, read_side_names};
use model::conflicts::MOST_FILES_LOOKED_INTO;
use protocol::reads::{ConflictFileJson, MergeSidesJson, MergeSidesParams, RulesParams, SideNamesJson, proposals_of};
use protocol::rpc::ResponseError;
use serde_json::Value;
use text::conflicts::{RulesInput, resolve_by_rules};

use super::{Reading, answered, invalid};
use crate::session::{git_failure, json};

/// How long the rules may look into one file before it is left alone, as the review of the conflicts has it.
const TIME_LIMIT: Duration = Duration::from_secs(5);

/// The versions of a file in conflict, and the file as it is now (`merge-view.read-sides`).
pub fn merge_sides(reading: &Reading<'_>, params: MergeSidesParams) -> Result<Value, ResponseError> {
    let request = MergeSidesRequest {
        file: params.path,
        unsaved: params
            .unsaved
            .as_ref()
            .map(protocol::reads::BytesJson::bytes)
            .transpose()
            .map_err(invalid)?,
    };
    answered(read_sides(&reading.git, &request), |read| MergeSidesJson::from(read))
}

/// The files in conflict, with the stages the index has of each (`conflicts.read-files`).
pub fn files(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_files(&reading.git, &ConflictFilesRequest), |files| {
        files.files.iter().map(ConflictFileJson::from).collect::<Vec<_>>()
    })
}

/// What the two sides of the operation under way are (`conflicts.side-names`).
pub fn side_names(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_side_names(&reading.git, &SideNamesRequest), |read| {
        SideNamesJson::from(&model::conflicts::side_names(&read.input()))
    })
}

/// What the rules propose for each conflict of the files in conflict, or of some of them
/// (`conflicts.resolve-by-rules`): the first 200 files git lists, each looked into for at most five seconds.
pub fn rules(reading: &Reading<'_>, params: &RulesParams) -> Result<Value, ResponseError> {
    let listed = read_files(&reading.git, &ConflictFilesRequest).map_err(|error| git_failure(&error))?;
    let asked = |path: &str| {
        params
            .paths
            .as_ref()
            .is_none_or(|paths| paths.iter().any(|asked| asked == path))
    };
    let mut proposals = Vec::new();
    for file in listed
        .files
        .iter()
        .filter(|file| asked(&file.path))
        .take(MOST_FILES_LOOKED_INTO)
    {
        let request = MergeSidesRequest {
            file: file.path.clone(),
            unsaved: None,
        };
        let sides = read_sides(&reading.git, &request).map_err(|error| git_failure(&error))?;
        let started = Instant::now();
        let keep_going = || started.elapsed() < TIME_LIMIT && !reading.cancel.is_cancelled();
        let resolutions = resolve_by_rules(&RulesInput {
            file: &sides.result,
            base: &sides.base,
            current: &sides.current,
            incoming: &sides.incoming,
            keep_going: &keep_going,
        });
        proposals.push(proposals_of(&file.path, &resolutions));
    }
    json(&proposals)
}
