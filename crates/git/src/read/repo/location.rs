//! Where the working tree of a repository is, and the folder of git's files for it.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::read::repo::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// The top folder of a working tree and the folder of git's files for it, as git names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The top folder of the working tree.
    pub folder: PathBuf,
    /// The folder of git's files for this worktree, in full.
    pub git_folder: PathBuf,
}

/// `git rev-parse --show-toplevel --absolute-git-dir`: the two folders, one per line.
#[must_use]
pub fn location_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--show-toplevel", "--absolute-git-dir"])
}

/// The two folders that `git rev-parse --show-toplevel --absolute-git-dir` prints.
///
/// # Errors
///
/// When git printed fewer than two folders.
pub fn parse_location(output: &[u8]) -> Result<Location, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut lines = text.lines();
    match (lines.next(), lines.next()) {
        (Some(folder), Some(git_folder)) if !folder.is_empty() && !git_folder.is_empty() => Ok(Location {
            folder: PathBuf::from(folder),
            git_folder: PathBuf::from(git_folder),
        }),
        _ => Err(unexpected("where the repository is", &text)),
    }
}

/// Where the repository of git's folder is.
pub(super) fn read_location(git: &Git) -> Result<Location, GitError> {
    parse_location(&git.run_read(&location_arguments())?)
}
