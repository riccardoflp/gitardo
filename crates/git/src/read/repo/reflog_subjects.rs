//! Where `HEAD` has been: the subjects of the last entries of its reflog, for `refs.recent`.

use std::ffi::OsString;

use crate::read::repo::fields::arguments;
use crate::{Git, GitError};

/// How many entries of the reflog are read.
const ENTRIES: &str = "--max-count=300";

/// `git reflog --max-count=300 --format=%gs HEAD --`: one subject per line, as git writes a reflog subject on one line.
#[must_use]
pub fn reflog_subjects_arguments() -> Vec<OsString> {
    arguments(&["reflog", ENTRIES, "--format=%gs", "HEAD", "--"])
}

/// The subjects that `git reflog --format=%gs` printed, newest first: `checkout: moving from main to feature`.
///
/// # Errors
///
/// None: every line is a subject. The `Result` keeps the shape of every parser of a read.
pub fn parse_reflog_subjects(output: &[u8]) -> Result<Vec<String>, GitError> {
    Ok(String::from_utf8_lossy(output)
        .split('\n')
        .filter(|subject| !subject.is_empty())
        .map(str::to_owned)
        .collect())
}

/// The subjects of the reflog of `HEAD`, none before the first commit.
pub(super) fn read_reflog_subjects(git: &Git) -> Result<Vec<String>, GitError> {
    let output = git.read_output(&reflog_subjects_arguments())?;
    if !output.is_success() {
        // Before the first commit `HEAD` has no reflog, and git says so by failing: there is nothing to read.
        return Ok(Vec::new());
    }
    parse_reflog_subjects(&output.stdout)
}
