//! `read/range-diff.read-pairs` (1.11, M36): the pairs of what a rewrite changed, commit by commit, for the tab of
//! `range-diff.page`, read in the repository opened with the lines its follow-up gave. It changes nothing.

use std::sync::Arc;
use std::time::Instant;

use git::read::range_diff::{RangeDiffRequest, read_pairs};
use git::{CancelToken, Git};
use protocol::range_diff::{RangeDiffJson, RangeDiffParams, methods};
use protocol::rpc::ResponseError;
use serde_json::Value;

use crate::session::{Session, git_failure, json, parameters};

/// The pairs of the two lines, logged as a read with how long it took.
///
/// # Errors
///
/// When the parameters are not the read's, the repository is not open, or git fails, refuses more commits than Gitardo compares, or is cancelled.
pub fn pairs(session: &Arc<Session>, params: Value, cancel: &CancelToken) -> Result<Value, ResponseError> {
    let params: RangeDiffParams = parameters(params)?;
    let opened = session.opened(&params.repository)?;
    let git = Git::new(opened.folder()).with_cancel(cancel.clone());
    let request = RangeDiffRequest {
        ranges: (&params.ranges).into(),
    };
    let started = Instant::now();
    let read = read_pairs(&git, &request);
    session.log_read(
        &params.repository,
        format!("Read {} in {}", methods::RANGE_DIFF_PAIRS, params.repository),
        started.elapsed(),
    );
    json(&RangeDiffJson::from(&read.map_err(|error| git_failure(&error))?))
}
