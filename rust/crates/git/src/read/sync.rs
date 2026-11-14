//! What a force push would overwrite: the commits of the upstream that the branch checked out does not have.

use std::ffi::OsString;

use crate::read::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// How many commits are shown; one more is read to tell whether there are more.
pub const OVERWRITTEN_SHOWN: usize = 10;

/// Which upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverwrittenRequest {
    /// The upstream of the branch checked out, as git names it: `origin/main`, or `refs/remotes/origin/main`.
    pub upstream: String,
}

/// The commits a force push would throw away, as of the last fetch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Overwritten {
    /// At most [`OVERWRITTEN_SHOWN`] of them, newest first.
    pub commits: Vec<OverwrittenCommit>,
    /// Whether the upstream has more of them than those.
    pub has_more: bool,
}

/// A commit of the upstream that the branch does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverwrittenCommit {
    /// Its short hash, as git abbreviates it.
    pub short_id: String,
    /// The first line of its message.
    pub subject: String,
}

/// The commits of the upstream that the branch checked out does not have, newest first, as of the last fetch: what a
/// force push throws away.
///
/// Feature: sync.read-overwritten
///
/// # Errors
///
/// When git cannot be started or fails, as for an upstream that does not exist.
pub fn read_overwritten(git: &Git, request: &OverwrittenRequest) -> Result<Overwritten, GitError> {
    parse_overwritten(&git.run_read(&overwritten_arguments(request))?)
}

/// `git log --max-count=11 --format=%h%x00%s%x00 HEAD..<upstream> --`: the short hash and the subject of each commit,
/// one more than are shown.
#[must_use]
pub fn overwritten_arguments(request: &OverwrittenRequest) -> Vec<OsString> {
    let mut words = arguments(&["log"]);
    words.push(OsString::from(format!("--max-count={}", OVERWRITTEN_SHOWN + 1)));
    words.push(OsString::from("--format=%h%x00%s%x00"));
    words.push(OsString::from(format!("HEAD..{}", request.upstream)));
    words.push(OsString::from("--"));
    words
}

/// The commits that the log of [`overwritten_arguments`] printed: pairs of fields, each ended by a NUL, the line break
/// git puts after each commit before the next pair.
///
/// # Errors
///
/// When a commit has a hash and no subject.
pub fn parse_overwritten(output: &[u8]) -> Result<Overwritten, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0').map(|field| field.trim_start_matches('\n'));
    let mut commits = Vec::new();
    while let Some(short_id) = fields.next().filter(|field| !field.is_empty()) {
        let subject = fields
            .next()
            .ok_or_else(|| unexpected("what a force push would overwrite", short_id))?;
        commits.push(OverwrittenCommit {
            short_id: short_id.to_owned(),
            subject: subject.to_owned(),
        });
    }
    let has_more = commits.len() > OVERWRITTEN_SHOWN;
    commits.truncate(OVERWRITTEN_SHOWN);
    Ok(Overwritten { commits, has_more })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_overwritten;

    #[test]
    fn sync_read_overwritten__eleven_commits_give_ten_and_more() {
        let output: Vec<String> = (0..11)
            .map(|index| format!("abc{index:04}\0Change {index}\0\n"))
            .collect();
        let output = output.concat();
        let read = parse_overwritten(output.as_bytes()).unwrap();
        assert_eq!(read.commits.len(), 10);
        assert!(read.has_more);
        assert_eq!(read.commits[0].short_id, "abc0000");
        assert_eq!(read.commits[9].subject, "Change 9");
    }

    #[test]
    fn sync_read_overwritten__nothing_printed_is_nothing_overwritten() {
        let read = parse_overwritten(b"").unwrap();
        assert!(read.commits.is_empty() && !read.has_more);
        assert!(parse_overwritten(b"abc1234").is_err());
    }
}
