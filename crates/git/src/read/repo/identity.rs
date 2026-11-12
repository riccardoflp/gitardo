//! Who commits are made as, from git's configuration.

use std::ffi::OsString;

use model::state::Identity;

use crate::read::fields::{arguments, read_or_nothing};
use crate::{Git, GitError};

/// `git config -z --get-regexp ^user\.(name|email)$`: each key and its value, a line break between them and a NUL after,
/// from every file of the configuration in the order git reads them; the exit code 1 when none is set.
#[must_use]
pub fn identity_arguments() -> Vec<OsString> {
    arguments(&["config", "-z", "--get-regexp", r"^user\.(name|email)$"])
}

/// The identity that `git config -z --get-regexp` printed. A key set in several files takes the value git reads last,
/// as git does.
///
/// # Errors
///
/// None: a key this does not ask for is left out. The `Result` keeps the shape of every parser of a read.
pub fn parse_identity(output: &[u8]) -> Result<Identity, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut identity = Identity::default();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        // A key with no value at all, `[user] name` alone, is printed without the line break.
        let (key, value) = record.split_once('\n').unwrap_or((record, ""));
        match key {
            "user.name" => identity.name = Some(value.to_owned()),
            "user.email" => identity.email = Some(value.to_owned()),
            _ => {}
        }
    }
    Ok(identity)
}

/// Who commits are made as in the repository.
pub(super) fn read_identity(git: &Git) -> Result<Identity, GitError> {
    read_or_nothing(git, &identity_arguments())?
        .map_or_else(|| Ok(Identity::default()), |output| parse_identity(&output))
}
