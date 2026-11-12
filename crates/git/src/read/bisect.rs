//! The bisect under way: the commits marked good, bad and skipped, and how many can still be the first bad one.

use std::ffi::OsString;
use std::path::PathBuf;

use model::CommitId;
use model::bisect::Bisect;

use crate::read::fields::{arguments, commit_id, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the bisect";

/// The file git writes when a bisect starts and removes when it ends.
const BISECT_START: &str = "BISECT_START";

/// Which worktree to read the bisect of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BisectRequest {
    /// The folder of git's files for the worktree, in full: a bisect belongs to one worktree.
    pub git_folder: PathBuf,
}

/// The bisect under way in a worktree: its marks, and how many commits can still be the first bad one once a bad and
/// a good commit are marked. Without `BISECT_START` no bisect is under way, whatever marks are left behind.
///
/// Feature: bisect.read
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read(git: &Git, request: &BisectRequest) -> Result<Option<Bisect>, GitError> {
    if !request.git_folder.join(BISECT_START).is_file() {
        return Ok(None);
    }
    let mut bisect = parse_marks(&git.run_read(&marks_arguments())?)?;
    if let Some(words) = remaining_arguments(&bisect) {
        bisect.remaining = Some(parse_remaining(&git.run_read(&words)?)?);
    }
    Ok(Some(bisect))
}

/// `git for-each-ref --format=%(refname) %(objectname) refs/bisect`: each mark, one per line.
#[must_use]
pub fn marks_arguments() -> Vec<OsString> {
    arguments(&["for-each-ref", "--format=%(refname) %(objectname)", "refs/bisect"])
}

/// The marks that [`marks_arguments`] listed: `refs/bisect/bad`, `refs/bisect/good-*` and `refs/bisect/skip-*`. Other
/// references under `refs/bisect` are passed over.
///
/// # Errors
///
/// When a line has no hash.
pub fn parse_marks(output: &[u8]) -> Result<Bisect, GitError> {
    let mut bisect = Bisect::default();
    for line in String::from_utf8_lossy(output).lines().filter(|line| !line.is_empty()) {
        let (name, hash) = line.split_once(' ').ok_or_else(|| unexpected(WHAT, line))?;
        let commit = commit_id(WHAT, hash)?;
        let Some(mark) = name.strip_prefix("refs/bisect/") else {
            continue;
        };
        if mark == "bad" {
            bisect.bad = Some(commit);
        } else if mark.starts_with("good-") {
            bisect.good.push(commit);
        } else if mark.starts_with("skip-") {
            bisect.skipped.push(commit);
        }
    }
    Ok(bisect)
}

/// `git rev-list --count <bad> --not <good>…`, with a bad commit and at least one good one; none otherwise.
#[must_use]
pub fn remaining_arguments(bisect: &Bisect) -> Option<Vec<OsString>> {
    let bad = bisect.bad.as_ref()?;
    if bisect.good.is_empty() {
        return None;
    }
    let mut words = arguments(&["rev-list", "--count", bad.as_str(), "--not"]);
    words.extend(bisect.good.iter().map(|good: &CommitId| OsString::from(good.as_str())));
    Some(words)
}

/// The count that [`remaining_arguments`] printed.
///
/// # Errors
///
/// When it is not a number.
pub fn parse_remaining(output: &[u8]) -> Result<u32, GitError> {
    let text = String::from_utf8_lossy(output);
    text.trim().parse().map_err(|_| unexpected(WHAT, &text))
}
