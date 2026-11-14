//! Reads of something that may not be there: a commit asked for by its hash, a file at a revision, the third parent of
//! a stash. That it is not there is an answer, not an error.

use std::ffi::OsString;

use crate::read::fields::{arguments, read_or_nothing};
use crate::{Git, GitError, command_line};

/// Runs a read about an object and returns what it printed; when it fails because the object is not there, nothing.
/// The object is named as `git rev-parse --verify` takes it: `<hash>^{commit}`, `<hash>:<path>`, `stash@{0}^3`.
///
/// The read runs first, and the object is looked for only when it failed, so that what is there costs one command.
pub(crate) fn read_unless_missing(git: &Git, read: &[OsString], object: &str) -> Result<Option<Vec<u8>>, GitError> {
    let output = git.read_output(read)?;
    if output.is_success() {
        return Ok(Some(output.stdout));
    }
    if is_there(git, object)? {
        Err(GitError::Failed {
            command: command_line(read),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    } else {
        Ok(None)
    }
}

/// Whether git knows an object by this name.
pub(crate) fn is_there(git: &Git, object: &str) -> Result<bool, GitError> {
    let found = read_or_nothing(git, &arguments(&["rev-parse", "--verify", "--quiet", object]))?;
    Ok(found.is_some())
}
