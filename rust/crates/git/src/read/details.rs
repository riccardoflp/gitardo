//! The details of one commit: its message, author, committer, dates, parents and the files it changed, and, for a
//! stash, the untracked files it took.

use std::collections::BTreeMap;
use std::ffi::OsString;

use model::CommitId;
use model::details::{CommitDetail, Signature, UNCOMMITTED_SUBJECT};
use model::state::{ChangeKind, FileChange, WorkingTree};

use crate::read::changed_files::parse_name_status;
use crate::read::existing::read_unless_missing;
use crate::read::fields::{arguments, commit_id, timestamp, unexpected};
use crate::read::repo::{parse_status, status_arguments};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the details of a commit";

/// What the subject of the third parent of a stash starts with when that parent holds its untracked files.
const UNTRACKED_SUBJECT: &str = "untracked files on ";

/// Which commit to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRequest {
    /// A commit, or a stash, by its hash.
    Commit(CommitId),
    /// The uncommitted changes, as the row above the commits shows them.
    UncommittedChanges,
}

/// What `git show --no-patch` says of a commit, before its files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHeader {
    /// Its full hash.
    pub id: CommitId,
    /// Its parents, the first one first.
    pub parents: Vec<CommitId>,
    /// Who made it, and when.
    pub author: Signature,
    /// Who committed it, and when.
    pub committer: Signature,
    /// Its message as written, without the newline at its end.
    pub message: String,
}

/// `git show --no-patch` of the commit with, separated by the unit separator, its hash, its parents, its author's
/// name, address and date, its committer's, and its raw message. `^{commit}` makes anything that is not a commit fail.
#[must_use]
pub fn commit_arguments(commit: &CommitId) -> Vec<OsString> {
    let object = format!("{commit}^{{commit}}");
    arguments(&[
        "show",
        "--no-patch",
        "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%B",
        &object,
        "--",
    ])
}

/// The files of a commit: `git diff --name-status -z -M <first parent> <commit>`, or for a commit with no parent
/// `git diff-tree --root --no-commit-id -r --name-status -z <commit>`.
#[must_use]
pub fn files_arguments(commit: &CommitId, parents: &[CommitId]) -> Vec<OsString> {
    match parents.first() {
        Some(parent) => arguments(&[
            "diff",
            "--name-status",
            "-z",
            "-M",
            parent.as_str(),
            commit.as_str(),
            "--",
        ]),
        None => root_files_arguments(commit),
    }
}

/// `git show --no-patch --format=%P%x1f%s` of the third parent of a stash, which says whether it holds the untracked
/// files.
#[must_use]
pub fn third_parent_arguments(third_parent: &CommitId) -> Vec<OsString> {
    arguments(&["show", "--no-patch", "--format=%P%x1f%s", third_parent.as_str(), "--"])
}

/// What `git show` printed with the format of [`commit_arguments`].
///
/// # Errors
///
/// When a field is missing, or a hash or a date cannot be read.
pub fn parse_commit(output: &[u8]) -> Result<CommitHeader, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.splitn(9, '\x1f');
    let mut next = || fields.next().ok_or_else(|| unexpected(WHAT, &text));
    let id = commit_id(WHAT, next()?.trim_start_matches('\n'))?;
    let parents = next()?
        .split(' ')
        .filter(|parent| !parent.is_empty())
        .map(|parent| commit_id(WHAT, parent))
        .collect::<Result<_, _>>()?;
    let author = signature(&mut next)?;
    let committer = signature(&mut next)?;
    let message = next()?.trim_end_matches('\n').to_owned();
    Ok(CommitHeader {
        id,
        parents,
        author,
        committer,
        message,
    })
}

/// Whether the third parent of a stash holds its untracked files, from what [`third_parent_arguments`] printed.
#[must_use]
pub fn parse_third_parent(output: &[u8]) -> bool {
    let text = String::from_utf8_lossy(output);
    text.split_once('\x1f')
        .is_some_and(|(_, subject)| subject.starts_with(UNTRACKED_SUBJECT))
}

