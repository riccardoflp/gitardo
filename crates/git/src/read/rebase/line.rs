//! What a rewrite reads before it runs: the parents of a commit, the line of commits from it up to `HEAD` with their
//! messages, the message of a commit, and the files a commit changes.

use std::ffi::OsString;

use model::CommitId;
use model::rebase::{LineCommit, LineToHead};

use crate::read::branch::ancestor_arguments;
use crate::read::fields::{arguments, commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError};

/// The commit whose line to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRequest {
    /// The commit a rewrite starts at.
    pub commit: CommitId,
}

/// The parents of a commit, and the commits from it up to `HEAD`, oldest first, with their messages, when they are a
/// single line of first parents without a merge.
///
/// # Errors
///
/// When git cannot be started or fails, for instance when the hash is not a commit.
pub fn read_line(git: &Git, request: &LineRequest) -> Result<LineToHead, GitError> {
    let parents = parse_parents(&git.run_read(&parents_arguments(&request.commit))?)?;
    let is_ancestor = read_or_nothing(git, &ancestor_arguments(request.commit.as_str(), "HEAD"))?.is_some();
    if !is_ancestor || parents.len() > 1 {
        return Ok(LineToHead { parents, commits: None });
    }
    let output = git.run_read(&line_arguments(&parents))?;
    let commits = parse_line(&output, &request.commit)?;
    Ok(LineToHead { parents, commits })
}

/// `git rev-list --parents -n 1 <hash> --`: the commit, then its parents.
#[must_use]
pub fn parents_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&["rev-list", "--parents", "-n", "1", commit.as_str(), "--"])
}

/// The parents that [`parents_arguments`] printed after the commit.
///
/// # Errors
///
/// When a word is not a full hash.
pub fn parse_parents(output: &[u8]) -> Result<Vec<CommitId>, GitError> {
    String::from_utf8_lossy(output)
        .split_whitespace()
        .skip(1)
        .map(|word| commit_id("a parent of a commit", word))
        .collect()
}

/// `git log -z --topo-order --format=%H%x00%P%x00%B HEAD --not <parents> --`: the commits of `HEAD` that the parents
/// of the commit do not have, newest first, each with its parents and its message; all of them for the first commit.
#[must_use]
pub fn line_arguments(parents: &[CommitId]) -> Vec<OsString> {
    let mut words = arguments(&["log", "-z", "--topo-order", "--format=%H%x00%P%x00%B", "HEAD", "--not"]);
    words.extend(parents.iter().map(|parent| OsString::from(parent.as_str())));
    words.push(OsString::from("--"));
    words
}

/// The commits that [`line_arguments`] printed, oldest first, when each is the only parent of the one after it and the
/// oldest is the commit asked about; none otherwise.
///
/// # Errors
///
/// When a hash is not a full hash, or a commit has fewer fields than three.
pub fn parse_line(output: &[u8], commit: &CommitId) -> Result<Option<Vec<LineCommit>>, GitError> {
    let text = String::from_utf8_lossy(output);
    let fields: Vec<&str> = text.trim_end_matches('\0').split('\0').collect();
    let (entries, rest) = fields.as_chunks::<3>();
    if !rest.is_empty() {
        return Err(unexpected("the line of commits before HEAD", &text));
    }
    let mut line = Vec::new();
    let mut expected: Option<CommitId> = None;
    for [id, parents, message] in entries {
        let id = commit_id("a commit of the line before HEAD", id.trim_start_matches('\n'))?;
        if expected.as_ref().is_some_and(|expected| *expected != id) {
            return Ok(None);
        }
        let parents: Vec<&str> = parents.split_whitespace().collect();
        let is_asked = id == *commit;
        if !is_asked && parents.len() != 1 {
            return Ok(None);
        }
        expected = parents
            .first()
            .map(|parent| commit_id("a parent of a commit", parent))
            .transpose()?;
        line.push(LineCommit {
            id,
            message: (*message).to_owned(),
        });
        if is_asked {
            line.reverse();
            return Ok(Some(line));
        }
    }
    Ok(None)
}

/// The whole message of each commit, in the order given.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_messages(git: &Git, commits: &[CommitId]) -> Result<Vec<String>, GitError> {
    commits
        .iter()
        .map(|commit| Ok(String::from_utf8_lossy(&git.run_read(&message_arguments(commit))?).into_owned()))
        .collect()
}

/// `git log -1 --format=%B <hash> --`.
#[must_use]
pub fn message_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&["log", "-1", "--format=%B", commit.as_str(), "--"])
}

/// The files a commit changes, both names of one renamed.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_commit_files(git: &Git, commit: &CommitId) -> Result<Vec<String>, GitError> {
    Ok(parse_commit_files(&git.run_read(&commit_files_arguments(commit))?))
}

/// `git diff-tree -r --no-commit-id --name-only --no-renames -z --root <hash>`.
#[must_use]
pub fn commit_files_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&[
        "diff-tree",
        "-r",
        "--no-commit-id",
        "--name-only",
        "--no-renames",
        "-z",
        "--root",
        commit.as_str(),
    ])
}

/// The paths that [`commit_files_arguments`] printed.
#[must_use]
pub fn parse_commit_files(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_commit_files, parse_line, parse_parents};
    use model::CommitId;

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap()
    }

    fn entry(digit: char, parents: &str, message: &str) -> String {
        let parents: Vec<String> = parents.chars().map(|parent| id(parent).to_string()).collect();
        format!("{}\0{}\0{message}\0", id(digit), parents.join(" "))
    }

    #[test]
    fn reads_the_line_oldest_first_down_to_the_commit() {
        let output = [entry('c', "b", "Third\n"), entry('b', "a", "Second\n\nBody\n")].concat();
        let line = parse_line(output.as_bytes(), &id('b')).unwrap().unwrap();
        assert_eq!(line.len(), 2);
        assert_eq!(line.first().unwrap().id, id('b'));
        assert_eq!(line.first().unwrap().message, "Second\n\nBody\n");
        assert_eq!(line.get(1).unwrap().id, id('c'));
    }

    #[test]
    fn gives_nothing_past_a_merge_or_a_break() {
        let merge = [entry('c', "b9", "Merge\n"), entry('b', "a", "Second\n")].concat();
        assert_eq!(parse_line(merge.as_bytes(), &id('b')).unwrap(), None);
        let broken = [entry('c', "9", "Third\n"), entry('b', "a", "Second\n")].concat();
        assert_eq!(parse_line(broken.as_bytes(), &id('b')).unwrap(), None);
        let root = entry('a', "", "First\n");
        assert_eq!(parse_line(root.as_bytes(), &id('a')).unwrap().unwrap().len(), 1);
    }

    #[test]
    fn reads_the_parents_and_the_files() {
        let parents = format!("{} {} {}\n", id('c'), id('a'), id('b'));
        assert_eq!(parse_parents(parents.as_bytes()).unwrap(), [id('a'), id('b')]);
        assert_eq!(parse_commit_files(b"a.txt\0dir/b c.txt\0"), ["a.txt", "dir/b c.txt"]);
    }
}
