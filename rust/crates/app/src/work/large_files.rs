//! The read of the Large files page (`lfs.page`): whether Git LFS is installed and its version, whether the repository
//! is set up, the patterns, and the files stored with Git LFS in the commit checked out.

use std::path::Path;

use git::read::lfs::{read_files, read_status};
use git::{CancelToken, Git};
use model::lfs::{LfsFile, LfsFilesRequest, LfsStatus, LfsStatusRequest};

/// What the page read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LargeFilesRead {
    /// Git LFS and the patterns.
    pub status: LfsStatus,
    /// The files, none without Git LFS.
    pub files: Vec<LfsFile>,
}

/// Reads Git LFS and the files of a repository; what git said when it failed.
///
/// # Errors
///
/// What git said when it could not be started or failed.
pub fn read_large_files(folder: &Path, cancel: &CancelToken) -> Result<LargeFilesRead, String> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let status = read_status(&git, &LfsStatusRequest::default()).map_err(|error| error.to_string())?;
    let files = match &status.version {
        Some(version) => read_files(
            &git,
            &LfsFilesRequest {
                version: version.clone(),
            },
        )
        .map_err(|error| error.to_string())?,
        None => Vec::new(),
    };
    Ok(LargeFilesRead { status, files })
}
