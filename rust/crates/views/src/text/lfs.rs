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
