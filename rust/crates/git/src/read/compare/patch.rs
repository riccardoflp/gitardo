//! The lines of a comparison, for its page, and the same as a patch to copy.

use std::ffi::OsString;

use model::CommitId;
use model::compare::{ComparePatch, Side};
use model::state::FileChange;

use super::CompareRequest;
use super::sides::{not_commits, pathspec, range};
use crate::read::commit_page::{PATCH_OPTIONS, parse_patch};
use crate::{Git, GitError};

/// Which form the changes are read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchForm {
    /// The lines of each file, for the page.
    Lines,
    /// A patch to copy, binary files included.
    Text,
}

/// The comparison, where its sides diverged as `compare.read-sides` found, the files it listed, and the form wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparePatchRequest {
    /// The comparison.
    pub comparison: CompareRequest,
    /// Where the two sides diverged, as `compare.read-sides` found.
    pub merge_base: Option<CommitId>,
    /// The files `compare.read-sides` listed, which the lines are matched to.
    pub files: Vec<FileChange>,
    /// The lines, or the patch to copy.
    pub form: PatchForm,
}

/// `git diff -M` with the options of every diff whose lines are shown, from `<since>` to the second side as for the
/// files of `compare.read-sides`, narrowed as the comparison is; `--binary` for the patch to copy.
#[must_use]
pub fn compare_patch_arguments(request: &ComparePatchRequest) -> Vec<OsString> {
    let comparison = &request.comparison;
    let is_reversed = comparison.first == Side::WorkingTree;
    let mut arguments: Vec<OsString> = vec![OsString::from("diff")];
    arguments.extend(PATCH_OPTIONS.into_iter().map(|option| {
        // `-R` swaps the prefixes too: given swapped, the header still says `a/<before> b/<after>`.
        match option {
            "--src-prefix=a/" if is_reversed => OsString::from("--src-prefix=b/"),
            "--dst-prefix=b/" if is_reversed => OsString::from("--dst-prefix=a/"),
            other => OsString::from(other),
        }
    }));
    if request.form == PatchForm::Text {
        arguments.push(OsString::from("--binary"));
    }
    arguments.push(OsString::from("-M"));
    arguments.extend(
        range(comparison, request.merge_base.as_ref())
            .into_iter()
            .map(OsString::from),
    );
    arguments.extend(pathspec(comparison.path.as_deref()));
    arguments
}

/// What changed in each file of a comparison: as lines matched to the files `compare.read-sides` listed, in their order
/// and number, with the cap of 20,000 lines; or as a patch that `git apply` takes whole, binary files included.
///
/// Feature: compare.read-patch
///
/// # Errors
///
/// Refuses two sides that are both the working tree; fails when git cannot be started or fails.
pub fn read_patch(git: &Git, request: &ComparePatchRequest) -> Result<ComparePatch, GitError> {
    let comparison = &request.comparison;
    if comparison.first == Side::WorkingTree && comparison.second == Side::WorkingTree {
        return Err(not_commits());
    }
    let output = git.run_read(&compare_patch_arguments(request))?;
    Ok(match request.form {
        PatchForm::Lines => ComparePatch::Lines(parse_patch(&output, &request.files)),
        PatchForm::Text => ComparePatch::Text(output),
    })
}
