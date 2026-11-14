//! The lines a commit changed in each of its files, for its page.

use std::ffi::OsString;

use model::CommitId;
use model::commit_page::{DiffOutput, LINE_LIMIT, PatchLines, diff_lines};
use model::state::FileChange;

use crate::{Git, GitError};

/// The options of every diff whose lines are shown or copied: no colors, none of the external diff drivers or text
/// conversions of the user's configuration, a submodule as the commits it moved between, and the prefixes `a/` and
/// `b/` whatever `diff.noprefix` or `diff.mnemonicPrefix` say, so that the names can be matched to the files listed and
/// a patch copied is one `git apply` takes.
pub(crate) const PATCH_OPTIONS: [&str; 6] = [
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--submodule=short",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

/// Which commit, and the files `details.read-commit` listed for it, which the lines are matched to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRequest {
    /// The commit.
    pub commit: CommitId,
    /// Its first parent; none for a commit with no parent.
    pub first_parent: Option<CommitId>,
    /// Its files, as `details.read-commit` lists them.
    pub files: Vec<FileChange>,
}

/// `git diff -M <first parent> <commit>` with [`PATCH_OPTIONS`]; for a commit with no parent,
/// `git diff-tree --root --no-commit-id -r -p` with the same options.
#[must_use]
pub fn patch_arguments(request: &PatchRequest) -> Vec<OsString> {
    let mut words: Vec<&str> = match &request.first_parent {
        Some(_) => vec!["diff"],
        None => vec!["diff-tree", "--root", "--no-commit-id", "-r", "-p"],
    };
    words.extend(PATCH_OPTIONS);
    match &request.first_parent {
        Some(parent) => words.extend(["-M", parent.as_str(), request.commit.as_str()]),
        None => words.push(request.commit.as_str()),
    }
    words.push("--");
    words.into_iter().map(OsString::from).collect()
}

/// The lines of each file of a diff, matched to the files listed for it, with the cap of lines of a page.
#[must_use]
pub fn parse_patch(output: &[u8], files: &[FileChange]) -> PatchLines {
    diff_lines(&DiffOutput {
        files,
        output,
        limit: LINE_LIMIT,
    })
}

/// What a commit changed in each of its files, as lines, in the order and number `details.read-commit` lists them; when
/// git's output does not match those files, the lines are not shown rather than shown against the wrong files.
///
/// Feature: commit-page.read-patch
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_patch(git: &Git, request: &PatchRequest) -> Result<PatchLines, GitError> {
    let output = git.run_read(&patch_arguments(request))?;
    Ok(parse_patch(&output, &request.files))
}
