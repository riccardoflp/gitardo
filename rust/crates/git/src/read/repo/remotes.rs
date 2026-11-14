//! The names of the remotes.

use std::ffi::OsString;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// `git remote`: one name per line, as a remote name never holds a line break.
#[must_use]
pub fn remotes_arguments() -> Vec<OsString> {
    arguments(&["remote"])
}

/// The names that `git remote` printed.
///
/// # Errors
///
/// None: every line is a name. The `Result` keeps the shape of every parser of a read.
pub fn parse_remotes(output: &[u8]) -> Result<Vec<String>, GitError> {
    Ok(String::from_utf8_lossy(output)
        .split('\n')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

/// The names of the remotes of the repository.
pub(super) fn read_remotes(git: &Git) -> Result<Vec<String>, GitError> {
    parse_remotes(&git.run_read(&remotes_arguments())?)
}
