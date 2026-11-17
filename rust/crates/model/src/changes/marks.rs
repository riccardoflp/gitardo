//! `file-view.files-changes`: how each file of the tree of the Files page differs from the last commit, as the Changes
//! page says it, and the dot of each folder that holds such a file.

use std::collections::BTreeMap;

use super::{ChangesList, FileStatus};
use crate::state::ChangeKind;

/// How a file of the tree differs from the last commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMark {
    /// Changed as git says, staged and unstaged changes taken together.
    Changed(ChangeKind),
    /// Not tracked by git: `?`.
    Untracked,
    /// In conflict: `!`.
    Conflicted,
}

impl TreeMark {
    /// The dot this mark gives the folders above the file.
    #[must_use]
    pub fn folder_mark(self) -> FolderMark {
        match self {
            TreeMark::Conflicted => FolderMark::Conflicted,
            TreeMark::Changed(ChangeKind::Deleted) => FolderMark::Deleted,
            TreeMark::Changed(ChangeKind::Added) | TreeMark::Untracked => FolderMark::Added,
            TreeMark::Changed(
                ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Copied | ChangeKind::TypeChanged,
            ) => FolderMark::Modified,
        }
    }
}

/// The dot of a folder: the most important state of the files under it, at any depth. A later one wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FolderMark {
    /// A new file, added or untracked.
    Added,
    /// A file modified, renamed, copied or whose type changed.
    Modified,
    /// A file deleted.
    Deleted,
    /// A file in conflict.
    Conflicted,
}

/// A file of the tree that differs from the last commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkedFile {
    /// How it differs.
    pub mark: TreeMark,
    /// Its name before a rename or a copy that is staged, which its tooltip names.
    pub original_path: Option<String>,
}

/// The marks of the tree of the Files page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TreeMarks {
    /// The files that differ from the last commit, by their paths from the top of the working tree.
    pub files: BTreeMap<String, MarkedFile>,
    /// The dot of each folder holding one of them, by its path.
    pub folders: BTreeMap<String, FolderMark>,
}

impl TreeMarks {
    /// The files deleted, which are not on disk: the tree lists them too, at their places.
    pub fn deleted(&self) -> impl Iterator<Item = &str> {
        self.files
            .iter()
            .filter(|(_, file)| file.mark == TreeMark::Changed(ChangeKind::Deleted))
            .map(|(path, _)| path.as_str())
    }
}

/// The marks of the tree of the Files page, from the list of the Changes page: each file it lists marked as it lists
/// it, staged and unstaged changes taken together; a file in conflict marked so, whatever else it is; a file only an
/// editor has changed not marked, since git sees no change in it; and each folder above a marked file, at any depth,
/// with the dot of the most important of them.
///
/// Feature: file-view.files-changes
#[must_use]
pub fn tree_marks(list: &ChangesList) -> TreeMarks {
    let mut marks = TreeMarks::default();
    for row in &list.rows {
        let mark = match row.status {
            FileStatus::Changed(kind) => TreeMark::Changed(kind),
            FileStatus::Untracked => TreeMark::Untracked,
            FileStatus::Unsaved => continue,
        };
        let file = MarkedFile {
            mark,
            original_path: row.original_path.clone(),
        };
        let _previous = marks.files.insert(row.path.clone(), file);
    }
    for conflict in &list.conflicts {
        let file = MarkedFile {
            mark: TreeMark::Conflicted,
            original_path: None,
        };
        let _previous = marks.files.insert(conflict.path.clone(), file);
    }
    let mut folders: BTreeMap<String, FolderMark> = BTreeMap::new();
    for (path, file) in &marks.files {
        let dot = file.mark.folder_mark();
        for (end, _) in path.match_indices('/') {
            let folder = path.get(..end).unwrap_or_default().to_owned();
            let known = folders.entry(folder).or_insert(dot);
            *known = (*known).max(dot);
        }
    }
    marks.folders = folders;
    marks
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{FolderMark, TreeMark, tree_marks};
    use crate::changes::{ChangeRow, ChangesList, FileStatus, StagedBox};
    use crate::state::{ChangeKind, ConflictKind, ConflictedFile};

    fn row(path: &str, status: FileStatus) -> ChangeRow {
        ChangeRow {
            path: path.to_owned(),
            original_path: None,
            status,
            staged: StagedBox::Empty,
            is_unsaved: false,
        }
    }

    #[test]
    fn file_view_files_changes__a_folder_takes_the_most_important_state_under_it_at_any_depth() {
        let list = ChangesList {
            rows: vec![
                row("src/new.rs", FileStatus::Untracked),
                row("src/ui/list.rs", FileStatus::Changed(ChangeKind::Modified)),
                row("src/ui/gone.rs", FileStatus::Changed(ChangeKind::Deleted)),
                row("docs/guide.md", FileStatus::Changed(ChangeKind::Added)),
                row("notes.md", FileStatus::Unsaved),
            ],
            conflicts: vec![ConflictedFile {
                path: String::from("lib/merge.rs"),
                kind: ConflictKind::BothModified,
            }],
            header: StagedBox::Empty,
        };
        let marks = tree_marks(&list);
        assert_eq!(marks.folders.get("src"), Some(&FolderMark::Deleted));
        assert_eq!(marks.folders.get("src/ui"), Some(&FolderMark::Deleted));
        assert_eq!(marks.folders.get("docs"), Some(&FolderMark::Added));
        assert_eq!(marks.folders.get("lib"), Some(&FolderMark::Conflicted));
        assert_eq!(
            marks.files.get("lib/merge.rs").map(|file| file.mark),
            Some(TreeMark::Conflicted)
        );
        assert!(
            !marks.files.contains_key("notes.md"),
            "a file only an editor changed is not marked"
        );
        assert_eq!(marks.deleted().collect::<Vec<_>>(), ["src/ui/gone.rs"]);
    }
}
