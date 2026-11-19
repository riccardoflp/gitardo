//! What a rewrite changed, commit by commit: where a branch was before its last rebase or amend, or a remote branch
//! before a fetch found it force pushed (`range-diff.read-last-rewrite`), and the commits of two lines paired as
//! `git range-diff` pairs them (`range-diff.read-pairs`).
//!
//! Git has no form of `range-diff` made for programs, so [`parse_pairs`] reads the one it prints for people, the one
//! exception rule R9.3 names (D81, Q1139).

use std::ffi::OsString;

use model::CommitId;
use model::range_diff::{Line, MOST_COMMITS, Pair, PairKind, RangeDiff, Ranges, Rewrite, RewriteKind};

use crate::read::fields::{arguments, commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError, command_line};

/// How many moves of a reference are looked through for its last rewrite.
const MOVES_READ: usize = 50;

/// The reference whose last rewrite is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastRewriteRequest {
    /// Its full name: `refs/heads/feature`, or `refs/remotes/origin/feature` for a remote branch.
    pub reference: String,
}

impl LastRewriteRequest {
    /// Whether it is a remote branch, whose rewrite is a force push a fetch found.
    fn is_remote(&self) -> bool {
        self.reference.starts_with("refs/remotes/")
    }
}

/// The arguments of the read of the last moves of a reference, newest first, each the commit it moved to and what git
/// wrote of the move.
#[must_use]
pub fn last_rewrite_arguments(request: &LastRewriteRequest) -> Vec<OsString> {
    let count = format!("--max-count={MOVES_READ}");
    arguments(&[
        "log",
        "-g",
        "-z",
        &count,
        "--format=%H%x00%gs",
        &request.reference,
        "--",
    ])
}

/// The newest rewrite among the moves read: for a local branch a rebase that finished or an amend, for a remote branch
/// a move that ends in `forced-update`. An amend is given as not of the first commit; [`read_last_rewrite`] looks.
///
/// # Errors
///
/// When a hash is not one.
pub fn parse_last_rewrite(output: &[u8], is_remote: bool) -> Result<Option<Rewrite>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0');
    let mut moves = Vec::new();
    while let (Some(commit), Some(said)) = (fields.next(), fields.next()) {
        if commit.is_empty() {
            break;
        }
        moves.push((
            commit_id("the commit of a move of a reference", commit.trim_start_matches('\n'))?,
            said,
        ));
    }
    for (index, (after, said)) in moves.iter().enumerate() {
        let Some(kind) = rewrite_kind(said, is_remote)? else {
            continue;
        };
        // The move before it is where the reference was before; past the moves read, nothing is known.
        return Ok(moves.get(index + 1).map(|(before, _)| Rewrite {
            kind,
            before: before.clone(),
            after: after.clone(),
        }));
    }
    Ok(None)
}

/// What rewrote a reference in one move, from what git wrote of it; none for a move that is not a rewrite.
fn rewrite_kind(said: &str, is_remote: bool) -> Result<Option<RewriteKind>, GitError> {
    if is_remote {
        return Ok(said.ends_with("forced-update").then_some(RewriteKind::ForcePush));
    }
    if said.starts_with("commit (amend): ") {
        return Ok(Some(RewriteKind::Amend { is_root: false }));
    }
    // `rebase (finish): refs/heads/feature onto <hash>`, `rebase -i (finish): …`, `pull --rebase (finish): …`.
    let Some((_, finished)) = said.split_once(" (finish): refs/") else {
        return Ok(None);
    };
    let Some((_, onto)) = finished.rsplit_once(" onto ") else {
        return Ok(None);
    };
    Ok(Some(RewriteKind::Rebase {
        onto: commit_id("the commit a rebase was onto", onto.trim())?,
    }))
}

