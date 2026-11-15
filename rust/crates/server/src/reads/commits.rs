//! The reads of commits: `search.read-revision`, `details.read-commit`, `commit-page.read-patch`, the four reads of a
//! comparison and `branch.read-log`.

use git::read::branch::{BranchLogRequest, read_log};
use git::read::commit_page::{PatchRequest, read_patch};
use git::read::compare::{self, ComparePatchRequest, CompareRequest, FilesRequest};
use git::read::details::{CommitRequest, read_commit};
use git::read::search::{RevisionRequest, read_revision};
use git::{Git, GitError};
use model::CommitId;
use model::state::FileChange;
use protocol::reads::{
    BranchLogJson, BranchLogParams, CommitAskedJson, CommitDetailJson, CommitDetailParams, CommitPatchParams,
    CompareFilesParams, ComparePatchParams, CompareSidesParams, CompareViewJson, PatchJson, RevisionParams,
    RevisionResult, commit_of,
};
use protocol::rpc::{ResponseError, codes};
use protocol::state::FileChangeJson;
use serde_json::Value;

use super::{Reading, answered, invalid};
use crate::session::git_failure;

/// The commit a name stands for, as a search reads it (`search.read-revision`).
pub fn search_revision(reading: &Reading<'_>, params: RevisionParams) -> Result<Value, ResponseError> {
    let read = read_revision(
        &reading.git,
        &RevisionRequest {
            revision: params.revision,
        },
    );
    answered(read, |commit| RevisionResult {
        commit: commit.as_ref().map(|commit| commit.as_str().to_owned()),
    })
}

/// The commit a side of a comparison names (`compare.resolve-revision`).
pub fn compare_revision(reading: &Reading<'_>, params: RevisionParams) -> Result<Value, ResponseError> {
    let read = compare::resolve_revision(&reading.git, &compare::RevisionRequest { text: params.revision });
    answered(read, |commit| RevisionResult {
        commit: commit.as_ref().map(|commit| commit.as_str().to_owned()),
    })
}

/// The details of a commit, a stash or the uncommitted changes (`details.read-commit`); none for a commit or a stash
/// the repository does not have.
pub fn details(reading: &Reading<'_>, params: CommitDetailParams) -> Result<Value, ResponseError> {
    let request = match params.asked {
        CommitAskedJson::Commit { commit } => CommitRequest::Commit(commit_of(&commit).map_err(invalid)?),
        CommitAskedJson::Stash { stash } => {
            match stash_commit(&reading.git, stash).map_err(|error| git_failure(&error))? {
                Some(commit) => CommitRequest::Commit(commit),
                None => return Ok(Value::Null),
            }
        }
        CommitAskedJson::Uncommitted { .. } => CommitRequest::UncommittedChanges,
    };
    answered(read_commit(&reading.git, &request), |detail| {
        detail.as_ref().map(CommitDetailJson::from)
    })
}

/// The commit of a stash, by its place in the list; none when there is no such stash.
fn stash_commit(git: &Git, index: usize) -> Result<Option<CommitId>, GitError> {
    read_revision(
        git,
        &RevisionRequest {
            revision: format!("stash@{{{index}}}"),
        },
    )
}

/// The lines a commit changed in each of its files, or in one of them (`commit-page.read-patch`), matched to the files
/// `details.read-commit` lists for it.
pub fn commit_patch(reading: &Reading<'_>, params: &CommitPatchParams) -> Result<Value, ResponseError> {
    let commit = commit_of(&params.commit).map_err(invalid)?;
    let detail = read_commit(&reading.git, &CommitRequest::Commit(commit.clone()))
        .map_err(|error| git_failure(&error))?
        .ok_or_else(|| {
            ResponseError::new(
                codes::GIT_FAILED,
                format!("{commit} is not a commit of this repository."),
            )
        })?;
    let files: Vec<FileChange> = detail
        .files
        .iter()
        .filter(|file| {
            params
                .path
                .as_deref()
                .is_none_or(|path| file.path == path || file.original_path.as_deref() == Some(path))
        })
        .cloned()
        .collect();
    let request = PatchRequest {
        commit,
        first_parent: detail.first_parent().cloned(),
        files,
    };
    answered(read_patch(&reading.git, &request), |read| PatchJson::from(read))
}

/// What two sides of a comparison have that the other has not (`compare.read-sides`).
pub fn compare_sides(reading: &Reading<'_>, params: &CompareSidesParams) -> Result<Value, ResponseError> {
    let request = CompareRequest::try_from(params).map_err(invalid)?;
    answered(compare::read_sides(&reading.git, &request), |read| {
        CompareViewJson::from(read)
    })
}

/// The files that differ between two commits (`compare.read-files`).
pub fn compare_files(reading: &Reading<'_>, params: CompareFilesParams) -> Result<Value, ResponseError> {
    let request = FilesRequest {
        from: params.from,
        to: params.to,
    };
    answered(compare::read_files(&reading.git, &request), |comparison| {
        comparison.files.iter().map(FileChangeJson::from).collect::<Vec<_>>()
    })
}

/// The lines that differ in a comparison, as lines or as text (`compare.read-patch`).
pub fn compare_patch(reading: &Reading<'_>, params: &ComparePatchParams) -> Result<Value, ResponseError> {
    let request = ComparePatchRequest {
        comparison: params.comparison().map_err(invalid)?,
        merge_base: params
            .merge_base
            .as_deref()
            .map(commit_of)
            .transpose()
            .map_err(invalid)?,
        files: params.files.iter().map(FileChange::from).collect(),
        form: params.patch_form(),
    };
    answered(compare::read_patch(&reading.git, &request), |read| {
        PatchJson::from(read)
    })
}

/// The commits of what is checked out and which of them are not pushed (`branch.read-log`).
pub fn branch_log(reading: &Reading<'_>, params: &BranchLogParams) -> Result<Value, ResponseError> {
    let read = read_log(&reading.git, &BranchLogRequest { count: params.count });
    answered(read, |read| BranchLogJson::from(read))
}
