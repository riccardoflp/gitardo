//! The commits that `git log` prints for a list: the graph, a search, the history of a file.

use std::ffi::OsString;

use model::commit::Commit;

use crate::GitError;
use crate::read::fields::{commit_id, timestamp, unexpected};

/// What is being read, for the errors.
const WHAT: &str = "a commit of the log";

/// The `--format` of every read that lists commits, given with `-z`: the hash, the parents, the author's name, address
/// and date, and the subject, separated by the unit separator.
pub const FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%s";

/// `git log -z` with [`FORMAT`], to which a read adds its own arguments.
#[must_use]
pub fn log_arguments() -> Vec<OsString> {
    ["log", "-z", FORMAT].into_iter().map(OsString::from).collect()
}

/// The commits that `git log -z` printed with [`FORMAT`], in the order printed.
///
/// # Errors
///
/// When a commit does not have its six fields, or a hash or a date cannot be read.
pub fn parse_commits(output: &[u8]) -> Result<Vec<Commit>, GitError> {
    String::from_utf8_lossy(output)
        .split('\0')
        .map(|record| record.trim_start_matches('\n'))
        .filter(|record| !record.is_empty())
        .map(commit)
        .collect()
}

/// One commit, from its six fields. The subject is the last, so that whatever it holds stays in it.
fn commit(record: &str) -> Result<Commit, GitError> {
    let mut fields = record.splitn(6, '\x1f');
    let mut next = || fields.next().ok_or_else(|| unexpected(WHAT, record));
    let id = commit_id(WHAT, next()?)?;
    let parents = next()?
        .split(' ')
        .filter(|parent| !parent.is_empty())
        .map(|parent| commit_id(WHAT, parent))
        .collect::<Result<_, _>>()?;
    let author_name = next()?.to_owned();
    let author_email = next()?.to_owned();
    let author_date = timestamp(WHAT, next()?)?;
    let subject = next()?.to_owned();
    Ok(Commit {
        id,
        parents,
        author_name,
        author_email,
        author_date,
        subject,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_commits;

    #[test]
    fn reads_a_merge_a_root_and_a_subject_with_a_tab() {
        let first = "a".repeat(40);
        let second = "b".repeat(40);
        let third = "c".repeat(40);
        let output = format!(
            "{third}\x1f{first} {second}\x1fAnn\x1fann@example.com\x1f20\x1fMerge\tthis\0\n{first}\x1f\x1fBo\x1fbo@example.com\x1f10\x1fAdd é\0"
        );
        let commits = parse_commits(output.as_bytes()).unwrap();
        assert_eq!(commits.len(), 2);
        assert!(commits[0].is_merge());
        assert_eq!(commits[0].subject, "Merge\tthis");
        assert_eq!(commits[1].parents, Vec::new());
        assert_eq!(commits[1].subject, "Add é");
    }

    #[test]
    fn refuses_a_record_with_a_field_missing() {
        assert!(parse_commits(format!("{}\x1f\x1fAnn", "a".repeat(40)).as_bytes()).is_err());
    }
}
