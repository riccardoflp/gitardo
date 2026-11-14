//! What new worktrees get from the checkout: the things git ignores there, and what the link and copy patterns take.

mod choose;
mod explain;
mod patterns;
mod propose;
mod settings;
mod steps;

pub use choose::{
    ChooseInput, ChosenLine, GetsInput, ProposedLine, SetupPatterns, choose, line_gets, name_pattern, own_path_pattern,
    path_patterns, use_proposed,
};
pub use explain::{ExplainInput, Ignored, SetupAction, SetupChange, Tracked, change_patterns, explain_change};
pub use patterns::{
    CopyInput, LinkInput, PatternTake, SetupPreview, folders_holding_matches, folders_to_link, paths_to_copy,
};
pub use propose::{
    CommandProposal, CommandsInput, Gets, LOCK_FILES, Proposal, ProposeInput, Section, propose, propose_commands,
};
pub use settings::{InEffect, Places, SettingSource, SettingsInput, SetupSettings, settings_in_effect};
pub use steps::{
    CommandStepsInput, LinkAndCopyInput, NoteStart, SetupSteps, Trust, command_steps, failed_setup_steps,
    link_and_copy_steps,
};

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

/// Where the settings of new worktrees are saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPlace {
    /// The repository's `.gitardo/settings.json`, for everyone who works on it.
    #[default]
    Repository,
    /// The user's settings file, for every repository.
    User,
}

/// What the page of new worktrees saves, and where.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupSave {
    /// Where.
    pub place: SettingsPlace,
    /// The link and copy patterns.
    pub patterns: SetupPatterns,
    /// The commands.
    pub commands: Vec<String>,
    /// Whether VS Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` get the same patterns, for the
    /// worktrees VS Code creates: only while saving in the repository.
    pub also_vs_code: bool,
}

/// What link and copy patterns take, asked of the checkout a worktree comes from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TakeRequest {
    /// The top folder of the checkout.
    pub folder: std::path::PathBuf,
    /// The patterns.
    pub patterns: SetupPatterns,
    /// The commit whose files the worktree has, by any name git understands; `HEAD` when none is given.
    pub commit: Option<String>,
}

/// What a path chosen from the Files page is for git.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathStatus {
    /// Whether it is a folder on disk.
    pub is_folder: bool,
    /// Whether git tracks it, or files in it.
    pub tracked: Tracked,
    /// How much of it git ignores.
    pub ignored: Ignored,
}
