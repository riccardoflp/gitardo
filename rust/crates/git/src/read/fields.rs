//! The fields of what git prints for a machine, and what is said when one cannot be read.

use std::ffi::OsString;

use model::{CommitId, Timestamp};

use crate::{Git, GitError, command_line};

/// The arguments of a read, as git is given them.
pub(crate) fn arguments(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// The error for a part of git's output that this program does not understand.
pub(crate) fn unexpected(what: &'static str, detail: &str) -> GitError {
    GitError::Unexpected {
        what,
        detail: detail.to_owned(),
    }
}

/// A full hash, as git prints it.
pub(crate) fn commit_id(what: &'static str, text: &str) -> Result<CommitId, GitError> {
    CommitId::parse(text).ok_or_else(|| unexpected(what, text))
}

/// A hash where the hash of nothing, all zeros, means that there is no commit yet.
pub(crate) fn optional_commit_id(what: &'static str, text: &str) -> Result<Option<CommitId>, GitError> {
    if !text.is_empty() && text.bytes().all(|byte| byte == b'0') {
        return Ok(None);
    }
    commit_id(what, text).map(Some)
}

/// A date in seconds since 1970, as git prints it with `%at` and `:unix`.
pub(crate) fn timestamp(what: &'static str, text: &str) -> Result<Timestamp, GitError> {
    text.parse().map(Timestamp).map_err(|_| unexpected(what, text))
}

/// Runs a read whose exit code 1 means that there is nothing to say, as `rev-parse --verify --quiet` before the first
/// commit or `symbolic-ref --quiet` when `HEAD` is detached: what it printed, or nothing.
pub(crate) fn read_or_nothing(git: &Git, arguments: &[OsString]) -> Result<Option<Vec<u8>>, GitError> {
    let output = git.read_output(arguments)?;
    match output.code {
        Some(0) => Ok(Some(output.stdout)),
        Some(1) => Ok(None),
        code => Err(GitError::Failed {
            command: command_line(arguments),
            code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}
