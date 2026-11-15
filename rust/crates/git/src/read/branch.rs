//! What the operations on branches ask before they decide: how far two branches are apart, whether one has the other,
//! which branches are merged or have a commit, and whether a commit has submodules; and the commits of what is checked
//! out ([`log`]).

mod log;

use std::ffi::OsString;

pub use log::{
    BranchLogRequest, head_arguments, log_of_head_arguments, parse_unpushed, parse_upstream, read_log,
    unpushed_arguments, upstream_arguments,
};

use crate::read::fields::{arguments, read_or_nothing, unexpected};
use crate::{Git, GitError};

/// What the reads of this file are called when git prints what they do not understand.
const WHAT: &str = "the counts of `git rev-list --left-right --count`";

/// How many commits each of two revisions has that the other has not: the left's, then the right's.
///
/// # Errors
///
/// When git cannot be started, fails, or does not print two numbers.
pub fn read_counts(git: &Git, left: &str, right: &str) -> Result<(u32, u32), GitError> {
    parse_counts(&git.run_read(&counts_arguments(left, right))?)
}

/// `git rev-list --left-right --count <left>...<right> --`.
#[must_use]
pub fn counts_arguments(left: &str, right: &str) -> Vec<OsString> {
    let mut words = arguments(&["rev-list", "--left-right", "--count"]);
    words.push(OsString::from(format!("{left}...{right}")));
    words.push(OsString::from("--"));
    words
}

/// The two counts `git rev-list --left-right --count` prints, the left's first.
///
/// # Errors
///
/// When it is not two numbers.
pub fn parse_counts(output: &[u8]) -> Result<(u32, u32), GitError> {
    let text = String::from_utf8_lossy(output);
    let mut numbers = text.split_whitespace().map(str::parse::<u32>);
    match (numbers.next(), numbers.next(), numbers.next()) {
        (Some(Ok(left)), Some(Ok(right)), None) => Ok((left, right)),
        _ => Err(unexpected(WHAT, &text)),
    }
}

/// Whether `ancestor` is an ancestor of `descendant`, or the same commit.
///
/// # Errors
///
/// When git cannot be started, or fails for another reason than saying no: a revision it does not know.
pub fn read_is_ancestor(git: &Git, ancestor: &str, descendant: &str) -> Result<bool, GitError> {
    read_or_nothing(git, &ancestor_arguments(ancestor, descendant)).map(|output| output.is_some())
}

/// `git merge-base --is-ancestor <ancestor> <descendant>`, which ends with code 0 for yes and 1 for no.
#[must_use]
pub fn ancestor_arguments(ancestor: &str, descendant: &str) -> Vec<OsString> {
    arguments(&["merge-base", "--is-ancestor", ancestor, descendant])
}

/// The local branches merged into a revision, by their short names, in the order git lists them.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_merged(git: &Git, into: &str) -> Result<Vec<String>, GitError> {
    Ok(parse_merged(&git.run_read(&merged_arguments(into))?))
}

/// `git branch --merged <into> --format=%(refname:short)`.
#[must_use]
pub fn merged_arguments(into: &str) -> Vec<OsString> {
    arguments(&["branch", "--merged", into, "--format=%(refname:short)"])
}

/// The names `git branch --format=%(refname:short)` prints, one a line.
#[must_use]
pub fn parse_merged(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The local and remote branches that have a commit, by their full names, in the order git lists them, without the
/// `HEAD` of a remote (`branch.name-from-format`).
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_containing(git: &Git, commit: &str) -> Result<Vec<String>, GitError> {
    Ok(parse_containing(&git.run_read(&containing_arguments(commit))?))
}

/// `git for-each-ref --contains <hash> --format=%(refname) refs/heads refs/remotes`.
#[must_use]
pub fn containing_arguments(commit: &str) -> Vec<OsString> {
    arguments(&[
        "for-each-ref",
        "--contains",
        commit,
        "--format=%(refname)",
        "refs/heads",
        "refs/remotes",
    ])
}

/// The full names that [`containing_arguments`] listed, one a line, without `refs/remotes/<remote>/HEAD`.
#[must_use]
pub fn parse_containing(output: &[u8]) -> Vec<String> {
    parse_merged(output)
        .into_iter()
        .filter(|name| !(name.starts_with("refs/remotes/") && name.ends_with("/HEAD")))
        .collect()
}

/// Whether a commit has a file. A commit git does not know, or no commit yet, has none.
///
/// # Errors
///
/// When git cannot be started.
pub fn read_has_file(git: &Git, revision: &str, path: &str) -> Result<bool, GitError> {
    let output = git.read_output(&has_file_arguments(revision, path))?;
    Ok(output.is_success())
}

/// `git cat-file -e <revision>:<path>`, which ends with code 0 when the commit has the file.
#[must_use]
pub fn has_file_arguments(revision: &str, path: &str) -> Vec<OsString> {
    arguments(&["cat-file", "-e", &format!("{revision}:{path}")])
}

#[cfg(test)]
mod tests {
    use super::{counts_arguments, parse_containing, parse_counts, parse_merged};

    #[test]
    fn reads_the_two_counts_the_left_first() {
        assert_eq!(parse_counts(b"1\t2\n").unwrap(), (1, 2));
        assert!(parse_counts(b"1\n").is_err());
        assert!(parse_counts(b"a\tb\n").is_err());
    }

    #[test]
    fn counts_the_symmetric_difference_with_paths_ended() {
        let words: Vec<String> = counts_arguments("refs/heads/feature", "origin/feature")
            .iter()
            .map(|word| word.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words,
            [
                "rev-list",
                "--left-right",
                "--count",
                "refs/heads/feature...origin/feature",
                "--"
            ]
        );
    }

    #[test]
    fn reads_the_branches_that_have_a_commit_without_the_head_of_a_remote() {
        assert_eq!(
            parse_containing(
                b"refs/heads/main
refs/remotes/origin/HEAD
refs/remotes/origin/feature
"
            ),
            ["refs/heads/main", "refs/remotes/origin/feature"]
        );
    }

    #[test]
    fn reads_the_merged_branches_one_a_line() {
        assert_eq!(parse_merged(b"a\nfeature/login\n\n"), ["a", "feature/login"]);
        assert_eq!(parse_merged(b""), Vec::<String>::new());
    }
}
