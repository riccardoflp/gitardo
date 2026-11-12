//! The groups that `git blame --porcelain` and `git blame --incremental` print: a line that names a commit and the lines
//! of the file that are its, then the details of that commit the first time it is named.

use std::collections::BTreeMap;
use std::sync::Arc;

use model::blame::{BlameCommit, BlameLine, PreviousVersion};
use model::{CommitId, Timestamp};

use crate::GitError;
use crate::read::fields::{commit_id, optional_commit_id, timestamp, unexpected};

/// What is being read, for the errors.
const WHAT: &str = "the blame";

/// A group: the lines of the file, from 1, that a commit is the last to have changed. None for the commit is a line
/// not committed yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Group {
    /// The commit; none for lines not committed yet.
    pub(super) commit: Option<CommitId>,
    /// The first line of the file that is the commit's, from 1.
    pub(super) first_line: usize,
    /// How many lines from it are.
    pub(super) count: usize,
}

/// The details of a commit, as git gives them the first time it names it.
#[derive(Debug, Clone, Default)]
struct Details {
    author: String,
    author_time: Timestamp,
    summary: String,
    previous: Option<PreviousVersion>,
}

/// What a blame says: its groups in the order printed, and who each commit is.
#[derive(Debug, Clone, Default)]
pub(super) struct Blame {
    /// The groups, in the order git printed them.
    pub(super) groups: Vec<Group>,
    /// Each commit named, by its hash, shared by every line of it.
    pub(super) commits: BTreeMap<CommitId, Arc<BlameCommit>>,
}

impl Blame {
    /// Who last changed each line of a group: the commit, or nobody yet.
    pub(super) fn line_of(&self, group: &Group) -> Option<BlameLine> {
        match &group.commit {
            None => Some(BlameLine::NotCommitted),
            Some(commit) => self.commits.get(commit).cloned().map(BlameLine::Committed),
        }
    }
}

/// The groups of a blame and the commits they name. The lines of the file that the porcelain format prints, which
/// start with a tab, are passed over.
pub(super) fn parse_groups(output: &[u8]) -> Result<Blame, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut groups = Vec::new();
    let mut details: BTreeMap<CommitId, Details> = BTreeMap::new();
    let mut current: Option<CommitId> = None;
    for line in text
        .split('\n')
        .filter(|line| !line.is_empty() && !line.starts_with('\t'))
    {
        if let Some(group) = group(line)? {
            current.clone_from(&group.commit);
            if let Some(commit) = &group.commit {
                let _ = details.entry(commit.clone()).or_default();
            }
            groups.push(group);
        } else if let Some(commit) = &current
            && let Some(described) = details.get_mut(commit)
        {
            describe(described, line)?;
        }
    }
    let commits = details
        .into_iter()
        .map(|(id, details)| {
            let commit = BlameCommit {
                id: id.clone(),
                author: details.author,
                author_time: details.author_time,
                summary: details.summary,
                previous: details.previous,
            };
            (id, Arc::new(commit))
        })
        .collect();
    Ok(Blame { groups, commits })
}

/// The group a line starts, `<hash> <line in the commit> <line now> <count>`, or none for a line of details. The
/// porcelain format leaves out the count on the lines of a group after its first.
fn group(line: &str) -> Result<Option<Group>, GitError> {
    let words: Vec<&str> = line.split(' ').collect();
    let (Some(hash), Some(_line_in_commit), Some(line_now)) = (words.first(), words.get(1), words.get(2)) else {
        return Ok(None);
    };
    let is_hash = matches!(hash.len(), 40 | 64) && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !is_hash || words.len() > 4 {
        return Ok(None);
    }
    let number = |word: &str| word.parse::<usize>().map_err(|_| unexpected(WHAT, line));
    Ok(Some(Group {
        commit: optional_commit_id(WHAT, hash)?,
        first_line: number(line_now)?,
        count: words.get(3).map_or(Ok(1), |count| number(count))?,
    }))
}

/// Takes a line of details into the details of a commit: `author`, `author-time`, `summary` and `previous`.
fn describe(details: &mut Details, line: &str) -> Result<(), GitError> {
    let (key, value) = line.split_once(' ').unwrap_or((line, ""));
    match key {
        "author" => value.clone_into(&mut details.author),
        "author-time" => details.author_time = timestamp(WHAT, value)?,
        "summary" => value.trim_end_matches('\r').clone_into(&mut details.summary),
        "previous" => {
            let (hash, path) = value.split_once(' ').ok_or_else(|| unexpected(WHAT, line))?;
            details.previous = Some(PreviousVersion {
                commit: commit_id(WHAT, hash)?,
                path: unquoted(path),
            });
        }
        // The other details, the committer, the time zones, the file name and `boundary`, are not shown.
        _ => {}
    }
    Ok(())
}

/// A file name as git writes it: as it is, or, when it holds a quote, a backslash or a control character, in double
/// quotes with C escapes, the bytes of other characters in octal.
pub(super) fn unquoted(name: &str) -> String {
    let Some(inner) = name.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) else {
        return name.to_owned();
    };
    let mut bytes = Vec::with_capacity(inner.len());
    let mut rest = inner.bytes().peekable();
    while let Some(byte) = rest.next() {
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        let Some(escaped) = rest.next() else { break };
        let plain = match escaped {
            b'a' => 0x07,
            b'b' => 0x08,
            b't' => b'\t',
            b'n' => b'\n',
            b'v' => 0x0b,
            b'f' => 0x0c,
            b'r' => b'\r',
            b'0'..=b'7' => {
                let mut value = u32::from(escaped - b'0');
                for _ in 0..2 {
                    if let Some(digit) = rest.next_if(|digit| (b'0'..=b'7').contains(digit)) {
                        value = value * 8 + u32::from(digit - b'0');
                    }
                }
                u8::try_from(value).unwrap_or(b'?')
            }
            other => other,
        };
        bytes.push(plain);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{parse_groups, unquoted};

    #[test]
    fn unquotes_the_names_git_quotes() {
        assert_eq!(unquoted("src/a b.rs"), "src/a b.rs");
        assert_eq!(unquoted(r#""say \"hi\".txt""#), "say \"hi\".txt");
        assert_eq!(unquoted(r#""tab\there\\.txt""#), "tab\there\\.txt");
        assert_eq!(unquoted(r#""caff\303\250.txt""#), "caffè.txt");
    }

    #[test]
    fn passes_over_the_lines_of_the_file_and_the_details_not_shown() {
        let hash = "a".repeat(40);
        let output = format!(
            "{hash} 1 1 2\nauthor Ann\nauthor-time 10\nsummary Add\nboundary\nfilename a.rs\n\tfirst\n{hash} 2 2\n\tsecond\n"
        );
        let blame = parse_groups(output.as_bytes()).unwrap();
        assert_eq!(blame.groups.len(), 2);
        assert_eq!(blame.commits.len(), 1);
        assert!(
            blame
                .commits
                .values()
                .all(|commit| commit.summary == "Add" && commit.previous.is_none())
        );
    }
}
