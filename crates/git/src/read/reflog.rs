//! The last entries of the reflog of a reference, for Undo.

use std::ffi::OsString;

use model::CommitId;
use model::contract::ReflogEntry;

use crate::{Git, GitError};

/// Which reflog, and how much of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflogRequest {
    /// `HEAD`, or a reference in full: `refs/heads/main`.
    pub reference: String,
    /// How many entries, at most.
    pub count: usize,
}

/// The last entries of a reflog, newest first. A reference with no reflog yet has none.
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_entries(git: &Git, request: &ReflogRequest) -> Result<Vec<ReflogEntry>, GitError> {
    let output = git.read_output(&entries_arguments(request))?;
    if !output.is_success() {
        // A branch with no commit yet, or a reference without a reflog, has nothing to undo: git says so by failing.
        return Ok(Vec::new());
    }
    parse_entries(&output.stdout)
}

/// `git log --walk-reflogs`, one entry per record, the hash and the subject apart.
fn entries_arguments(request: &ReflogRequest) -> Vec<OsString> {
    let count = format!("--max-count={}", request.count);
    [
        "log",
        "--walk-reflogs",
        "-z",
        "--format=%H%x1f%gs",
        count.as_str(),
        request.reference.as_str(),
        "--",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// The entries of `git log --walk-reflogs -z --format=%H%x1f%gs`.
///
/// # Errors
///
/// When a record has no hash.
pub fn parse_entries(output: &[u8]) -> Result<Vec<ReflogEntry>, GitError> {
    let text = String::from_utf8_lossy(output);
    text.split('\0')
        .map(|record| record.trim_start_matches('\n'))
        .filter(|record| !record.is_empty())
        .map(|record| {
            let (hash, subject) = record.split_once('\x1f').unwrap_or((record, ""));
            let commit = CommitId::parse(hash).ok_or_else(|| GitError::Unexpected {
                what: "the reflog",
                detail: record.to_owned(),
            })?;
            Ok(ReflogEntry {
                commit,
                subject: subject.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ReflogRequest, parse_entries, read_entries};
    use crate::Git;
    use testkit::Repository;

    #[test]
    fn reads_the_entries_newest_first() {
        let mut repository = Repository::new();
        let first = repository.commit("Add the parser", &[("parser.rs", "fn parse() {}\n")]);
        let second = repository.commit("Fix the parser", &[("parser.rs", "fn parse() { }\n")]);
        let git = Git::new(repository.path());
        let request = ReflogRequest {
            reference: String::from("HEAD"),
            count: 2,
        };
        let entries = read_entries(&git, &request).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].commit.as_str(), second);
        assert_eq!(entries[0].subject, "commit: Fix the parser");
        assert_eq!(entries[1].commit.as_str(), first);
        assert_eq!(entries[1].subject, "commit (initial): Add the parser");
    }

    #[test]
    fn has_no_entries_before_the_first_commit() {
        let repository = Repository::new();
        let request = ReflogRequest {
            reference: String::from("HEAD"),
            count: 2,
        };
        assert_eq!(
            read_entries(&Git::new(repository.path()), &request).unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn refuses_a_record_without_a_hash() {
        assert!(parse_entries(b"nothing\x1fcommit: x\0").is_err());
        assert_eq!(parse_entries(b"").unwrap(), Vec::new());
    }
}
