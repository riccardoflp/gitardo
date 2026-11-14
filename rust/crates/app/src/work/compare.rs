//! The reads of comparisons on a worker: a comparison page, its followed sides moved to where their names are now
//! (`compare.follow-sides`, `compare.read-sides`), and the files of the comparison in the inspector
//! (`compare.read-files`).

use std::path::Path;

use git::read::compare::{CompareRequest, FilesRequest, RevisionRequest, read_files, read_sides, resolve_revision};
use git::{CancelToken, Git, GitError};
use model::compare::{CompareSide, CompareView, ComparisonTarget, SideInput, follow_sides};
use model::state::FileChange;

/// A comparison as a worker read it: its two sides where they are now, and what differs between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonRead {
    /// The first side, moved to where its name is now when it is followed.
    pub first: CompareSide,
    /// The second side.
    pub second: CompareSide,
    /// What differs between them.
    pub view: CompareView,
}

/// Reads a comparison: each followed side that is not the working tree is moved to where its name is now, or marked
/// gone, then the two are compared as the mode says, narrowed to the path.
///
/// # Errors
///
/// What git said when it could not resolve a name or compare the two.
pub fn read_comparison(
    folder: &Path,
    target: &ComparisonTarget,
    cancel: &CancelToken,
) -> Result<ComparisonRead, String> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let failed = |error: GitError| error.to_string();
    let first = followed(&git, &target.first).map_err(failed)?;
    let second = followed(&git, &target.second).map_err(failed)?;
    let request = CompareRequest {
        first: first.side.clone(),
        second: second.side.clone(),
        mode: target.mode,
        path: target.path.clone(),
    };
    let view = read_sides(&git, &request).map_err(failed)?;
    Ok(ComparisonRead { first, second, view })
}

/// A side where its name is now, when it follows it.
fn followed(git: &Git, side: &CompareSide) -> Result<CompareSide, GitError> {
    if !side.is_followed || side.is_working_tree() {
        return Ok(side.clone());
    }
    let resolved = resolve_revision(
        git,
        &RevisionRequest {
            text: side.name.clone(),
        },
    )?;
    Ok(follow_sides(&SideInput {
        name: &side.name,
        chosen: &side.side,
        is_followed: Some(true),
        resolved: resolved.as_ref(),
    }))
}

/// The files that differ between the two commits of the comparison in the inspector.
///
/// # Errors
///
/// What git said when it could not compare them.
pub fn read_in_place(folder: &Path, request: &FilesRequest, cancel: &CancelToken) -> Result<Vec<FileChange>, String> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    read_files(&git, request)
        .map(|comparison| comparison.files)
        .map_err(|error| error.to_string())
}
