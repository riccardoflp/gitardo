//! The reads of the details and of the commit box that version 1.13 adds: the signature of a commit
//! (`details.read-signature`) and the authors of the history (`commit.read-authors`).

use git::read::commit::{AuthorsRequest, read_authors};
use git::read::details::{SignatureRequest, read_signature};
use protocol::reads::{AuthorsResult, CommitSignatureJson, SignatureParams, commit_of};
use protocol::rpc::ResponseError;
use serde_json::Value;

use super::{Reading, answered, invalid};

/// What git says of the signature of a commit, checked with the program its configuration names
/// (`details.read-signature`).
pub fn signature(reading: &Reading<'_>, params: &SignatureParams) -> Result<Value, ResponseError> {
    let request = SignatureRequest {
        commit: commit_of(&params.commit).map_err(invalid)?,
    };
    answered(read_signature(&reading.git, &request), |signature| {
        CommitSignatureJson::from(signature)
    })
}

/// The people who made commits in the repository, the one with the most first (`commit.read-authors`).
pub fn authors(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_authors(&reading.git, &AuthorsRequest), |authors| {
        AuthorsResult::from(authors.as_slice())
    })
}
