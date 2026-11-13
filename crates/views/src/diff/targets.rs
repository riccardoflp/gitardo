//! What the places that list the files of a commit open in the Diff page: a file between the first parent and the
//! commit, all of them in one page, an untracked file of a stash against nothing, and a file compared with a side
//! chosen for it (`details.compare-file-with`).

use model::changes::{OpenVersion, PageRequest};
use model::compare::{CompareSide, FileSides, Side};
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

/// The Diff page a file of the Changes page opens, from what `changes.open` decided; none for another page, or for a
/// side the Diff page cannot show yet (what an editor has not saved).
#[must_use]
pub fn changes_file(request: &PageRequest) -> Option<DiffTarget> {
    let PageRequest::Diff {
        left,
        right,
        left_path,
        right_path,
    } = request
    else {
        return None;
    };
    let file = DiffFile {
        left: left_path.clone(),
        right: right_path.clone(),
    };
    Some(DiffTarget::file(version_of(*left)?, version_of(*right)?, file))
}

/// The version of a side of the Diff page.
fn version_of(version: OpenVersion) -> Option<DiffVersion> {
    match version {
        OpenVersion::Head => Some(DiffVersion::Head),
        OpenVersion::Index => Some(DiffVersion::Index),
        OpenVersion::WorkingTree => Some(DiffVersion::WorkingTree),
        OpenVersion::Nothing => Some(DiffVersion::Empty),
        OpenVersion::Buffer => None,
    }
}

/// The Diff page of a file between two sides of a comparison, each named as the comparison names it.
#[must_use]
pub fn file_sides(sides: &FileSides) -> DiffTarget {
    DiffTarget::file(
        side_version(&sides.left),
        side_version(&sides.right),
        DiffFile::same(&sides.path),
    )
}

/// The version of a side of a comparison.
fn side_version(side: &CompareSide) -> DiffVersion {
    match &side.side {
        Side::WorkingTree => DiffVersion::WorkingTree,
        Side::Commit(commit) => DiffVersion::Commit {
            commit: commit.clone(),
            name: side.name.clone(),
        },
    }
}
