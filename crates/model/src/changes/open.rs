//! Which page, and which two versions, a click or a button on a file of the Changes page opens.

use super::{ChangeRow, FileStatus, StagedBox};
use crate::state::ChangeKind;

/// What was done on a file of the Changes page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAction {
    /// **Open changes**.
    OpenChanges,
    /// **Open file**.
    OpenFile,
    /// A click on its row.
    Click,
    /// **Compare with the saved file**, on a file only an editor has changed.
    CompareWithSaved,
}

/// `changes.openOnClick`: what a click on a row does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpenOnClick {
    /// What **Open file** does.
    #[default]
    File,
    /// What **Open changes** does.
    Diff,
}

/// A version of a file a side of a Diff page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenVersion {
    /// The last commit.
    Head,
    /// The index.
    Index,
    /// The file on disk.
    WorkingTree,
    /// What an editor holds and has not saved, editable.
    Buffer,
    /// Nothing: the file is added on the other side, or deleted.
    Nothing,
}

/// The page a file opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRequest {
    /// A Diff page of the file between two versions.
    Diff {
        /// The version on the left.
        left: OpenVersion,
        /// The version on the right.
        right: OpenVersion,
        /// Its path on the left: its old name, for a rename all staged.
        left_path: String,
        /// Its path on the right.
        right_path: String,
    },
    /// The File page, with the file as it is on disk.
    File(String),
    /// The Merge page of a file in conflict.
    Merge(String),
}

/// The file something was done on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenTarget<'a> {
    /// A file of **Changes**.
    Changed(&'a ChangeRow),
    /// A file of **Conflicts**, by its path.
    Conflict(&'a str),
}

/// What [`open`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenInput<'a> {
    /// The file.
    pub file: OpenTarget<'a>,
    /// What was done on it.
    pub action: OpenAction,
    /// `changes.openOnClick`.
    pub on_click: OpenOnClick,
}

/// The page a click or a button on a file of the Changes page opens.
///
/// Feature: changes.open
#[must_use]
pub fn open(input: &OpenInput<'_>) -> PageRequest {
    let row = match input.file {
        OpenTarget::Conflict(path) => return PageRequest::Merge(path.to_owned()),
        OpenTarget::Changed(row) => row,
    };
    let is_deleted = row.status == FileStatus::Changed(ChangeKind::Deleted);
    let wants_diff = match input.action {
        OpenAction::OpenChanges => true,
        OpenAction::OpenFile => is_deleted,
        OpenAction::Click => is_deleted || input.on_click == OpenOnClick::Diff,
        OpenAction::CompareWithSaved => {
            return PageRequest::Diff {
                left: OpenVersion::WorkingTree,
                right: OpenVersion::Buffer,
                left_path: row.path.clone(),
                right_path: row.path.clone(),
            };
        }
    };
    if wants_diff && row.status != FileStatus::Unsaved {
        changes_diff(row, is_deleted)
    } else {
        PageRequest::File(row.path.clone())
    }
}

