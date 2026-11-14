//! Patches: commits read as one patch for `git am`, and what applying a patch would do.

mod check;

use std::collections::BTreeSet;
use std::ffi::OsString;

use model::CommitId;

use crate::read::compare::parse_merge_base;
use crate::read::fields::{commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError};

pub use check::{PatchCheckRequest, PatchSource, check, check_arguments, parse_check};

/// Which commits to read as one patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatPatchRequest {
    /// The commits, in any order: the patch has them in the order of the history.
    pub commits: Vec<CommitId>,
}

/// `git format-patch --stdout` of commits given newest first, with `--no-walk=unsorted` so that git takes them as they
/// are given and writes them in the reverse order, oldest first, and `--max-count` so that a single commit is not read
/// as where to start from up to `HEAD`. No external diff driver or text conversion is used.
#[must_use]
pub fn format_patch_arguments(newest_first: &[CommitId]) -> Vec<OsString> {
    let count = format!("--max-count={}", newest_first.len());
    let mut arguments: Vec<OsString> = [
        "format-patch",
        "--stdout",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--submodule=short",
        &count,
        "--no-walk=unsorted",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    arguments.extend(newest_first.iter().map(|commit| OsString::from(commit.as_str())));
    arguments.push(OsString::from("--"));
    arguments
}

/// `git merge-base --octopus` of the commits: the commit all of them come from, below which the history need not be
/// walked to put them in order.
#[must_use]
pub fn base_arguments(commits: &[CommitId]) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("merge-base"), OsString::from("--octopus")];
    arguments.extend(commits.iter().map(|commit| OsString::from(commit.as_str())));
    arguments
}

/// `git rev-list --topo-order` of the commits down to the commit they come from, which is listed too: the commits in the
/// order of the history, newest first, children before their parents.
#[must_use]
pub fn history_order_arguments(commits: &[CommitId], base: Option<&CommitId>) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("rev-list"), OsString::from("--topo-order")];
    arguments.extend(commits.iter().map(|commit| OsString::from(commit.as_str())));
    if let Some(base) = base {
        arguments.push(OsString::from("--not"));
        arguments.push(OsString::from(format!("{base}^@")));
    }
    arguments.push(OsString::from("--"));
    arguments
}

/// The commits that `git rev-list` printed that are among those asked for, in the order printed.
///
/// # Errors
///
/// When a line is not a hash.
pub fn parse_history_order(output: &[u8], asked: &[CommitId]) -> Result<Vec<CommitId>, GitError> {
    let asked: BTreeSet<&CommitId> = asked.iter().collect();
    let text = String::from_utf8_lossy(output);
    let mut ordered = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let commit = commit_id("the order of commits", line)?;
        if asked.contains(&commit) && !ordered.contains(&commit) {
            ordered.push(commit);
        }
    }
    Ok(ordered)
}

/// One or more commits as one patch, oldest first whatever the order they are asked in, each with its author, date and
/// message, as `git am` takes it. Git makes no patch of a merge commit: the patch then has fewer commits than were asked
/// for, which `patch.count-commits` tells.
///
/// Feature: patch.read-format-patch
///
/// # Errors
///
/// When git cannot be started, fails, or lists commits this does not understand.
pub fn read_format_patch(git: &Git, request: &FormatPatchRequest) -> Result<String, GitError> {
    let newest_first = in_history_order(git, &request.commits)?;
    let output = git.run_read(&format_patch_arguments(&newest_first))?;
    Ok(String::from_utf8(output).unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()))
}

/// The commits in the order of the history, newest first.
fn in_history_order(git: &Git, commits: &[CommitId]) -> Result<Vec<CommitId>, GitError> {
    if commits.len() < 2 {
        return Ok(commits.to_vec());
    }
    let base = match read_or_nothing(git, &base_arguments(commits))? {
        Some(output) => parse_merge_base(&output)?,
        None => None,
    };
    let ordered = parse_history_order(
        &git.run_read(&history_order_arguments(commits, base.as_ref()))?,
        commits,
    )?;
    if ordered.len() == commits.iter().collect::<BTreeSet<_>>().len() {
        Ok(ordered)
    } else {
        Err(unexpected(
            "the order of commits",
            "a commit asked for is not in the history",
        ))
    }
}
