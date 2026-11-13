//! What the operations that make commits again read before they decide: commits by name, the line of commits after a
//! commit up to `HEAD`, the whole message of a commit, and the stop of a cherry-pick or a revert with nothing left to
//! do.

use std::ffi::OsString;

use model::commit::Commit;
use model::contract::EmptyStop;
use model::state::Operation;

use crate::read::fields::{arguments, commit_id, read_or_nothing};
use crate::read::log_records::{log_arguments, parse_commits};
use crate::{Git, GitError, command_line};

/// What is being read, for the errors.
const WHAT: &str = "the commit a cherry-pick stopped on";

/// Commits, each with its parents and its subject, in the order given.
///
/// # Errors
///
/// When git cannot be started, knows no commit by one of the names, or prints what this does not understand.
pub fn read_commits(git: &Git, revisions: &[String]) -> Result<Vec<Commit>, GitError> {
    parse_commits(&git.run_read(&commits_arguments(revisions))?)
}

/// `git log -z --format=… --no-walk=unsorted <revision>… --`: the commits named, and only them, in the order given.
#[must_use]
pub fn commits_arguments(revisions: &[String]) -> Vec<OsString> {
    let mut words = log_arguments();
    words.push(OsString::from("--no-walk=unsorted"));
    words.extend(revisions.iter().map(OsString::from));
    words.push(OsString::from("--"));
    words
}

/// The commits after a commit up to `HEAD`, newest first, each with its parents.
///
/// # Errors
///
/// When git cannot be started, knows no such commit, or prints what this does not understand.
pub fn read_line_after(git: &Git, commit: &model::CommitId) -> Result<Vec<Commit>, GitError> {
    parse_commits(&git.run_read(&line_after_arguments(commit))?)
}

/// `git log -z --format=… --topo-order <commit>..HEAD --`.
#[must_use]
pub fn line_after_arguments(commit: &model::CommitId) -> Vec<OsString> {
    let mut words = log_arguments();
    words.push(OsString::from("--topo-order"));
    words.push(OsString::from(format!("{commit}..HEAD")));
    words.push(OsString::from("--"));
    words
}

/// The whole message of a commit, without the line break git ends it with.
///
/// # Errors
///
/// When git cannot be started or knows no such commit.
pub fn read_message(git: &Git, commit: &model::CommitId) -> Result<String, GitError> {
    Ok(parse_message(&git.run_read(&message_arguments(commit))?))
}

/// `git log -1 --format=%B <commit> --`.
#[must_use]
pub fn message_arguments(commit: &model::CommitId) -> Vec<OsString> {
    arguments(&["log", "-1", "--format=%B", commit.as_str(), "--"])
}

/// The message [`message_arguments`] printed, without the line breaks at its end.
#[must_use]
pub fn parse_message(output: &[u8]) -> String {
    String::from_utf8_lossy(output).trim_end_matches(['\n', '\r']).to_owned()
}

/// The commit a cherry-pick or a revert stopped on, when nothing is left to do on it: `CHERRY_PICK_HEAD` (or
/// `REVERT_HEAD`) is there, no file is left in conflict, and the tracked files, in the index and on disk, are as in
/// `HEAD`, so that skipping it loses nothing.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_empty_stop(git: &Git) -> Result<Option<EmptyStop>, GitError> {
    for (name, operation) in [
        ("CHERRY_PICK_HEAD", Operation::CherryPick),
        ("REVERT_HEAD", Operation::Revert),
    ] {
        let Some(printed) = read_or_nothing(git, &stopped_on_arguments(name))? else {
            continue;
        };
        let commit = commit_id(WHAT, String::from_utf8_lossy(&printed).trim())?;
        if !git.run_read(&unmerged_arguments())?.is_empty() || !is_like_head(git)? {
            return Ok(None);
        }
        return Ok(Some(EmptyStop { operation, commit }));
    }
    Ok(None)
}

/// `git rev-parse -q --verify <CHERRY_PICK_HEAD or REVERT_HEAD>`.
#[must_use]
pub fn stopped_on_arguments(name: &str) -> Vec<OsString> {
    arguments(&["rev-parse", "-q", "--verify", name])
}

/// `git ls-files -u -z`: the files left in conflict, which must be none.
#[must_use]
pub fn unmerged_arguments() -> Vec<OsString> {
    arguments(&["ls-files", "-u", "-z"])
}

/// `git diff --quiet HEAD --`: exit 0 when the tracked files, in the index and on disk, are as `HEAD` has them.
#[must_use]
pub fn like_head_arguments() -> Vec<OsString> {
    arguments(&["diff", "--quiet", "HEAD", "--"])
}

/// Whether the tracked files are as `HEAD` has them.
fn is_like_head(git: &Git) -> Result<bool, GitError> {
    let words = like_head_arguments();
    let output = git.read_output(&words)?;
    match output.code {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        code => Err(GitError::Failed {
            command: command_line(&words),
            code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{commits_arguments, line_after_arguments, parse_message};
    use model::CommitId;

    #[test]
    fn names_the_commits_in_the_order_given_and_the_line_up_to_head() {
        let words = |list: Vec<std::ffi::OsString>| {
            list.iter()
                .map(|word| word.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };
        let read = words(commits_arguments(&[String::from("b"), String::from("a")]));
        assert_eq!(read.get(3..).unwrap(), ["--no-walk=unsorted", "b", "a", "--"]);
        let commit = CommitId::parse(&"a".repeat(40)).unwrap();
        let line = words(line_after_arguments(&commit));
        assert_eq!(
            line.get(3..).unwrap(),
            ["--topo-order", &format!("{commit}..HEAD"), "--"]
        );
    }

    #[test]
    fn keeps_the_body_of_a_message_and_drops_the_line_breaks_after_it() {
        assert_eq!(parse_message(b"Subject\n\nBody.\n\n"), "Subject\n\nBody.");
    }
}
