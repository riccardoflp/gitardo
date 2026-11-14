//! `changes.list`: the one list of the Changes page, each file once, as it is next to the last commit, with how much of
//! it is staged.

use std::collections::BTreeMap;

use crate::state::{ChangeKind, ConflictedFile, FileChange, WorkingTree};

/// What the list is made from: what git says is not committed, and the files with edits not saved
/// (`changes.unsaved`), by their paths from the top of the working tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangesInput<'a> {
    /// What git says.
    pub working_tree: &'a WorkingTree,
    /// The files with edits not saved, sorted.
    pub unsaved: &'a [String],
}

/// The list of the Changes page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ChangesList {
    /// The files changed, by path, byte by byte; the files in conflict are not among them.
    pub rows: Vec<ChangeRow>,
    /// The files in conflict, apart.
    pub conflicts: Vec<ConflictedFile>,
    /// The box on the header of the list.
    pub header: StagedBox,
}

impl ChangesList {
    /// Whether at least one file is staged, all or some of it.
    #[must_use]
    pub fn has_staged(&self) -> bool {
        self.rows
            .iter()
            .any(|row| matches!(row.staged, StagedBox::Ticked | StagedBox::HalfTicked))
    }

    /// Whether at least one file has edits not saved.
    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.rows.iter().any(|row| row.is_unsaved)
    }
}

/// A file of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeRow {
    /// Its path from the top of the working tree, with `/`.
    pub path: String,
    /// Its name before a rename or a copy that is staged, shown beside it.
    pub original_path: Option<String>,
    /// How it is changed next to the last commit.
    pub status: FileStatus,
    /// How much of it is staged.
    pub staged: StagedBox,
    /// Whether it has edits typed in Gitardo's editors and not saved: a dot after its name.
    pub is_unsaved: bool,
}

/// How a file of the list is changed next to the last commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileStatus {
    /// Changed as git says.
    Changed(ChangeKind),
    /// Not tracked by git: `?`.
    Untracked,
    /// Changed only in an editor of Gitardo; git sees no change in it.
    Unsaved,
}

/// A box of the list: how much of a file, or of every file, is staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StagedBox {
    /// All of it is staged.
    Ticked,
    /// Some of it is staged.
    HalfTicked,
    /// None of it is staged.
    Empty,
    /// It cannot be ticked: a file only an editor has changed, or the header while git sees no file.
    #[default]
    Disabled,
}

/// What a path is in each list of git.
#[derive(Clone, Copy, Debug, Default)]
struct Sides<'a> {
    /// Its change in the index.
    staged: Option<&'a FileChange>,
    /// Its change in the working tree.
    unstaged: Option<&'a FileChange>,
    /// Whether git does not track it.
    is_untracked: bool,
}

/// Turns what git says into the one list of the Changes page: each file once, all, some or none of it staged; a file
/// in both lists shown as the working tree has it next to the last commit; a staged rename one row under its new name;
/// the files in conflict apart; the files only an editor has changed as rows whose box cannot be ticked; the rows by
/// path, byte by byte; and the box of the header from them.
///
/// Feature: changes.list
#[must_use]
pub fn list(input: &ChangesInput<'_>) -> ChangesList {
    let tree = input.working_tree;
    let mut sides: BTreeMap<&str, Sides<'_>> = BTreeMap::new();
    for change in &tree.staged {
        sides.entry(&change.path).or_default().staged = Some(change);
    }
    for change in &tree.unstaged {
        sides.entry(&change.path).or_default().unstaged = Some(change);
    }
    for path in &tree.untracked {
        sides.entry(path).or_default().is_untracked = true;
    }
    let mut rows: Vec<ChangeRow> = sides
        .into_iter()
        .map(|(path, sides)| ChangeRow {
            path: path.to_owned(),
            original_path: sides.staged.and_then(|change| change.original_path.clone()),
            status: status(sides),
            staged: staged(sides),
            is_unsaved: input.unsaved.iter().any(|unsaved| unsaved == path),
        })
        .collect();
    let header = header(&rows);
    for path in input.unsaved {
        let is_known =
            rows.iter().any(|row| row.path == *path) || tree.conflicted.iter().any(|file| file.path == *path);
        if !is_known {
            rows.push(ChangeRow {
                path: path.clone(),
                original_path: None,
                status: FileStatus::Unsaved,
                staged: StagedBox::Disabled,
                is_unsaved: true,
            });
        }
    }
    rows.sort_by(|first, second| first.path.as_bytes().cmp(second.path.as_bytes()));
    ChangesList {
        rows,
        conflicts: tree.conflicted.clone(),
        header,
    }
}

/// How much of a file is staged: all of it when only the index has it, none when only the working tree has it.
fn staged(sides: Sides<'_>) -> StagedBox {
    let is_changed_on_disk = sides.unstaged.is_some() || sides.is_untracked;
    match (sides.staged.is_some(), is_changed_on_disk) {
        (true, true) => StagedBox::HalfTicked,
        (true, false) => StagedBox::Ticked,
        (false, _) => StagedBox::Empty,
    }
}

/// How a file is changed next to the last commit. In both lists: staged as added, it stays added; staged as deleted
/// and still on disk, it is modified; staged as anything else and deleted from disk since, it is deleted.
fn status(sides: Sides<'_>) -> FileStatus {
    let on_disk = sides
        .unstaged
        .map(|change| FileStatus::Changed(change.kind))
        .or_else(|| sides.is_untracked.then_some(FileStatus::Untracked));
    match (sides.staged.map(|change| change.kind), on_disk) {
        (Some(ChangeKind::Added), _) => FileStatus::Changed(ChangeKind::Added),
        (Some(ChangeKind::Deleted), Some(_)) => FileStatus::Changed(ChangeKind::Modified),
        (Some(_), Some(FileStatus::Changed(ChangeKind::Deleted))) => FileStatus::Changed(ChangeKind::Deleted),
        (Some(kind), _) => FileStatus::Changed(kind),
        (None, Some(status)) => status,
        (None, None) => FileStatus::Untracked,
    }
}

/// The box of the header: ticked when every file git sees is all staged, half ticked when some of it is staged, empty
/// when none is, and disabled when git sees no file.
fn header(rows: &[ChangeRow]) -> StagedBox {
    if rows.is_empty() {
        return StagedBox::Disabled;
    }
    if rows.iter().all(|row| row.staged == StagedBox::Ticked) {
        StagedBox::Ticked
    } else if rows.iter().all(|row| row.staged == StagedBox::Empty) {
        StagedBox::Empty
    } else {
        StagedBox::HalfTicked
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
