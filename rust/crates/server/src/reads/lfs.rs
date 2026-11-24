//! The reads of Git LFS that version 1.13 adds for milestone M47: whether Git LFS is there with its version
//! (`lfs.read-status`), the files it stores (`lfs.read-files`), the locks of the server (`lfs.read-locks`), and a file
//! of a commit or of the index with the content in place of its pointer when the store of the repository has it
//! (`lfs.read-content`, `lfs.show-in-pages`). None downloads anything: a content not in the store is said not
//! downloaded, and only the locks talk to the server.

use git::read::diff::{DiffRequest, FileVersion, VersionRequest, read_sides};
use git::read::file_view::{RevisionFileRequest, read_revision};
use git::read::lfs::{read_files, read_locks, read_status, stored_content};
use git::read::search::{self, RevisionRequest};
use model::lfs::{LfsContent, LfsFilesRequest, LfsLocksRequest, LfsStatusRequest};
use protocol::reads::{
    BytesJson, LfsContentJson, LfsContentParams, LfsFileJson, LfsFilesJson, LfsFilesParams, LfsLocksJson,
    LfsLocksParams, LfsStatusJson, StoredPointerJson,
};
use protocol::rpc::ResponseError;
use serde_json::Value;

use super::{Reading, answered};
use crate::session::{git_failure, json};

/// Whether Git LFS is installed, its version, whether the repository is set up, and its patterns; the version is read
/// every time, as **Refresh** of the Large files tab asks.
pub fn status(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_status(&reading.git, &LfsStatusRequest::default()), |status| {
        LfsStatusJson::from(status)
    })
}

/// The files of the commit checked out stored with Git LFS, with their sizes and whether they are downloaded.
pub fn files(reading: &Reading<'_>, params: LfsFilesParams) -> Result<Value, ResponseError> {
    let request = LfsFilesRequest {
        version: params.version,
    };
    answered(read_files(&reading.git, &request), |files| LfsFilesJson {
        files: files.iter().map(LfsFileJson::from).collect(),
    })
}

/// The locks of the server of a remote, or that it does not support them; any other failure is what git said.
pub fn locks(reading: &Reading<'_>, params: LfsLocksParams) -> Result<Value, ResponseError> {
    let request = LfsLocksRequest { remote: params.remote };
    answered(read_locks(&reading.git, &request), |locks| LfsLocksJson::from(locks))
}

/// A file as a commit or the index has it: the content of its pointer when the store of the repository has it, the
/// pointer otherwise, with its size, its id and whether it is downloaded; any other file as it is.
pub fn content(reading: &Reading<'_>, params: &LfsContentParams) -> Result<Value, ResponseError> {
    let (bytes, commit) = version(reading, params)?;
    let Some(bytes) = bytes else {
        return json(&LfsContentJson {
            bytes: None,
            commit,
            stored: None,
        });
    };
    let (shown, stored) = match stored_content(&reading.git, &params.path, &bytes) {
        None => (bytes, None),
        Some((pointer, LfsContent::Downloaded(content))) => (content, Some(StoredPointerJson::of(&pointer, true))),
        Some((pointer, LfsContent::NotDownloaded)) => (bytes, Some(StoredPointerJson::of(&pointer, false))),
    };
    json(&LfsContentJson {
        bytes: Some(BytesJson::of(&shown)),
        commit,
        stored,
    })
}

/// The bytes of the file as the commit the revision names has it, with that commit by its full id, or as the index has
/// it for no revision; no bytes when there is no such file, or no such commit.
fn version(
    reading: &Reading<'_>,
    params: &LfsContentParams,
) -> Result<(Option<Vec<u8>>, Option<String>), ResponseError> {
    let Some(revision) = &params.revision else {
        let request = DiffRequest {
            left: VersionRequest {
                version: FileVersion::Index,
                path: params.path.clone(),
            },
            right: VersionRequest {
                version: FileVersion::Absent,
                path: params.path.clone(),
            },
        };
        let sides = read_sides(&reading.git, &request).map_err(|error| git_failure(&error))?;
        return Ok((sides.left, None));
    };
    let asked = RevisionRequest {
        revision: revision.clone(),
    };
    let commit = search::read_revision(&reading.git, &asked).map_err(|error| git_failure(&error))?;
    let Some(commit) = commit else {
        return Ok((None, None));
    };
    let request = RevisionFileRequest {
        commit: commit.clone(),
        path: params.path.clone(),
    };
    let bytes = read_revision(&reading.git, &request).map_err(|error| git_failure(&error))?;
    Ok((bytes, Some(commit.as_str().to_owned())))
}
