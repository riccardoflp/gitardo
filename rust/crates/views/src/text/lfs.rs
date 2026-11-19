//! The words of the files stored with Git LFS (`lfs.show-in-pages`, `lfs.download`, `lfs.page`, `lfs.set-up`,
//! `lfs.track`, `lfs.store-committed`, `lfs.untrack`, `lfs.read-locks`, `lfs.lock`, `lfs.unlock`).

/// What the File page says of a file left as a pointer.
pub const NOT_DOWNLOADED_HEADING: &str = "Stored with Git LFS, not downloaded";
/// What the Diff page says of a file with a side not downloaded, and beside its title when both sides are there.
pub const STORED: &str = "Stored with Git LFS";
/// The button that downloads a file.
pub const DOWNLOAD: &str = "Download";
/// The tooltip of **Download**.
pub const DOWNLOAD_TOOLTIP: &str = "Download its content with Git LFS";
/// A side whose content is in the store.
pub const DOWNLOADED: &str = "Downloaded";
/// A side whose content is not in the store.
pub const NOT_DOWNLOADED: &str = "Not downloaded";
/// The side before, in the Diff page.
pub const BEFORE: &str = "Before";
/// The side after, in the Diff page.
pub const AFTER: &str = "After";
/// The button of the page of a commit that opens the diff of the contents.
pub const OPEN_IN_DIFF_PAGE: &str = "Open it in the Diff page";
/// How many digits of the id of a content are shown.
pub const SHORT_OID: usize = 10;

/// A side as the message says it: its size, the first digits of its id, and whether it is downloaded.
#[must_use]
pub fn side(size: u64, oid: &str, is_downloaded: bool) -> String {
    let short = oid.get(..SHORT_OID).unwrap_or(oid);
    let state = if is_downloaded { DOWNLOADED } else { NOT_DOWNLOADED };
    format!("{} \u{b7} {short} \u{b7} {state}", super::worktree_setup::bytes(size))
}

/// What the page of a commit says in place of the lines of two pointers: **Stored with Git LFS: 2.0 MB → 2.1 MB**,
/// a side where the file is not said **nothing**.
#[must_use]
pub fn pointer_change(before: Option<u64>, after: Option<u64>) -> String {
    let size = |size: Option<u64>| size.map_or_else(|| String::from("nothing"), super::worktree_setup::bytes);
    format!("{STORED}: {} \u{2192} {}", size(before), size(after))
}

/// The Large files page in its trail.
pub const PAGE: &str = "Large files";
/// The entry of the menu of the row of a repository that opens the page.
pub const MENU_ENTRY: &str = "Large files\u{2026}";
/// The command of Quick search that opens the page.
pub const COMMAND: &str = "Large Files (Git LFS)\u{2026}";
/// The head of the page without Git LFS.
pub const NOT_INSTALLED: &str = "Git LFS is not installed";
/// The button that opens the site of Git LFS.
pub const OPEN_SITE: &str = "Open git-lfs.com";
/// The site of Git LFS.
pub const SITE: &str = "https://git-lfs.com";
/// What the page says of a repository with patterns that is not set up.
pub const NOT_SET_UP: &str = "Git LFS is not set up in this repository: its files stay pointers.";
/// The button that sets it up.
pub const SET_UP: &str = "Set up";
/// The title of the patterns.
pub const PATTERNS: &str = "Patterns";
/// The button that stores a pattern with Git LFS.
pub const TRACK: &str = "Track a pattern\u{2026}";
/// The entry of the menu of a pattern that takes it out of the `.gitattributes` at the top.
pub const STOP_STORING: &str = "Stop storing with Git LFS";
/// The entry of the menu of a pattern that stores the files committed before it.
pub const STORE_COMMITTED: &str = "Store them with Git LFS in the next commit";
/// The title of the files.
pub const FILES: &str = "Files";
/// The button that reads the files again.
pub const REFRESH: &str = "Refresh";
/// The entry of the menu of a file that opens its File page.
pub const OPEN_FILE: &str = "Open file";
/// The entry of the menu of a file that shows it beside the tree of the Files page.
pub const SHOW_IN_FILES: &str = "Show in the Files page";
/// The entry of the Files page and of the changes that stores a file with Git LFS.
pub const STORE_WITH_LFS: &str = "Store with Git LFS\u{2026}";
/// What the page says while it reads the files.
pub const READING: &str = "Reading the files stored with Git LFS\u{2026}";
/// What the page says of a repository that stores nothing with Git LFS.
pub const NOTHING_STORED: &str = "This repository stores no file with Git LFS.";

/// The head of the page: **Git LFS 3.7.1**.
#[must_use]
pub fn version(version: &str) -> String {
    format!("Git LFS {version}")
}

/// The entry of the menu of a pattern that opens its file of attributes: **Open assets/.gitattributes**.
#[must_use]
pub fn open_attributes(file: &str) -> String {
    format!("Open {file}")
}

/// The count of the files and their size: **2 files, 4.1 MB**.
#[must_use]
pub fn files(count: usize, size: u64) -> String {
    let files = if count == 1 { "file" } else { "files" };
    format!("{count} {files}, {}", super::worktree_setup::bytes(size))
}

/// **Download all** with the count and the size of the files not downloaded: **Download all (1, 2.0 MB)**.
#[must_use]
pub fn download_all(count: usize, size: u64) -> String {
    if count == 0 {
        return String::from("Download all");
    }
    format!("Download all ({count}, {})", super::worktree_setup::bytes(size))
}
