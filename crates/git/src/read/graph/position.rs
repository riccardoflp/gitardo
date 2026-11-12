//! Where a commit is in the history the graph lists, to know how far it must be loaded for the commit to be there.

use std::ffi::OsString;

use model::CommitId;

use crate::read::graph::History;
use crate::{Git, GitError};

/// Which commit is looked for, in which history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionRequest {
    /// The history listed, with the same selection, narrowing and parents as the graph reads.
    pub history: History,
    /// The commit looked for.
    pub commit: CommitId,
}

/// `git log` with the full hash of each commit, one per line, over the history listed.
#[must_use]
pub fn position_arguments(request: &PositionRequest) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("log"), OsString::from("--format=%H")];
    arguments.extend(request.history.arguments());
    arguments
}

/// How many commits come before a commit in what `git log` printed with [`position_arguments`]: the line of its full
/// hash, from 0; none when it is not there.
#[must_use]
pub fn parse_position(output: &[u8], commit: &CommitId) -> Option<usize> {
    let wanted = commit.as_str().as_bytes();
    output
        .split(|byte| *byte == b'\n')
        .position(|line| line.strip_suffix(b"\r").unwrap_or(line) == wanted)
}

/// How many commits come before a commit in the history the graph lists; none when it is not in it.
///
/// Feature: graph.read-position
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_position(git: &Git, request: &PositionRequest) -> Result<Option<usize>, GitError> {
    let output = git.run_read(&position_arguments(request))?;
    Ok(parse_position(&output, &request.commit))
}
