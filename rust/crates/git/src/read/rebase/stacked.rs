//! The other local branches on the commits a rebase makes again, and git's own `rebase.updateRefs`
//! (`rebase.read-stacked-branches`).

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

use model::rebase::{StackedBranch, StackedBranches};

use crate::read::fields::{arguments, commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError};

/// The rebase about to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackedRequest {
    /// What the rebase starts from: the commit or the branch it is onto, or the base of its todo list; `None` when an
    /// interactive rebase starts at the root.
    pub base: Option<String>,
    /// The branch rewritten, by its short name, or the commit of a detached `HEAD`.
    pub tip: String,
}

/// The branches, other than the one rewritten, whose tip is one of the commits the rebase makes again, each with the
/// worktree that has it checked out; and git's own `rebase.updateRefs`.
///
/// Feature: rebase.read-stacked-branches
///
/// # Errors
///
/// When git cannot be started or fails, or prints what is not a hash.
pub fn read_stacked_branches(git: &Git, request: &StackedRequest) -> Result<StackedBranches, GitError> {
    let made_again = parse_made_again(&git.run_read(&made_again_arguments(request))?)?;
    let branches = parse_local_branches(&git.run_read(&local_branches_arguments())?)?
        .into_iter()
        .filter(|branch| branch.name != request.tip && made_again.contains(branch.commit.as_str()))
        .collect();
    let update_refs = read_or_nothing(git, &update_refs_arguments())?
        .map(|output| parse_update_refs(&output))
        .transpose()?;
    Ok(StackedBranches { branches, update_refs })
}

/// `git rev-list <base>..<tip> --`, or `git rev-list <tip> --` from the root: the commits the rebase makes again.
#[must_use]
pub fn made_again_arguments(request: &StackedRequest) -> Vec<OsString> {
    let range = match &request.base {
        Some(base) => format!("{base}..{}", request.tip),
        None => request.tip.clone(),
    };
    arguments(&["rev-list", &range, "--"])
}

/// The hashes [`made_again_arguments`] printed.
///
/// # Errors
///
/// When a line is not a full hash.
pub fn parse_made_again(output: &[u8]) -> Result<HashSet<String>, GitError> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|line| commit_id("a commit made again", line).map(|commit| commit.as_str().to_owned()))
        .collect()
}

/// `git for-each-ref --format=%(refname)%00%(objectname)%00%(worktreepath)%00 refs/heads`: each local branch, its
/// commit, and the folder of the worktree that has it checked out.
#[must_use]
pub fn local_branches_arguments() -> Vec<OsString> {
    arguments(&[
        "for-each-ref",
        "--format=%(refname)%00%(objectname)%00%(worktreepath)%00",
        "refs/heads",
    ])
}

/// The branches [`local_branches_arguments`] printed, one a line.
///
/// # Errors
///
/// When a line does not have its three fields, or its commit is not a full hash.
pub fn parse_local_branches(output: &[u8]) -> Result<Vec<StackedBranch>, GitError> {
    const WHAT: &str = "a local branch and its worktree";
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\0');
            let mut next = || fields.next().ok_or_else(|| unexpected(WHAT, line));
            let reference = next()?;
            let name = reference
                .strip_prefix("refs/heads/")
                .ok_or_else(|| unexpected(WHAT, line))?;
            let commit = commit_id(WHAT, next()?)?;
            let folder = next()?;
            Ok(StackedBranch {
                name: name.to_owned(),
                commit,
                worktree: (!folder.is_empty()).then(|| PathBuf::from(folder)),
            })
        })
        .collect()
}

/// `git config --type=bool --get rebase.updateRefs`, which ends with 1 when it is not set.
#[must_use]
pub fn update_refs_arguments() -> Vec<OsString> {
    arguments(&["config", "--type=bool", "--get", "rebase.updateRefs"])
}

/// The setting [`update_refs_arguments`] printed: `true` or `false`.
///
/// # Errors
///
/// When it is neither.
pub fn parse_update_refs(output: &[u8]) -> Result<bool, GitError> {
    match String::from_utf8_lossy(output).trim_end() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(unexpected("rebase.updateRefs", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::{StackedRequest, made_again_arguments, parse_local_branches, parse_update_refs};

    #[test]
    fn the_range_starts_at_the_base_or_at_the_root() {
        let onto = StackedRequest {
            base: Some(String::from("main")),
            tip: String::from("p3"),
        };
        assert_eq!(made_again_arguments(&onto), ["rev-list", "main..p3", "--"]);
        let root = StackedRequest { base: None, ..onto };
        assert_eq!(made_again_arguments(&root), ["rev-list", "p3", "--"]);
    }

    #[test]
    fn reads_each_branch_with_its_worktree_or_none() {
        let hash = "a".repeat(40);
        let output = format!("refs/heads/p1\0{hash}\0\0\nrefs/heads/p2\0{hash}\0C:/work/p2\0\n");
        let branches = parse_local_branches(output.as_bytes()).unwrap();
        assert_eq!(branches[0].name, "p1");
        assert_eq!(branches[0].worktree, None);
        assert_eq!(branches[1].worktree, Some(std::path::PathBuf::from("C:/work/p2")));
        assert!(parse_update_refs(b"true\n").unwrap());
        assert!(!parse_update_refs(b"false\n").unwrap());
    }
}
