//! The remotes of a repository, each with the URL it is fetched from.

use std::ffi::OsString;

use crate::read::fields::arguments;
use crate::read::repo::{parse_remotes, remotes_arguments};
use crate::{Git, GitError};

/// Which repository: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemotesRequest;

/// A remote of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Its name: `origin`.
    pub name: String,
    /// The URL it is fetched from, as git uses it: after `insteadOf` is applied.
    pub url: String,
}

/// The remotes of the repository, in the order git lists them, each with the URL it is fetched from. A repository with
/// no remote has none.
///
/// Feature: remote.read-remotes
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_remotes(git: &Git, _request: &RemotesRequest) -> Result<Vec<Remote>, GitError> {
    let names = parse_remotes(&git.run_read(&remotes_arguments())?)?;
    names
        .into_iter()
        .map(|name| {
            let url = parse_url(&git.run_read(&url_arguments(&name))?);
            Ok(Remote { name, url })
        })
        .collect()
}

/// `git remote get-url <name>`: the URL a remote is fetched from, with `insteadOf` applied.
#[must_use]
pub fn url_arguments(name: &str) -> Vec<OsString> {
    let mut words = arguments(&["remote", "get-url"]);
    words.push(OsString::from(name));
    words
}

/// The URL that `git remote get-url` printed, without its line break.
#[must_use]
pub fn parse_url(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .trim_end_matches(['\n', '\r'])
        .to_owned()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{parse_url, url_arguments};

    #[test]
    fn remote_read_remotes__the_url_is_read_whole_without_its_line_break() {
        assert_eq!(
            parse_url(b"https://example.com/team/app [x].git\n"),
            "https://example.com/team/app [x].git"
        );
        assert_eq!(url_arguments("my/fork"), ["remote", "get-url", "my/fork"]);
    }
}
