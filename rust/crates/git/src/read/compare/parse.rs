//! What git prints for a comparison, read.

use std::collections::BTreeSet;

use model::CommitId;
use model::compare::{CommitSide, ComparedCommit, MergeOutlook};

use crate::read::fields::{commit_id, timestamp, unexpected};
use crate::{GitError, Output};

/// What is being read, for the errors.
const WHAT: &str = "a comparison";

/// How many commits each side has that the other has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// The commits only in the first side.
    pub only_in_first: usize,
    /// The commits only in the second side.
    pub only_in_second: usize,
}

impl Counts {
    /// How many commits the two have between them.
    #[must_use]
    pub fn total(&self) -> usize {
        self.only_in_first + self.only_in_second
    }
}

/// The one hash a command printed, as `git merge-base` and `git rev-parse --verify` print it; none when it printed
/// nothing.
///
/// # Errors
///
/// When what it printed is not a hash.
pub fn parse_merge_base(output: &[u8]) -> Result<Option<CommitId>, GitError> {
    let text = String::from_utf8_lossy(output);
    let line = text.lines().next().unwrap_or_default().trim();
    if line.is_empty() {
        return Ok(None);
    }
    commit_id(WHAT, line).map(Some)
}

/// What `git rev-list --left-right --count <second>...<first>` printed: the commits only in the second side, a tab,
/// and the commits only in the first.
///
/// # Errors
///
/// When it is not two numbers.
pub fn parse_counts(output: &[u8]) -> Result<Counts, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut numbers = text.split_whitespace().map(str::parse::<usize>);
    match (numbers.next(), numbers.next(), numbers.next()) {
        (Some(Ok(only_in_second)), Some(Ok(only_in_first)), None) => Ok(Counts {
            only_in_first,
            only_in_second,
        }),
        _ => Err(unexpected(WHAT, &text)),
    }
}

/// The commits that `git log --left-right` printed with the format of the comparison: each record its side mark (`<`
/// for the first side, `>` for the second), hash, author's name and date, and subject, ended by the record separator.
///
/// # Errors
///
/// When a record does not have its five fields, or its mark, hash or date cannot be read.
pub fn parse_compared_commits(output: &[u8]) -> Result<Vec<ComparedCommit>, GitError> {
    String::from_utf8_lossy(output)
        .split('\x1e')
        .map(|record| record.trim_start_matches('\n'))
        .filter(|record| !record.is_empty())
        .map(compared_commit)
        .collect()
}

/// One commit, from its five fields. The subject is the last, so that whatever it holds stays in it.
fn compared_commit(record: &str) -> Result<ComparedCommit, GitError> {
    let mut fields = record.splitn(5, '\x1f');
    let mut next = || fields.next().ok_or_else(|| unexpected(WHAT, record));
    let side = match next()? {
        "<" => CommitSide::First,
        ">" => CommitSide::Second,
        _ => return Err(unexpected(WHAT, record)),
    };
    Ok(ComparedCommit {
        side,
        id: commit_id(WHAT, next()?)?,
        author_name: next()?.to_owned(),
        author_date: timestamp(WHAT, next()?)?,
        subject: next()?.to_owned(),
        is_also_in_other_side: false,
    })
}

/// The commits that `git rev-list --cherry-mark` marked with `=`: their change is on the other side too.
///
/// # Errors
///
/// When a line is not a mark and a hash.
pub fn parse_cherry_marks(output: &[u8]) -> Result<BTreeSet<CommitId>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut marked = BTreeSet::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        match line.split_at_checked(1) {
            Some(("=", hash)) => {
                let _ = marked.insert(commit_id(WHAT, hash)?);
            }
            Some(("+", hash)) => {
                let _ = commit_id(WHAT, hash)?;
            }
            _ => return Err(unexpected(WHAT, line)),
        }
    }
    Ok(marked)
}

/// What `git merge-tree --write-tree --no-messages --name-only -z` says of a merge by how it ended: 0 for no conflict,
/// 1 for the files after the tree that would conflict, each once, and any other code, as with a git older than 2.40 or
/// histories that never meet, for a merge that cannot be told.
#[must_use]
pub fn parse_merge_tree(output: &Output) -> MergeOutlook {
    match output.code {
        Some(0) => MergeOutlook::Clean,
        Some(1) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut files: Vec<String> = Vec::new();
            for name in text.split('\0').skip(1).filter(|name| !name.is_empty()) {
                if !files.iter().any(|file| file == name) {
                    files.push(name.to_owned());
                }
            }
            MergeOutlook::Conflicts(files)
        }
        _ => MergeOutlook::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_cherry_marks, parse_counts, parse_merge_tree};
    use crate::Output;
    use model::compare::MergeOutlook;

    #[test]
    fn reads_counts_marks_and_the_outcome_of_a_merge() {
        let counts = parse_counts(b"2\t3\n").unwrap();
        assert_eq!((counts.only_in_first, counts.only_in_second), (3, 2));
        assert!(parse_counts(b"2\n").is_err());
        let hash = "a".repeat(40);
        assert_eq!(parse_cherry_marks(format!("+{hash}\n").as_bytes()).unwrap().len(), 0);
        assert_eq!(parse_cherry_marks(format!("={hash}\n").as_bytes()).unwrap().len(), 1);
        assert!(parse_cherry_marks(b"?x\n").is_err());
        let ended = |code, stdout: &[u8]| Output {
            code: Some(code),
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(parse_merge_tree(&ended(0, b"tree\0")), MergeOutlook::Clean);
        assert_eq!(
            parse_merge_tree(&ended(1, b"tree\0a b.txt\0a b.txt\0c\0\0")),
            MergeOutlook::Conflicts(vec![String::from("a b.txt"), String::from("c")])
        );
        assert_eq!(parse_merge_tree(&ended(129, b"")), MergeOutlook::Unknown);
    }
}
