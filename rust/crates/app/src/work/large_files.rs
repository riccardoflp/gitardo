//! The read of the Large files page (`lfs.page`): whether Git LFS is installed and its version, whether the repository
//! is set up, the patterns, the files stored with Git LFS in the commit checked out, and the locks of the server when
//! asked, which only the opening of the page and **Refresh** do (Q1195).

use std::path::Path;

use git::read::lfs::{read_files, read_locks, read_status};
use git::{CancelToken, Git};
use model::lfs::{LfsFile, LfsFilesRequest, LfsLocks, LfsLocksRequest, LfsStatus, LfsStatusRequest};

/// What the page read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LargeFilesRead {
    /// Git LFS and the patterns.
    pub status: LfsStatus,
    /// The files, none without Git LFS.
    pub files: Vec<LfsFile>,
    /// The locks, or what git said when they could not be read; none when they were not asked.
    pub locks: Option<Result<LfsLocks, String>>,
}

/// Reads Git LFS and the files of a repository, and the locks when asked and Git LFS is installed.
///
/// # Errors
///
/// What git said when it could not be started or failed.
pub fn read_large_files(folder: &Path, with_locks: bool, cancel: &CancelToken) -> Result<LargeFilesRead, String> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let status = read_status(&git, &LfsStatusRequest::default()).map_err(|error| error.to_string())?;
    let Some(version) = status.version.clone() else {
        return Ok(LargeFilesRead {
            status,
            files: Vec::new(),
            locks: None,
        });
    };
    let files = read_files(&git, &LfsFilesRequest { version }).map_err(|error| error.to_string())?;
    let locks = with_locks.then(|| read_locks(&git, &LfsLocksRequest::default()).map_err(|error| error.to_string()));
    Ok(LargeFilesRead { status, files, locks })
}
