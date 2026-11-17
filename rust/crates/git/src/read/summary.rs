//! What a repository or a worktree is on (`repo.worktree-summary`): its branch or the commit it is detached at, how many
//! files are changed there, and how many commits are to pull and to push, from one `git status`.

use std::ffi::OsString;
use std::path::Path;

use model::repo::{SummaryHead, WorktreeSummary};

use crate::read::fields::{arguments, optional_commit_id, unexpected};
use crate::read::repo::status::parse_status;
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "what a worktree is on";

/// `git status --porcelain=v2 --branch -z --untracked-files=normal`: an untracked folder counts as one file.
#[must_use]
pub fn summary_arguments() -> Vec<OsString> {
    arguments(&["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=normal"])
}

/// What a folder is on, read with git in it; a folder that is gone is `missing`, without running git.
///
/// # Errors
///
/// When git fails in a folder that is there, or prints what is not documented: the caller keeps the summary before,
/// muted.
///
/// Feature: repo.worktree-summary
pub fn worktree_summary(folder: &Path) -> Result<WorktreeSummary, GitError> {
    if !folder.is_dir() {
        return Ok(WorktreeSummary::missing());
    }
    parse_summary(&Git::new(folder).run_read(&summary_arguments())?)
}

/// The summary `git status --porcelain=v2 --branch -z` printed: the branch from `# branch.head`, the commit from
/// `# branch.oid` when it is detached, ahead and behind from `# branch.ab`, and the files from the count of the other
/// lines.
///
/// # Errors
///
/// When a line is of no kind git documents, or a count is not a number.
pub fn parse_summary(output: &[u8]) -> Result<WorktreeSummary, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut branch = None;
    let mut commit = None;
    let mut upstream = None;
    for field in text.split('\0') {
        if let Some(oid) = field.strip_prefix("# branch.oid ") {
            commit = if oid == "(initial)" {
                None
            } else {
                optional_commit_id(WHAT, oid)?
            };
        } else if let Some(head) = field.strip_prefix("# branch.head ") {
            branch = (head != "(detached)").then(|| head.to_owned());
        } else if let Some(counts) = field.strip_prefix("# branch.ab ") {
            upstream = Some(ahead_behind(counts).ok_or_else(|| unexpected(WHAT, field))?);
        }
    }
    let head = match (branch, commit) {
        (Some(name), Some(_)) => SummaryHead::Branch(name),
        (None, Some(commit)) => SummaryHead::Detached(commit.short().to_owned()),
        (_, None) => SummaryHead::NoCommits,
    };
    let changed = parse_status(output)?.file_count();
    Ok(WorktreeSummary {
        head,
        changed: u32::try_from(changed).unwrap_or(u32::MAX),
        upstream,
    })
}

/// `+1 -2`: one commit ahead, two behind.
fn ahead_behind(counts: &str) -> Option<(u32, u32)> {
    let (ahead, behind) = counts.split_once(' ')?;
    Some((
        ahead.strip_prefix('+')?.parse().ok()?,
        behind.strip_prefix('-')?.parse().ok()?,
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::repo::SummaryHead;

    use super::parse_summary;

    #[test]
    fn parses_the_branch_the_counts_and_a_branch_with_no_commit() {
        let commit = "3f2a1c9".to_owned() + &"0".repeat(33);
        let output = format!(
            "# branch.oid {commit}\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +0 -2\0\
             ? a.txt\0? b.txt\0"
        );
        let summary = parse_summary(output.as_bytes()).unwrap();
        assert_eq!(summary.head, SummaryHead::Branch(String::from("main")));
        assert_eq!((summary.changed, summary.ahead(), summary.behind()), (2, 0, 2));
        let first = parse_summary(b"# branch.oid (initial)\0# branch.head main\0").unwrap();
        assert_eq!((first.head, first.upstream), (SummaryHead::NoCommits, None));
        assert!(parse_summary(b"# branch.oid (initial)\0# branch.ab +x -1\0").is_err());
    }
}