/// Where a branch was before its last rebase or amend, and what the rebase was onto; or where a remote branch was
/// before a fetch found it force pushed. Nothing when none of its 50 last moves is one, or when the commit before is no
/// longer in the repository.
///
/// Feature: range-diff.read-last-rewrite
///
/// # Errors
///
/// When git cannot be run or fails, or prints a hash that is not one.
pub fn read_last_rewrite(git: &Git, request: &LastRewriteRequest) -> Result<Option<Rewrite>, GitError> {
    // A reference with no reflog makes `git log -g` fail: it has no rewrite to tell.
    let Some(output) = read_or_failure_is_nothing(git, &last_rewrite_arguments(request))? else {
        return Ok(None);
    };
    let Some(mut rewrite) = parse_last_rewrite(&output, request.is_remote())? else {
        return Ok(None);
    };
    if !exists(git, &format!("{}^{{commit}}", rewrite.before))? {
        return Ok(None);
    }
    if let RewriteKind::Amend { is_root } = &mut rewrite.kind {
        *is_root = !exists(git, &format!("{}^", rewrite.before))?;
    }
    Ok(Some(rewrite))
}

/// Whether a revision names a commit of the repository.
fn exists(git: &Git, revision: &str) -> Result<bool, GitError> {
    Ok(read_or_nothing(git, &arguments(&["rev-parse", "--verify", "--quiet", revision]))?.is_some())
}

