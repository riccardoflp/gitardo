//! What new worktrees get from the checkout: the things git ignores there, and what the link and copy patterns take.

mod patterns;
mod settings;

pub use patterns::{
    CopyInput, LinkInput, PatternTake, SetupPreview, folders_holding_matches, folders_to_link, paths_to_copy,
};
pub use settings::{InEffect, Places, SettingSource, SettingsInput, SetupSettings, settings_in_effect};

/// How many files of a folder are counted before its size is said to be more.
pub const FOLDER_FILES_COUNTED: usize = 5_000;

/// Something git ignores in the checkout, as the page that chooses what new worktrees get lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredEntry {
    /// Its path from the top of the checkout, with `/`, without one at its end: `node_modules`, `config/.env`.
    pub path: String,
    /// What it is on disk, and its size.
    pub kind: IgnoredKind,
}

/// What an entry git ignores is on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoredKind {
    /// A file, of so many bytes.
    File {
        /// Its size.
        bytes: u64,
    },
    /// A folder git ignores whole.
    Folder(FolderSize),
    /// A link, which is not looked into and has no size.
    Link,
}

/// The size of a folder, counting its files without following links, at most [`FOLDER_FILES_COUNTED`] of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FolderSize {
    /// How many files were counted.
    pub files: usize,
    /// How many bytes they hold.
    pub bytes: u64,
    /// Whether it has more files than were counted: its size is more than this says.
    pub has_more: bool,
}
