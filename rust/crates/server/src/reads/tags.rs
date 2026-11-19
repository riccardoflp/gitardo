//! The read of the tags of a remote that version 1.13 adds for milestone M35 (`tag.read-remote-tags`): the remote asked
//! over the network, on the thread of the read, and its tags compared with those of the repository
//! (`tag.compare-remote`).

use git::read::tag::{RemoteTagsRequest, read_remote_tags};
use model::tag::compare_remote;
use protocol::reads::{RemoteTagsParams, TagComparisonJson};
use protocol::rpc::ResponseError;
use serde_json::Value;

use super::{Reading, answered};

/// The tags of the repository sorted against those of a remote: pushed, different there or not pushed, and those only
/// the remote has. A remote that cannot be reached answers with what git said; git never asks in a terminal.
pub fn remote_tags(reading: &Reading<'_>, params: RemoteTagsParams) -> Result<Value, ResponseError> {
    let request = RemoteTagsRequest { remote: params.remote };
    answered(read_remote_tags(&reading.git, &request), |read| {
        TagComparisonJson::from(&compare_remote(&read.here, read))
    })
}
