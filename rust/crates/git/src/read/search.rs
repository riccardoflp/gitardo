//! Searching the whole history, and finding the commit a name stands for.

use std::ffi::OsString;

use model::CommitId;
use model::search::{Matches, SearchMode};

use crate::read::fields::{arguments, commit_id, read_or_nothing};
use crate::{Git, GitError};

/// The most commits a search keeps; git is asked for one more, to know whether there are more.
pub const MOST_MATCHES: usize = 5_000;

/// What to search for, and in which history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchesRequest {
    /// How to look at the commits.
    pub mode: SearchMode,
    /// What to look for, as typed.
    pub text: String,
    /// The arguments of `git log` that say which history the graph shows (`filter.log-arguments`), with those of the
    /// history of a file or of some lines when the graph shows one.
    pub selection: Vec<OsString>,
}

/// The commits of the history shown that a search of their messages, their authors or their changes finds, newest
/// first: at most [`MOST_MATCHES`], and whether there are more.
///
/// Feature: search.read-matches
///
/// # Errors
///
/// When git refuses the search, as for an invalid regular expression, with git's message; when git cannot be started
/// or prints what this does not understand.
pub fn read_matches(git: &Git, request: &MatchesRequest) -> Result<Matches, GitError> {
    parse_matches(&git.run_read(&matches_arguments(request))?)
}

/// The arguments of `git log` that make it find what a search finds, without the format and the count: those of the
/// search itself, which also narrow the graph to what is found when it is shown as a list.
#[must_use]
pub fn search_arguments(mode: SearchMode, text: &str) -> Vec<OsString> {
    match mode {
        SearchMode::Message => arguments(&["--fixed-strings", "--regexp-ignore-case", &format!("--grep={text}")]),
        SearchMode::Author => arguments(&["--fixed-strings", "--regexp-ignore-case", &format!("--author={text}")]),
        SearchMode::Changes => arguments(&[&format!("-S{text}")]),
        SearchMode::ChangesPattern => arguments(&[&format!("-G{text}")]),
    }
}

/// `git log --format=%H --max-count=5001`, the arguments of the search, then those of the history shown.
#[must_use]
pub fn matches_arguments(request: &MatchesRequest) -> Vec<OsString> {
    let mut words = arguments(&["log", "--format=%H", &format!("--max-count={}", MOST_MATCHES + 1)]);
    words.extend(search_arguments(request.mode, &request.text));
    words.extend(request.selection.iter().cloned());
    words
}

/// The commits that [`matches_arguments`] found, one hash per line: the first [`MOST_MATCHES`], and whether there was
/// one more.
///
/// # Errors
///
/// When a line is not a full hash.
pub fn parse_matches(output: &[u8]) -> Result<Matches, GitError> {
    let mut commits = String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| commit_id("the commits found", line))
        .collect::<Result<Vec<_>, _>>()?;
    let has_more = commits.len() > MOST_MATCHES;
    commits.truncate(MOST_MATCHES);
    Ok(Matches { commits, has_more })
}

/// A name of a commit: a hash, a branch, a tag, or anything else git understands, such as `HEAD~3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRequest {
    /// The name, as typed.
    pub revision: String,
}

/// The commit that a hash, a branch, a tag or anything else git understands stands for; none for a name git does not
/// know, an abbreviated hash that starts several objects, or something that is not a commit and leads to none. An empty
/// name, or one that starts with `-`, which git would take for an option, is not asked of git.
///
/// Feature: search.read-revision
///
/// # Errors
///
/// When git cannot be started, fails otherwise, or prints what this does not understand.
pub fn read_revision(git: &Git, request: &RevisionRequest) -> Result<Option<CommitId>, GitError> {
    let Some(words) = revision_arguments(request) else {
        return Ok(None);
    };
    read_or_nothing(git, &words)?
        .map(|output| parse_revision(&output))
        .transpose()
}

/// `git rev-parse --verify --quiet <revision>^{commit}`, which prints the full hash of the commit, or nothing and the
/// exit code 1; none for a name that is not to be asked of git.
#[must_use]
pub fn revision_arguments(request: &RevisionRequest) -> Option<Vec<OsString>> {
    let revision = request.revision.as_str();
    if revision.is_empty() || revision.starts_with('-') {
        return None;
    }
    Some(arguments(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{revision}^{{commit}}"),
    ]))
}

/// The commit that [`revision_arguments`] printed.
///
/// # Errors
///
/// When it is not a full hash.
pub fn parse_revision(output: &[u8]) -> Result<CommitId, GitError> {
    commit_id("the commit of a name", String::from_utf8_lossy(output).trim_end())
}

#[cfg(test)]
mod tests {
    use super::{MOST_MATCHES, RevisionRequest, parse_matches, revision_arguments};

    #[test]
    fn keeps_the_most_matches_and_says_there_are_more() {
        let hashes: Vec<String> = (0..=MOST_MATCHES).map(|number| format!("{number:040x}")).collect();
        let output = hashes.join("\n");
        let matches = parse_matches(output.as_bytes()).unwrap();
        assert_eq!(matches.commits.len(), MOST_MATCHES);
        assert!(matches.has_more);
    }

    #[test]
    fn asks_nothing_for_an_empty_name_or_an_option() {
        for revision in ["", "--all", "-n"] {
            let request = RevisionRequest {
                revision: revision.to_owned(),
            };
            assert_eq!(revision_arguments(&request), None, "{revision}");
        }
    }
}
