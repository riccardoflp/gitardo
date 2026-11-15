//! The commits of what is checked out, newest first, and which of them are not pushed: what the Commits view of the
//! extension lists (`branch.read-log`).

use std::ffi::OsString;

use model::CommitId;
use model::commit::BranchLog;

use crate::read::fields::{arguments, commit_id, read_or_nothing};
use crate::read::log_records::{log_arguments, parse_commits};
use crate::read::repo::{parse_remotes, remotes_arguments};
use crate::{Git, GitError};

/// How many commits to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchLogRequest {
    /// The most commits listed.
    pub count: usize,
}

/// The commits of `HEAD`, newest first and `count` at most, whatever the graph is narrowed to, whether it has more,
/// and which of them are not pushed: those not in the upstream of the branch, or in no remote branch when it has none.
/// Before the first commit nothing is listed; with no remote, nothing is said of what is pushed.
///
/// Feature: branch.read-log
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_log(git: &Git, request: &BranchLogRequest) -> Result<BranchLog, GitError> {
    if read_or_nothing(git, &head_arguments())?.is_none() {
        return Ok(BranchLog::default());
    }
    let mut commits = parse_commits(&git.run_read(&log_of_head_arguments(request))?)?;
    let has_more = commits.len() > request.count;
    commits.truncate(request.count);
    let upstream_output = git.read_output(&upstream_arguments())?;
    let upstream = upstream_output
        .is_success()
        .then(|| parse_upstream(&upstream_output.stdout))
        .flatten();
    let has_remotes = !parse_remotes(&git.run_read(&remotes_arguments())?)?.is_empty();
    if commits.is_empty() || (upstream.is_none() && !has_remotes) {
        return Ok(BranchLog {
            commits,
            has_more,
            unpushed: None,
            upstream: None,
        });
    }
    let unpushed = parse_unpushed(&git.run_read(&unpushed_arguments(request, upstream.as_deref()))?)?;
    Ok(BranchLog {
        commits,
        has_more,
        unpushed: Some(unpushed),
        upstream: upstream.as_deref().map(short_name),
    })
}

/// `git rev-parse --verify --quiet HEAD^{commit}`, which prints nothing and ends with 1 before the first commit.
#[must_use]
pub fn head_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])
}

/// `git log -z` with the format of every list, one commit more than asked for (which says whether there are more),
/// from `HEAD`.
#[must_use]
pub fn log_of_head_arguments(request: &BranchLogRequest) -> Vec<OsString> {
    let mut words = log_arguments();
    words.push(OsString::from(format!(
        "--max-count={}",
        request.count.saturating_add(1)
    )));
    words.extend(arguments(&["HEAD", "--"]));
    words
}

/// `git rev-parse --symbolic-full-name @{upstream}`, which fails when the branch has no upstream, or one whose branch is
/// gone from the remote.
#[must_use]
pub fn upstream_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--symbolic-full-name", "@{upstream}"])
}

/// The full name of the upstream that [`upstream_arguments`] printed; none when it is not a reference.
#[must_use]
pub fn parse_upstream(output: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(output).trim().to_owned();
    text.starts_with("refs/").then_some(text)
}

/// `git rev-list --max-count=<count> HEAD --not <upstream> --`, or `--remotes` in place of the upstream when there is
/// none: the commits of `HEAD` not pushed, newest first.
#[must_use]
pub fn unpushed_arguments(request: &BranchLogRequest, upstream: Option<&str>) -> Vec<OsString> {
    let mut words = arguments(&["rev-list"]);
    words.push(OsString::from(format!("--max-count={}", request.count)));
    words.extend(arguments(&["HEAD", "--not", upstream.unwrap_or("--remotes"), "--"]));
    words
}

/// The commits that [`unpushed_arguments`] printed, one a line.
///
/// # Errors
///
/// When a line is not a full hash.
pub fn parse_unpushed(output: &[u8]) -> Result<Vec<CommitId>, GitError> {
    String::from_utf8_lossy(output)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| commit_id("a commit not pushed", line))
        .collect()
}

/// The short name of an upstream: `refs/remotes/origin/main` is `origin/main`, `refs/heads/main` is `main`.
fn short_name(reference: &str) -> String {
    reference
        .strip_prefix("refs/remotes/")
        .or_else(|| reference.strip_prefix("refs/heads/"))
        .unwrap_or(reference)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{BranchLogRequest, parse_unpushed, parse_upstream, short_name, unpushed_arguments};

    #[test]
    fn the_upstream_is_a_reference_or_nothing_and_is_named_short() {
        assert_eq!(
            parse_upstream(b"refs/remotes/origin/main\n").as_deref(),
            Some("refs/remotes/origin/main")
        );
        assert_eq!(parse_upstream(b"@{upstream}\n"), None);
        assert_eq!(short_name("refs/remotes/origin/main"), "origin/main");
        assert_eq!(short_name("refs/heads/main"), "main");
    }

    #[test]
    fn without_an_upstream_the_commits_are_those_in_no_remote_branch() {
        let words = unpushed_arguments(&BranchLogRequest { count: 5 }, None);
        let words: Vec<&str> = words.iter().filter_map(|word| word.to_str()).collect();
        assert_eq!(words, ["rev-list", "--max-count=5", "HEAD", "--not", "--remotes", "--"]);
        assert!(parse_unpushed(b"not a hash\n").is_err());
    }
}
