//! What the places that list the files of a commit open in the Diff page: a file between the first parent and the
//! commit, all of them in one page, and an untracked file of a stash against nothing.

use model::details::CommitDetail;
use model::state::FileChange;

use super::{DiffFile, DiffTarget, DiffVersion};

/// The two versions the files of a commit are compared between: its first parent, or nothing for a root commit, and
/// the commit; for the uncommitted changes, `HEAD` and the working tree.
#[must_use]
pub fn commit_sides(detail: &CommitDetail) -> (DiffVersion, DiffVersion) {
    match &detail.id {
        Some(commit) => (
            detail
                .first_parent()
                .cloned()
                .map_or(DiffVersion::Empty, DiffVersion::commit),
            DiffVersion::commit(commit.clone()),
        ),
        None => (DiffVersion::Head, DiffVersion::WorkingTree),
    }
}

/// A file as a commit lists it, under its old path on the left when it was renamed.
#[must_use]
pub fn listed_file(file: &FileChange) -> DiffFile {
    DiffFile {
        left: file.original_path.clone().unwrap_or_else(|| file.path.clone()),
        right: file.path.clone(),
    }
}

/// The Diff page of a file of a commit, between its first parent and the commit.
#[must_use]
pub fn commit_file(detail: &CommitDetail, file: &FileChange) -> DiffTarget {
    let (left, right) = commit_sides(detail);
    DiffTarget::file(left, right, listed_file(file))
}

/// The Diff page of every file of a commit, one after the other, under a title.
#[must_use]
pub fn commit_files(detail: &CommitDetail, title: String) -> DiffTarget {
    let (left, right) = commit_sides(detail);
    DiffTarget {
        left,
        right,
        files: detail.files.iter().map(listed_file).collect(),
        title: Some(title),
    }
}

/// The Diff page of an untracked file a stash took, against nothing: all its lines added. Git keeps those files in the
/// third parent of the stash.
#[must_use]
pub fn untracked_file(detail: &CommitDetail, file: &FileChange) -> Option<DiffTarget> {
    let untracked = detail.parents.get(2)?.clone();
    let name = detail
        .id
        .as_ref()
        .map_or_else(|| untracked.short().to_owned(), |stash| stash.short().to_owned());
    let right = DiffVersion::Commit {
        commit: untracked,
        name,
    };
    Some(DiffTarget::file(DiffVersion::Empty, right, DiffFile::same(&file.path)))
}
