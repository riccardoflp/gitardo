//! The commits of what is checked out, newest first, and which of them are not pushed yet, for the Commits pane.

use std::ffi::OsString;

use model::CommitId;
use model::commit::Commit;
use model::commits::BranchLog;

use crate::read::fields::{arguments, commit_id, read_or_nothing};
use crate::read::log_records::{log_arguments, parse_commits};
use crate::read::repo::{head_commit_arguments, parse_remotes, remotes_arguments};
use crate::{Git, GitError};

/// The most commits one read lists.
pub const MOST_COMMITS: usize = 10_000;

/// How many commits of `HEAD` to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogRequest {
    /// How many commits, at least 1 and at most [`MOST_COMMITS`]; a number outside is taken to the nearest of the two.
    pub limit: usize,
}

impl LogRequest {
    /// The number of commits asked for, between 1 and [`MOST_COMMITS`].
    #[must_use]
    pub fn count(&self) -> usize {
        self.limit.clamp(1, MOST_COMMITS)
    }
}

/// The commits of `HEAD`, newest first, whether there are more, the upstream of the branch checked out, whether the
/// repository has a remote, and which commits listed are not pushed: not in the upstream, or, without one, in no
/// remote branch. Before the first commit there are none, and no error.
///
/// Feature: commits.read-log
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_log(git: &Git, request: &LogRequest) -> Result<BranchLog, GitError> {
    let upstream = git.read_output(&upstream_arguments())?;
    // Without an upstream, or when its branch is gone, git fails to name it: there is no upstream.
    let upstream = if upstream.is_success() {
        parse_upstream(&upstream.stdout)
    } else {
        None
    };
    let has_remote = !parse_remotes(&git.run_read(&remotes_arguments())?)?.is_empty();
    if read_or_nothing(git, &head_commit_arguments())?.is_none() {
        return Ok(BranchLog {
            upstream,
            has_remote,
            ..BranchLog::default()
        });
    }
    let (commits, has_more) = parse_branch_log(&git.run_read(&branch_log_arguments(request))?, request)?;
    let unpushed = if upstream.is_some() || has_remote {
        parse_unpushed(&git.run_read(&unpushed_arguments(request, upstream.as_deref()))?)?
    } else {
        Vec::new()
    };
    Ok(BranchLog {
        commits,
        has_more,
        upstream,
        has_remote,
        unpushed,
    })
}

/// `git log -z` with the fields of every list of commits, one more than asked for, to know whether there are more.
#[must_use]
pub fn branch_log_arguments(request: &LogRequest) -> Vec<OsString> {
    let mut words = log_arguments();
    words.extend(arguments(&[
        &format!("--max-count={}", request.count() + 1),
        "HEAD",
        "--",
    ]));
    words
}

/// The commits that [`branch_log_arguments`] listed, as many as asked for, and whether git listed one more.
///
/// # Errors
///
/// When a commit cannot be read.
pub fn parse_branch_log(output: &[u8], request: &LogRequest) -> Result<(Vec<Commit>, bool), GitError> {
    let mut commits = parse_commits(output)?;
    let has_more = commits.len() > request.count();
    commits.truncate(request.count());
    Ok((commits, has_more))
}

/// `git rev-parse --symbolic-full-name @{upstream}`: the upstream of the branch checked out, by its full name.
#[must_use]
pub fn upstream_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--symbolic-full-name", "@{upstream}"])
}

/// The upstream that [`upstream_arguments`] named: `refs/remotes/origin/main`; none when it is not a full name.
#[must_use]
pub fn parse_upstream(output: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(output);
    let name = text.trim_end();
    name.starts_with("refs/").then(|| name.to_owned())
}

/// `git rev-list` of the commits of `HEAD` not in the upstream, or, without one, in no remote branch.
#[must_use]
pub fn unpushed_arguments(request: &LogRequest, upstream: Option<&str>) -> Vec<OsString> {
    let count = format!("--max-count={}", request.count());
    match upstream {
        Some(upstream) => arguments(&["rev-list", &count, "HEAD", "--not", upstream, "--"]),
        None => arguments(&["rev-list", &count, "HEAD", "--not", "--remotes", "--"]),
    }
}

/// The commits that [`unpushed_arguments`] listed, one hash per line.
///
/// # Errors
///
/// When a line is not a full hash.
pub fn parse_unpushed(output: &[u8]) -> Result<Vec<CommitId>, GitError> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| commit_id("the commits not pushed", line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LogRequest, MOST_COMMITS, parse_upstream, unpushed_arguments};

    #[test]
    fn keeps_the_limit_between_one_and_the_most() {
        assert_eq!(LogRequest { limit: 0 }.count(), 1);
        assert_eq!(LogRequest { limit: 50_000 }.count(), MOST_COMMITS);
        assert_eq!(LogRequest { limit: 100 }.count(), 100);
    }

    #[test]
    fn reads_the_upstream_only_by_its_full_name() {
        assert_eq!(
            parse_upstream(b"refs/remotes/origin/main\n").as_deref(),
            Some("refs/remotes/origin/main")
        );
        assert_eq!(parse_upstream(b"@{upstream}\n"), None);
    }

    #[test]
    fn asks_for_the_commits_in_no_remote_branch_without_an_upstream() {
        let words: Vec<String> = unpushed_arguments(&LogRequest { limit: 100 }, None)
            .iter()
            .map(|word| word.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words,
            ["rev-list", "--max-count=100", "HEAD", "--not", "--remotes", "--"]
        );
    }
}