/// Everything the details, the Stashes pane and the page of a commit show of one commit: its message,
/// author, committer, dates and parents, its note (`note.read`), the files it changed against its first parent (every file it added when it has
/// none), and for a stash with three parents the untracked files it took. The uncommitted changes asked for as a commit
/// have the subject **Uncommitted changes**, no author, and the files of the index and the working tree together. A
/// hash that is not a commit of the repository has no detail.
///
/// Feature: details.read-commit
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_commit(git: &Git, request: &CommitRequest) -> Result<Option<CommitDetail>, GitError> {
    match request {
        CommitRequest::Commit(commit) => read_one_commit(git, commit),
        CommitRequest::UncommittedChanges => {
            let working_tree = parse_status(&git.run_read(&status_arguments())?)?;
            Ok(Some(uncommitted_detail(&working_tree)))
        }
    }
}

/// The detail of a commit, none when it is not a commit of the repository.
fn read_one_commit(git: &Git, commit: &CommitId) -> Result<Option<CommitDetail>, GitError> {
    let object = format!("{commit}^{{commit}}");
    let Some(output) = read_unless_missing(git, &commit_arguments(commit), &object)? else {
        return Ok(None);
    };
    let header = parse_commit(&output)?;
    let files = parse_name_status(&git.run_read(&files_arguments(&header.id, &header.parents))?)?;
    let untracked_files = match header.parents.get(2) {
        Some(third) if parse_third_parent(&git.run_read(&third_parent_arguments(third))?) => {
            parse_name_status(&git.run_read(&root_files_arguments(third))?)?
        }
        _ => Vec::new(),
    };
    let note = crate::read::note::read_text(git, &header.id)?;
    Ok(Some(CommitDetail {
        id: Some(header.id),
        parents: header.parents,
        author: Some(header.author),
        committer: Some(header.committer),
        message: header.message,
        files,
        untracked_files,
        note,
    }))
}

/// `git diff-tree --root --no-commit-id -r --name-status -z <commit>`: every file of a commit with no parent.
fn root_files_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&[
        "diff-tree",
        "--root",
        "--no-commit-id",
        "-r",
        "--name-status",
        "-z",
        commit.as_str(),
        "--",
    ])
}

/// A name, an address and a date, the next three fields.
fn signature<'text>(next: &mut impl FnMut() -> Result<&'text str, GitError>) -> Result<Signature, GitError> {
    let name = next()?.to_owned();
    let email = next()?.to_owned();
    let date = timestamp(WHAT, next()?)?;
    Ok(Signature { name, email, date })
}

/// The uncommitted changes as a commit: each file of the index and of the working tree once, by path. A file staged
/// keeps what the index did to it unless it is now gone from the disk; an untracked file is added; a file in conflict
/// is changed.
fn uncommitted_detail(working_tree: &WorkingTree) -> CommitDetail {
    let mut files: BTreeMap<&str, FileChange> = BTreeMap::new();
    for file in &working_tree.staged {
        let _ = files.insert(&file.path, file.clone());
    }
    for file in &working_tree.unstaged {
        let replaces = files
            .get(file.path.as_str())
            .is_none_or(|_| file.kind == ChangeKind::Deleted);
        if replaces {
            let _ = files.insert(&file.path, file.clone());
        }
    }
    let changed = |path: &String, kind: ChangeKind| FileChange {
        path: path.clone(),
        original_path: None,
        kind,
    };
    for path in &working_tree.untracked {
        let _ = files.insert(path, changed(path, ChangeKind::Added));
    }
    for file in &working_tree.conflicted {
        let _ = files.insert(&file.path, changed(&file.path, ChangeKind::Modified));
    }
    CommitDetail {
        id: None,
        parents: Vec::new(),
        author: None,
        committer: None,
        message: UNCOMMITTED_SUBJECT.to_owned(),
        files: files.into_values().collect(),
        untracked_files: Vec::new(),
        note: None,
    }
}
