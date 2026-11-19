//! The files a pattern of Git LFS takes that were committed before it was there (`lfs.store-committed`), and the files
//! stored with Git LFS left as pointers in the working tree (`lfs.set-up`).

use std::ffi::OsString;

use model::lfs::LfsFilesRequest;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// The arguments that list the files git tracks that pathspecs take: `git ls-files -z -- <pathspec>…`.
#[must_use]
pub fn tracked_arguments(pathspecs: &[String]) -> Vec<OsString> {
    let mut listed = arguments(&["ls-files", "-z", "--"]);
    listed.extend(pathspecs.iter().map(OsString::from));
    listed
}

/// The paths `git ls-files -z` printed, one before each NUL.
#[must_use]
pub fn parse_tracked(printed: &[u8]) -> Vec<String> {
    printed
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect()
}

/// The paths `git lfs ls-files --name-only` printed, one on each line.
#[must_use]
pub fn parse_stored(printed: &str) -> Vec<String> {
    printed
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The files git tracks that the pathspecs take and that Git LFS does not store yet: those `git ls-files` lists
/// without those `git lfs ls-files --name-only` lists, which the commits before keep as they are (Q1193).
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
pub fn read_to_store(git: &Git, pathspecs: &[String]) -> Result<Vec<String>, GitError> {
    let tracked = parse_tracked(&git.run_read(&tracked_arguments(pathspecs))?);
    if tracked.is_empty() {
        return Ok(tracked);
    }
    let stored = parse_stored(&String::from_utf8_lossy(&git.run_read(&arguments(&[
        "lfs",
        "ls-files",
        "--name-only",
    ]))?));
    Ok(tracked.into_iter().filter(|path| !stored.contains(path)).collect())
}

/// The files stored with Git LFS whose file on disk is their pointer, not their content (`lfs.read-files`).
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
pub fn read_pointers(git: &Git, request: &LfsFilesRequest) -> Result<Vec<String>, GitError> {
    Ok(super::read_files(git, request)?
        .into_iter()
        .filter(|file| !file.is_checked_out)
        .map(|file| file.path)
        .collect())
}
