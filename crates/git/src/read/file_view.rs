//! A file as it is at a revision, for the File page.

use std::ffi::OsString;

use model::CommitId;

use crate::read::existing::read_unless_missing;
use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Which file, at which commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionFileRequest {
    /// The commit.
    pub commit: CommitId,
    /// The path of the file from the top of the repository, with `/`.
    pub path: String,
}

/// `git show <commit>:<path>`, which prints the blob as it is: no text conversion is applied to a blob.
#[must_use]
pub fn revision_file_arguments(request: &RevisionFileRequest) -> Vec<OsString> {
    blob_arguments(&blob_name(&request.commit, &request.path))
}

/// The text of a file as it is in a commit, byte for byte, line endings included; none when the commit has no such
/// file.
///
/// Feature: file-view.read-revision
///
/// # Errors
///
/// When git cannot be started or fails for another reason than the file not being there.
pub fn read_revision(git: &Git, request: &RevisionFileRequest) -> Result<Option<Vec<u8>>, GitError> {
    let object = blob_name(&request.commit, &request.path);
    read_unless_missing(git, &blob_arguments(&object), &object)
}

/// The name git gives a file of a commit: `<hash>:<path>`. A path that starts with `:` is read as it is, since the hash
/// has no `:` in it.
pub(crate) fn blob_name(commit: &CommitId, path: &str) -> String {
    format!("{commit}:{path}")
}

/// `git show <object> --`: the blob as it is.
pub(crate) fn blob_arguments(object: &str) -> Vec<OsString> {
    arguments(&["show", object, "--"])
}
