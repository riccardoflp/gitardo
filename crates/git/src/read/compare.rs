//! Comparisons: which commit a revision is, everything the comparison page shows of two sides, their lines, and the
//! files that differ between two commits for the comparison in place of the details.

mod parse;
mod patch;
mod sides;

use std::ffi::OsString;

use model::CommitId;
use model::compare::{CompareMode, Comparison, Side};

use crate::read::changed_files::parse_name_status;
use crate::read::fields::{arguments, read_or_nothing};
use crate::{Git, GitError};

pub use parse::{Counts, parse_cherry_marks, parse_compared_commits, parse_counts, parse_merge_base, parse_merge_tree};
pub use patch::{ComparePatchRequest, PatchForm, compare_patch_arguments, read_patch};
pub use sides::{
    Revisions, cherry_arguments, commits_arguments, compare_files_arguments, counts_arguments, merge_base_arguments,
    merge_tree_arguments, read_sides,
};

/// What the user typed or chose, to be turned into the commit it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRequest {
    /// A hash, a branch, a tag, `HEAD~3`, `v1.0^`, `main@{yesterday}`…
    pub text: String,
}

/// The two sides of a comparison, how they are compared, and the file or folder it is narrowed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareRequest {
    /// The first side.
    pub first: Side,
    /// The second side.
    pub second: Side,
    /// How they are compared.
    pub mode: CompareMode,
    /// The file or folder it is narrowed to, from the top of the repository; none for the whole repository.
    pub path: Option<String>,
}

/// The two commits the comparison in place of the details compares, as their full hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesRequest {
    /// The commit the comparison goes from.
    pub from: String,
    /// The commit the comparison goes to.
    pub to: String,
}

/// The message of a comparison whose sides are not commits of the repository, or are both the working tree.
pub const NOT_COMMITS: &str = "These are not commits of this repository.";

/// `git rev-parse --verify --quiet <revision>^{commit}`: the commit a revision stands for, a tag peeled to its commit.
#[must_use]
pub fn revision_arguments(request: &RevisionRequest) -> Vec<OsString> {
    let object = format!("{}^{{commit}}", request.text);
    arguments(&["rev-parse", "--verify", "--quiet", &object])
}

/// The commit a revision stands for: a hash, a branch, a tag (an annotated one peeled to its commit), `HEAD~3`, `v1.0^`
/// or `main@{yesterday}`; none for a name git does not know. An empty text, or one that starts with `-`, is none
/// without running git, which would read it as an option.
///
/// Feature: compare.resolve-revision
///
/// # Errors
///
/// When git cannot be started, or fails otherwise than by not knowing the name.
pub fn resolve_revision(git: &Git, request: &RevisionRequest) -> Result<Option<CommitId>, GitError> {
    if request.text.is_empty() || request.text.starts_with('-') {
        return Ok(None);
    }
    let Some(output) = read_or_nothing(git, &revision_arguments(request))? else {
        return Ok(None);
    };
    parse_merge_base(&output)
}

/// `git diff --name-status -z -M <from> <to>`.
#[must_use]
pub fn files_between_arguments(from: &CommitId, to: &CommitId) -> Vec<OsString> {
    arguments(&["diff", "--name-status", "-z", "-M", from.as_str(), to.as_str(), "--"])
}

/// The files that differ between two commits, renames with both names. Sides that are not full hashes give nothing,
/// without running git.
///
/// Feature: compare.read-files
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_files(git: &Git, request: &FilesRequest) -> Result<Comparison, GitError> {
    let (Some(from), Some(to)) = (CommitId::parse(&request.from), CommitId::parse(&request.to)) else {
        return Ok(Comparison::default());
    };
    let files = parse_name_status(&git.run_read(&files_between_arguments(&from, &to))?)?;
    Ok(Comparison { files })
}
