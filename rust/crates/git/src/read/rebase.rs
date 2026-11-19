//! What rewriting commits needs to know: where a commit was pushed, and where an interactive rebase stopped to change
//! a commit.

mod line;
mod stacked;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use model::CommitId;
use model::rebase::{RebaseStop, StopKind};

use crate::read::fields::{arguments, commit_id};
use crate::{Git, GitError};

pub use line::{
    LineRequest, commit_files_arguments, line_arguments, message_arguments, parents_arguments, parse_commit_files,
    parse_line, parse_parents, read_commit_files, read_line, read_messages,
};
pub use stacked::{
    StackedRequest, local_branches_arguments, made_again_arguments, parse_local_branches, parse_made_again,
    parse_update_refs, read_stacked_branches, update_refs_arguments,
};

/// The commit whose remote branches to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushedRequest {
    /// The commit about to be rewritten.
    pub commit: CommitId,
}

/// The remote branches that have a commit, by their short names (`origin/main`), so that the confirmation of a rewrite
/// can say the branch will have to be force pushed.
///
/// Feature: rebase.read-pushed
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_pushed(git: &Git, request: &PushedRequest) -> Result<Vec<String>, GitError> {
    Ok(parse_pushed(&git.run_read(&pushed_arguments(request))?))
}

/// `git for-each-ref --contains <hash> --format=%(refname:short) refs/remotes`.
#[must_use]
pub fn pushed_arguments(request: &PushedRequest) -> Vec<OsString> {
    arguments(&[
        "for-each-ref",
        "--contains",
        request.commit.as_str(),
        "--format=%(refname:short)",
        "refs/remotes",
    ])
}

/// The remote branches that [`pushed_arguments`] listed, without `<remote>/HEAD`, and without the names with no `/`,
/// which are `<remote>/HEAD` too, shortened to the name of the remote.
#[must_use]
pub fn parse_pushed(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|name| name.contains('/') && !name.ends_with("/HEAD"))
        .map(str::to_owned)
        .collect()
}

/// Which worktree to read the stop of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopRequest {
    /// The folder of git's files for the worktree, in full.
    pub git_folder: PathBuf,
}

/// The files an interactive rebase leaves in `rebase-merge` that tell a stop to change a commit, as they are; none for
/// a file that is not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StopFiles<'a> {
    /// `amend`: the commit git stopped on to edit, written when it stops on `edit` and left there after.
    pub amend: Option<&'a str>,
    /// `done`: the steps of the rebase done so far, the last one last.
    pub done: Option<&'a str>,
    /// `gitardo-split`: the commit being split, written by `split.commit`.
    pub split: Option<&'a str>,
}

/// Where an interactive rebase stopped to edit or to split a commit: the kind, the commit and its subject. None when no
/// rebase is under way, or when it stopped on a conflict.
///
/// Feature: rebase.read-stop
///
/// # Errors
///
/// When git cannot be started or fails, or a file names something that is not a commit.
pub fn read_stop(git: &Git, request: &StopRequest) -> Result<Option<RebaseStop>, GitError> {
    let folder = request.git_folder.join("rebase-merge");
    // A file that cannot be read is not there: git removes them when the rebase ends, maybe while this reads.
    let read = |name: &str| {
        fs::read(folder.join(name))
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    };
    let (amend, done, split) = (read("amend"), read("done"), read("gitardo-split"));
    let files = StopFiles {
        amend: amend.as_deref(),
        done: done.as_deref(),
        split: split.as_deref(),
    };
    let Some((kind, commit)) = parse_stop(&files)? else {
        return Ok(None);
    };
    let subject = parse_subject(&git.run_read(&subject_arguments(&commit))?);
    Ok(Some(RebaseStop { kind, commit, subject }))
}

/// The kind of stop and the commit, from the files of the rebase. The step done last must be the `edit` git stopped
/// on, since `amend` stays after it; a split is of the commit in `gitardo-split` when that is the commit of the step.
///
/// # Errors
///
/// When `amend` does not hold a full hash.
pub fn parse_stop(files: &StopFiles<'_>) -> Result<Option<(StopKind, CommitId)>, GitError> {
    let Some(amend) = files.amend else {
        return Ok(None);
    };
    let last_step = files
        .done
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty() && !line.starts_with('#'));
    let mut words = last_step.unwrap_or_default().split_whitespace();
    let (Some("edit" | "e"), Some(step_hash)) = (words.next(), words.next()) else {
        return Ok(None);
    };
    let split = files.split.map(str::trim).and_then(CommitId::parse);
    if let Some(split) = split.filter(|split| split.as_str().starts_with(step_hash)) {
        return Ok(Some((StopKind::Split, split)));
    }
    Ok(Some((
        StopKind::Edit,
        commit_id("the commit a rebase stopped on", amend.trim())?,
    )))
}

/// `git log -1 --format=%s <hash> --`: the subject of the commit.
#[must_use]
pub fn subject_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&["log", "-1", "--format=%s", commit.as_str(), "--"])
}

/// The subject that [`subject_arguments`] printed.
#[must_use]
pub fn parse_subject(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .trim_end_matches(['\n', '\r'])
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_pushed;

    #[test]
    fn leaves_out_the_head_of_each_remote() {
        let output = b"origin\norigin/HEAD\norigin/main\nmy/fork/feature\n";
        assert_eq!(parse_pushed(output), ["origin/main", "my/fork/feature"]);
    }
}