/// What **Open changes** shows of a file: the index against the file on disk while some of it is not staged, the last
/// commit against the index once all of it is.
fn changes_diff(row: &ChangeRow, is_deleted: bool) -> PageRequest {
    let is_new = matches!(
        row.status,
        FileStatus::Untracked | FileStatus::Changed(ChangeKind::Added)
    );
    let right = if is_deleted {
        OpenVersion::Nothing
    } else if row.staged == StagedBox::Ticked {
        OpenVersion::Index
    } else {
        OpenVersion::WorkingTree
    };
    if row.staged == StagedBox::Ticked {
        let left = if is_new {
            OpenVersion::Nothing
        } else {
            OpenVersion::Head
        };
        return PageRequest::Diff {
            left,
            right,
            left_path: row.original_path.clone().unwrap_or_else(|| row.path.clone()),
            right_path: row.path.clone(),
        };
    }
    let left = if is_new && row.staged == StagedBox::Empty {
        OpenVersion::Nothing
    } else {
        OpenVersion::Index
    };
    PageRequest::Diff {
        left,
        right,
        left_path: row.path.clone(),
        right_path: row.path.clone(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{OpenAction, OpenInput, OpenOnClick, OpenTarget, OpenVersion, PageRequest, open};
    use crate::changes::{ChangeRow, FileStatus, StagedBox};
    use crate::state::ChangeKind;

    fn row(status: FileStatus, staged: StagedBox) -> ChangeRow {
        ChangeRow {
            path: String::from("src/app.rs"),
            original_path: None,
            status,
            staged,
            is_unsaved: false,
        }
    }

    fn opened(row: &ChangeRow, action: OpenAction, on_click: OpenOnClick) -> PageRequest {
        open(&OpenInput {
            file: OpenTarget::Changed(row),
            action,
            on_click,
        })
    }

    fn sides(request: &PageRequest) -> Option<(OpenVersion, OpenVersion)> {
        match request {
            PageRequest::Diff { left, right, .. } => Some((*left, *right)),
            PageRequest::File(_) | PageRequest::Merge(_) => None,
        }
    }

    const MODIFIED: FileStatus = FileStatus::Changed(ChangeKind::Modified);

    #[test]
    fn changes_open__a_file_partly_staged_opens_the_index_against_the_file_on_disk() {
        let partly = row(MODIFIED, StagedBox::HalfTicked);
        let request = opened(&partly, OpenAction::OpenChanges, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::Index, OpenVersion::WorkingTree)));
        let not_staged = row(MODIFIED, StagedBox::Empty);
        let request = opened(&not_staged, OpenAction::OpenChanges, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::Index, OpenVersion::WorkingTree)));
    }

    #[test]
    fn changes_open__a_file_all_staged_opens_the_last_commit_against_the_index() {
        let mut renamed = row(FileStatus::Changed(ChangeKind::Renamed), StagedBox::Ticked);
        renamed.original_path = Some(String::from("src/old.rs"));
        assert_eq!(
            opened(&renamed, OpenAction::OpenChanges, OpenOnClick::File),
            PageRequest::Diff {
                left: OpenVersion::Head,
                right: OpenVersion::Index,
                left_path: String::from("src/old.rs"),
                right_path: String::from("src/app.rs"),
            }
        );
    }

    #[test]
    fn changes_open__a_click_follows_the_setting() {
        let changed = row(MODIFIED, StagedBox::Empty);
        assert_eq!(
            opened(&changed, OpenAction::Click, OpenOnClick::Diff),
            opened(&changed, OpenAction::OpenChanges, OpenOnClick::File)
        );
        assert_eq!(
            opened(&changed, OpenAction::Click, OpenOnClick::File),
            PageRequest::File(String::from("src/app.rs"))
        );
    }

    #[test]
    fn changes_open__a_click_on_a_deleted_file_opens_its_diff_page_whatever_the_setting() {
        let deleted = row(FileStatus::Changed(ChangeKind::Deleted), StagedBox::Empty);
        for on_click in [OpenOnClick::File, OpenOnClick::Diff] {
            let request = opened(&deleted, OpenAction::Click, on_click);
            assert_eq!(sides(&request), Some((OpenVersion::Index, OpenVersion::Nothing)));
        }
        let removed = row(FileStatus::Changed(ChangeKind::Deleted), StagedBox::Ticked);
        let request = opened(&removed, OpenAction::OpenFile, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::Head, OpenVersion::Nothing)));
    }

    #[test]
    fn a_new_file_has_nothing_on_its_left_and_a_conflict_opens_its_merge_page() {
        let untracked = row(FileStatus::Untracked, StagedBox::Empty);
        let request = opened(&untracked, OpenAction::OpenChanges, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::Nothing, OpenVersion::WorkingTree)));
        let added = row(FileStatus::Changed(ChangeKind::Added), StagedBox::Ticked);
        let request = opened(&added, OpenAction::OpenChanges, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::Nothing, OpenVersion::Index)));
        let conflict = open(&OpenInput {
            file: OpenTarget::Conflict("src/app.rs"),
            action: OpenAction::Click,
            on_click: OpenOnClick::Diff,
        });
        assert_eq!(conflict, PageRequest::Merge(String::from("src/app.rs")));
        let unsaved = row(FileStatus::Unsaved, StagedBox::Disabled);
        let request = opened(&unsaved, OpenAction::CompareWithSaved, OpenOnClick::File);
        assert_eq!(sides(&request), Some((OpenVersion::WorkingTree, OpenVersion::Buffer)));
    }
}
