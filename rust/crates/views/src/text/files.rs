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

/// What the page says when git could not list the files.
#[must_use]
pub fn failed(said: &str) -> String {
    format!("{FAILED} {said}")
}
