//! What is checked out: the commit, and the branch by its full name.

use std::ffi::OsString;

use model::CommitId;

use crate::read::fields::{arguments, commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError};

/// `git rev-parse --verify --quiet HEAD`: the commit checked out, or nothing and the exit code 1 before the first commit.
#[must_use]
pub fn head_commit_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--verify", "--quiet", "HEAD"])
}

/// The commit that `git rev-parse --verify --quiet HEAD` printed.
///
/// # Errors
///
/// When it is not a full hash.
pub fn parse_head_commit(output: &[u8]) -> Result<CommitId, GitError> {
    commit_id("the commit checked out", String::from_utf8_lossy(output).trim_end())
}

/// `git symbolic-ref --quiet HEAD`: the branch checked out by its full name, or nothing and the exit code 1 when `HEAD`
/// is detached. The full name, as a branch and a tag can share a name.
#[must_use]
pub fn head_branch_arguments() -> Vec<OsString> {
    arguments(&["symbolic-ref", "--quiet", "HEAD"])
}

/// The branch that `git symbolic-ref --quiet HEAD` printed, by its full name: `refs/heads/main`.
///
/// # Errors
///
/// When it is not the full name of a reference.
pub fn parse_head_branch(output: &[u8]) -> Result<String, GitError> {
    let text = String::from_utf8_lossy(output);
    let branch = text.trim_end_matches('\n');
    if branch.starts_with("refs/") {
        Ok(branch.to_owned())
    } else {
        Err(unexpected("the branch checked out", branch))
    }
}

/// The commit checked out, none before the first commit.
pub(super) fn read_head_commit(git: &Git) -> Result<Option<CommitId>, GitError> {
    read_or_nothing(git, &head_commit_arguments())?
        .map(|output| parse_head_commit(&output))
        .transpose()
}

/// The branch checked out, none when `HEAD` is detached.
pub(super) fn read_head_branch(git: &Git) -> Result<Option<String>, GitError> {
    read_or_nothing(git, &head_branch_arguments())?
        .map(|output| parse_head_branch(&output))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{parse_head_branch, parse_head_commit};

    #[test]
    fn reads_a_full_hash_and_refuses_anything_else() {
        let hash = "21fd002bf0706ec8e4f1da3386bafa7701182c07";
        assert_eq!(
            parse_head_commit(format!("{hash}\n").as_bytes()).unwrap().as_str(),
            hash
        );
        assert!(parse_head_commit(b"21fd002\n").is_err());
        assert!(parse_head_commit(b"").is_err());
    }

    #[test]
    fn reads_the_branch_by_its_full_name() {
        assert_eq!(parse_head_branch(b"refs/heads/main\n").unwrap(), "refs/heads/main");
        assert_eq!(
            parse_head_branch(b"refs/heads/caff\xc3\xa8\n").unwrap(),
            "refs/heads/caffè"
        );
        assert!(parse_head_branch(b"main\n").is_err());
    }
}
