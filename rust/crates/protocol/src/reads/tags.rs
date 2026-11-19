//! The read of the tags of a remote, which version 1.13 adds for milestone M35: `read/tag.read-remote-tags` asks a
//! remote for its tags and gives them compared with those of the repository (`tag.read-remote-tags`,
//! `tag.compare-remote`), for **Check tags on `<remote>`** and the marks of the tags it lacks (`tag.remote-marks`).

use model::tag::{TagComparison, TagOnRemote};
use serde::{Deserialize, Serialize};

/// The parameters of `read/tag.read-remote-tags`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTagsParams {
    /// The remote, by its name: `origin`.
    pub remote: String,
}

/// Where a tag of the repository stands on the remote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TagOnRemoteJson {
    /// The remote has a tag of its name naming the same object.
    Pushed,
    /// The remote has a tag of its name naming another object.
    DifferentThere,
    /// The remote has no tag of its name.
    NotPushed,
}

/// A tag of the repository and where it stands on the remote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagStandingJson {
    /// Its name: `v1.0`.
    pub name: String,
    /// Where it stands.
    pub on_remote: TagOnRemoteJson,
}

/// The answer of `read/tag.read-remote-tags`: the tags of the repository sorted against those of the remote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagComparisonJson {
    /// The remote asked.
    pub remote: String,
    /// Each tag of the repository, with where it stands on the remote.
    pub tags: Vec<TagStandingJson>,
    /// The tags of the remote with no tag of their name here.
    pub only_there: Vec<String>,
}

impl From<&TagComparison> for TagComparisonJson {
    fn from(comparison: &TagComparison) -> TagComparisonJson {
        let on_remote = |state: TagOnRemote| match state {
            TagOnRemote::Pushed => TagOnRemoteJson::Pushed,
            TagOnRemote::DifferentThere => TagOnRemoteJson::DifferentThere,
            TagOnRemote::NotPushed => TagOnRemoteJson::NotPushed,
        };
        TagComparisonJson {
            remote: comparison.remote.clone(),
            tags: comparison
                .here
                .iter()
                .map(|(name, state)| TagStandingJson {
                    name: name.clone(),
                    on_remote: on_remote(*state),
                })
                .collect(),
            only_there: comparison.only_there.clone(),
        }
    }
}
