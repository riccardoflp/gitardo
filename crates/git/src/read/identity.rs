//! Who commits will be made as, and which of the `user.*` keys the repository sets for itself.

use std::ffi::OsString;

use model::state::Identity;

use crate::read::fields::{arguments, read_or_nothing};
use crate::read::repo::{identity_arguments, parse_identity};
use crate::{Git, GitError};

/// Which repository: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdentityRequest;

/// The identity of a repository, and the keys it sets for itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentityState {
    /// The name and the address git will use, wherever they are set.
    pub identity: Identity,
    /// The `user.*` keys set in the configuration of the repository itself, each once, in the order git lists them:
    /// `user.name`, `user.signingkey`.
    pub local_keys: Vec<String>,
}

/// Who commits will be made as in the repository, and which of the `user.*` keys it sets for itself. A key set nowhere
/// is none, which is no error.
///
/// Feature: identity.read
///
/// # Errors
///
/// When git cannot be started or fails otherwise than by finding nothing.
pub fn read_identity(git: &Git, _request: &IdentityRequest) -> Result<IdentityState, GitError> {
    let identity = read_or_nothing(git, &identity_arguments())?
        .map_or_else(|| Ok(Identity::default()), |output| parse_identity(&output))?;
    let local_keys = read_or_nothing(git, &local_keys_arguments())?
        .map(|output| parse_local_keys(&output))
        .unwrap_or_default();
    Ok(IdentityState { identity, local_keys })
}

/// `git config -z --local --get-regexp ^user\.`: the `user.*` keys of the repository's own configuration, each key and
/// its value with a line break between them and a NUL after; the exit code 1 when there is none.
#[must_use]
pub fn local_keys_arguments() -> Vec<OsString> {
    arguments(&["config", "-z", "--local", "--get-regexp", r"^user\."])
}

/// The keys that `git config -z --local --get-regexp` printed, each once, in the order printed.
#[must_use]
pub fn parse_local_keys(output: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(output);
    let mut keys: Vec<String> = Vec::new();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        // A key with no value at all is printed without the line break.
        let key = record.split_once('\n').map_or(record, |(key, _)| key);
        if !keys.iter().any(|known| known == key) {
            keys.push(key.to_owned());
        }
    }
    keys
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_local_keys;

    #[test]
    fn identity_read__the_local_keys_are_each_key_once() {
        let output = b"user.name\nZo\xc3\xab\0user.signingkey\nABCD\0user.useconfigonly\0user.name\nOther\0";
        assert_eq!(
            parse_local_keys(output),
            ["user.name", "user.signingkey", "user.useconfigonly"]
        );
        assert_eq!(parse_local_keys(b""), Vec::<String>::new());
    }
}