/// What a read printed, or nothing when git failed.
fn read_or_failure_is_nothing(git: &Git, arguments: &[OsString]) -> Result<Option<Vec<u8>>, GitError> {
    match git.run_read(arguments) {
        Ok(output) => Ok(Some(output)),
        Err(GitError::Failed { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The two lines of commits to pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeDiffRequest {
    /// The two lines, as `range-diff.ranges` gives them.
    pub ranges: Ranges,
}

/// The arguments of the count of the commits of one line.
#[must_use]
pub fn count_arguments(line: &Line) -> Vec<OsString> {
    arguments(&["rev-list", "--count", &line.range(), "--"])
}

/// The arguments of `git range-diff`, its hashes in full: git takes a length past that of its hashes as the whole hash,
/// 40 characters or 64.
#[must_use]
pub fn range_diff_arguments(request: &RangeDiffRequest) -> Vec<OsString> {
    let mut words = vec!["range-diff", "--no-color", "--no-notes", "--abbrev=64"];
    words.extend(request.ranges.arguments.iter().map(String::as_str));
    arguments(&words)
}

/// The pairs of what `git range-diff` printed, each with the lines of its diff of diffs.
///
/// # Errors
///
/// When a line of a pair is not of the form `<n>:  <hash> <mark> <m>:  <hash> <subject>`.
pub fn parse_pairs(output: &[u8]) -> Result<Vec<Pair>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut pairs: Vec<Pair> = Vec::new();
    for line in text.lines() {
        if let Some(diff) = line.strip_prefix("    ") {
            match pairs.last_mut() {
                Some(pair) => pair.diff.push(diff.to_owned()),
                None => return Err(unexpected("a line of a diff of diffs before any pair", line)),
            }
        } else if !line.trim().is_empty() {
            pairs.push(pair_of(line)?);
        }
    }
    Ok(pairs)
}

/// One pair, from its line: `1:  <hash> ! 1:  <hash> <subject>`, `-:  ----- > 2:  <hash> <subject>`.
fn pair_of(line: &str) -> Result<Pair, GitError> {
    let not_a_pair = || unexpected("a pair of git range-diff", line);
    let (_, rest) = word(line).ok_or_else(not_a_pair)?;
    let (before, rest) = word(rest).ok_or_else(not_a_pair)?;
    let (mark, rest) = word(rest).ok_or_else(not_a_pair)?;
    let (_, rest) = word(rest).ok_or_else(not_a_pair)?;
    let rest = rest.trim_start();
    let (now, subject) = rest.split_once(' ').unwrap_or((rest, ""));
    let kind = match mark {
        "=" => PairKind::Same,
        "!" => PairKind::Changed,
        ">" => PairKind::Added,
        "<" => PairKind::Dropped,
        _ => return Err(not_a_pair()),
    };
    Ok(Pair {
        kind,
        before: side(before)?,
        now: side(now)?,
        subject: subject.to_owned(),
        diff: Vec::new(),
    })
}

/// The first word of a text and what follows it.
fn word(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let end = text.find(' ')?;
    Some((&text[..end], &text[end..]))
}

/// The commit of one side of a pair: none when it is a line of `-`, the side it is missing on.
fn side(hash: &str) -> Result<Option<CommitId>, GitError> {
    if hash.bytes().all(|byte| byte == b'-') {
        return Ok(None);
    }
    commit_id("a commit of a pair of git range-diff", hash).map(Some)
}

/// The commits of two lines paired as `git range-diff` pairs them, with how many commits each line has. Lines of more
/// than 1,000 commits are not compared (Q1138).
///
/// Feature: range-diff.read-pairs
///
/// # Errors
///
/// When git cannot be run or fails, when either line has more than 1,000 commits, or when a pair is not of the form
/// git prints.
pub fn read_pairs(git: &Git, request: &RangeDiffRequest) -> Result<RangeDiff, GitError> {
    let before_count = count(git, &request.ranges.before)?;
    let now_count = count(git, &request.ranges.now)?;
    if before_count > MOST_COMMITS || now_count > MOST_COMMITS {
        return Err(GitError::Refused {
            reason: format!(
                "These lines have {} and {} commits; Gitardo compares at most {} on each side.",
                with_commas(before_count),
                with_commas(now_count),
                with_commas(MOST_COMMITS)
            ),
        });
    }
    let mut pairs = parse_pairs(&git.run_read(&range_diff_arguments(request))?)?;
    with_subjects(git, &mut pairs)?;
    Ok(RangeDiff {
        pairs,
        before_count,
        now_count,
    })
}

/// The arguments of the read of the subjects of commits, their hashes given on its standard input: there may be more
/// than a command line holds.
#[must_use]
pub fn subjects_arguments() -> Vec<OsString> {
    arguments(&["log", "--no-walk=unsorted", "--stdin", "-z", "--format=%H%x00%s"])
}

/// The subjects read, each with its commit.
///
/// # Errors
///
/// When a hash is not one.
pub fn parse_subjects(output: &[u8]) -> Result<Vec<(CommitId, String)>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0');
    let mut subjects = Vec::new();
    while let (Some(commit), Some(subject)) = (fields.next(), fields.next()) {
        subjects.push((
            commit_id("the commit of a subject", commit.trim_start_matches('\n'))?,
            subject.to_owned(),
        ));
    }
    Ok(subjects)
}

/// Gives each pair the subject of its commit now, or before for one dropped: on its line git writes the subject of the
/// commit before.
fn with_subjects(git: &Git, pairs: &mut [Pair]) -> Result<(), GitError> {
    let named: Vec<&CommitId> = pairs
        .iter()
        .filter_map(|pair| pair.now.as_ref().or(pair.before.as_ref()))
        .collect();
    if named.is_empty() {
        return Ok(());
    }
    let mut input = named
        .iter()
        .map(|commit| commit.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    input.push('\n');
    let words = subjects_arguments();
    let output = git.read_output_with_input(&words, input.as_bytes())?;
    if output.code != Some(0) {
        return Err(GitError::Failed {
            command: command_line(&words),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let subjects = parse_subjects(&output.stdout)?;
    for pair in pairs {
        let commit = pair.now.as_ref().or(pair.before.as_ref());
        if let Some((_, subject)) = subjects.iter().find(|(read, _)| Some(read) == commit) {
            pair.subject.clone_from(subject);
        }
    }
    Ok(())
}

/// How many commits a line has.
fn count(git: &Git, line: &Line) -> Result<usize, GitError> {
    let output = git.run_read(&count_arguments(line))?;
    let text = String::from_utf8_lossy(&output);
    text.trim()
        .parse()
        .map_err(|_| unexpected("a count of commits", text.trim()))
}

/// A number with a comma between each group of three digits: `1,200`.
fn with_commas(number: usize) -> String {
    let digits = number.to_string();
    let mut written = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            written.push(',');
        }
        written.push(digit);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::with_commas;

    #[test]
    fn a_number_is_written_with_a_comma_between_each_group_of_three_digits() {
        assert_eq!(with_commas(7), "7");
        assert_eq!(with_commas(1_000), "1,000");
        assert_eq!(with_commas(1_234_567), "1,234,567");
    }
}
