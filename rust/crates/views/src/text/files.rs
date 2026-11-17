//! The words of the Files page: the tree of the working tree, where a file or a folder is picked for its history, its
//! page or a comparison.

/// The name of the page, in its trail.
pub const PAGE: &str = "Files";
/// The command of Quick search that opens the Files page.
pub const COMMAND: &str = "Show files";
/// The name of the tree, for the screen reader.
pub const TREE: &str = "Files of the working tree";
/// What the page says while the files are listed.
pub const READING: &str = "Listing the files\u{2026}";
/// What the page says of a working tree with no file.
pub const NO_FILES: &str = "This working tree has no files.";
/// What the page says when git could not list the files, before what it said.
pub const FAILED: &str = "Git could not list the files:";
/// **Open file** in the context menu of a file.
pub const OPEN_FILE: &str = "Open file";
/// **Show file history** in the context menu of a file (`file-history.file`).
pub const SHOW_FILE_HISTORY: &str = "Show file history";
/// **Show folder history** in the context menu of a folder (`file-history.folder`).
pub const SHOW_FOLDER_HISTORY: &str = "Show folder history";
/// **Compare with Branch, Tag or Commit…** in the context menu of a file or a folder (`compare.path-with`).
pub const COMPARE_WITH: &str = "Compare with Branch, Tag or Commit\u{2026}";

/// How a file differs from the last commit, said after its name to the screen reader (`file-view.files-changes`).
pub const ADDED: &str = "added";
/// A file modified.
pub const MODIFIED: &str = "modified";
/// A file deleted.
pub const DELETED: &str = "deleted";
/// A file renamed, before its old name.
pub const RENAMED_FROM: &str = "renamed from";
/// A file copied, before the name of the file it was copied from.
pub const COPIED_FROM: &str = "copied from";
/// A file renamed or copied whose old name is not known.
pub const RENAMED: &str = "renamed";
/// A file copied whose old name is not known.
pub const COPIED: &str = "copied";
/// A file that became a link, a submodule or a file, from another of these.
pub const TYPE_CHANGED: &str = "type changed";
/// A file git does not track.
pub const UNTRACKED: &str = "untracked";
/// A file in conflict.
pub const CONFLICTED: &str = "in conflict";
/// A folder holding a file that differs from the last commit.
pub const HAS_CHANGES: &str = "has changes";

/// A name with what is said of it to the screen reader: `app.rs, modified`.
#[must_use]
pub fn with_state(name: &str, state: &str) -> String {
    format!("{name}, {state}")
}

/// What is said of a file renamed or copied, with its old name: `renamed from src/a.rs`.
#[must_use]
pub fn from_path(said: &str, original: &str) -> String {
    format!("{said} {original}")
}

/// The tooltip of a file renamed or copied, with its old name: `Renamed from src/a.rs`.
#[must_use]
pub fn tooltip_from(is_copy: bool, original: &str) -> String {
    let said = if is_copy { "Copied from" } else { "Renamed from" };
    format!("{said} {original}")
}

/// What the page says when git could not list the files.
#[must_use]
pub fn failed(said: &str) -> String {
    format!("{FAILED} {said}")
}
