//! The two versions of a file the Diff page compares.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::thread;

use model::CommitId;
use model::diff::DiffSides;

use crate::read::existing::read_unless_missing;
use crate::read::file_view::{blob_arguments, blob_name};
use crate::{Git, GitError};

/// Where a version of a file is taken from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileVersion {
    /// Nowhere: the side of a commit with no parent, or of a file the other side does not have.
    Absent,
    /// The index.
    Index,
    /// A commit.
    Commit(CommitId),
    /// The common ancestor of a file in conflict, stage 1 of the index (`:1:<path>`).
    Base,
    /// The working tree: the text a File page holds for the file when it has unsaved edits there, else the disk.
    WorkingTree {
        /// The text of the File page, when it has unsaved edits.
        unsaved: Option<Vec<u8>>,
    },
}

/// One version of a file: where from, and its path there, which differs between the two sides of a rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequest {
    /// Where it is taken from.
    pub version: FileVersion,
    /// Its path from the top of the repository, with `/`.
    pub path: String,
}

/// The two versions a Diff page compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRequest {
    /// The version on the left.
    pub left: VersionRequest,
    /// The version on the right.
    pub right: VersionRequest,
}

/// The git command that reads a version, for the index (`git show :0:<path>`) and a commit
/// (`git show <commit>:<path>`); none for the working tree and for a version that is not there. The index is named with
/// its stage, `:0:`, so that a path starting with `:` is not read as one.
#[must_use]
pub fn version_arguments(request: &VersionRequest) -> Option<Vec<OsString>> {
    object_name(request).map(|object| blob_arguments(&object))
}

/// Each version of a file a Diff page compares, read at once. A version that is not there (a file not yet in the index
/// or in `HEAD`, deleted on disk, a commit with no parent) is none. Git runs in the top folder of the working tree,
/// which the paths are from.
///
/// Feature: diff.read-sides
///
/// # Errors
///
/// When git cannot be started or fails for another reason than the file not being there, or the file on disk cannot
/// be read.
pub fn read_sides(git: &Git, request: &DiffRequest) -> Result<DiffSides, GitError> {
    thread::scope(|scope| {
        let left = scope.spawn(|| read_version(git, &request.left));
        let right = read_version(git, &request.right)?;
        let left = left
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))?;
        Ok(DiffSides { left, right })
    })
}

/// One version, as bytes; none when it is not there.
fn read_version(git: &Git, request: &VersionRequest) -> Result<Option<Vec<u8>>, GitError> {
    match &request.version {
        FileVersion::Absent => Ok(None),
        FileVersion::WorkingTree { unsaved: Some(text) } => Ok(Some(text.clone())),
        FileVersion::WorkingTree { unsaved: None } => read_from_disk(git, &request.path),
        FileVersion::Index | FileVersion::Base | FileVersion::Commit(_) => match object_name(request) {
            Some(object) => read_unless_missing(git, &blob_arguments(&object), &object),
            None => Ok(None),
        },
    }
}

/// The name git gives a version of a file that it keeps: `:0:<path>` in the index, `<hash>:<path>` in a commit.
fn object_name(request: &VersionRequest) -> Option<String> {
    match &request.version {
        FileVersion::Index => Some(format!(":0:{}", request.path)),
        FileVersion::Base => Some(format!(":1:{}", request.path)),
        FileVersion::Commit(commit) => Some(blob_name(commit, &request.path)),
        FileVersion::Absent | FileVersion::WorkingTree { .. } => None,
    }
}

/// The file as it is on disk; none when it is not there.
fn read_from_disk(git: &Git, path: &str) -> Result<Option<Vec<u8>>, GitError> {
    let full = git.directory().join(path);
    match fs::read(&full) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GitError::File { path: full, source }),
    }
}
