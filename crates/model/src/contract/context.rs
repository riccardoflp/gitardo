//! What a feature is given: the repository as last read, what it was invoked on, the settings and the time.

use std::sync::Arc;

use crate::state::RepositoryState;
use crate::{CommitId, Timestamp};

/// Everything a feature may know, as one immutable snapshot. A feature reads nothing else: no clock, no environment,
/// no disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// The repository, as last read.
    pub repository: Arc<RepositoryState>,
    /// What the feature was invoked on.
    pub target: Target,
    /// The settings the features read.
    pub settings: FeatureSettings,
    /// The time the feature was invoked.
    pub now: Timestamp,
}

/// What a feature was invoked on: the item of a menu, or what is checked out for the toolbar and the palette.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Target {
    /// What is checked out: the toolbar, the command palette.
    #[default]
    Head,
    /// A commit, by its hash.
    Commit(CommitId),
    /// A local branch, by its short name.
    Branch(String),
    /// A remote branch, by its short name: `origin/main`.
    RemoteBranch(String),
    /// A tag, by its short name.
    Tag(String),
    /// A stash, by its place in the list: 0 for `stash@{0}`.
    Stash(usize),
    /// A remote, by its name.
    Remote(String),
    /// Files of the working tree, from its top: the one a menu was opened on, or those ticked.
    Files(Vec<String>),
    /// A file of the Changes page, by its path from the top of the working tree.
    File(String),
    /// The list of the Changes page: the files changed for git, without the files in conflict, which are apart.
    Changes,
    /// A change of a Diff page, from a button of its gutter.
    DiffChange(Box<crate::diff::DiffChange>),
    /// Several commits: those selected in the timeline, in its order, newest first (`selection.select`), or the
    /// commit selected before and the one clicked (`compare.selected-before`).
    Commits(Vec<CommitId>),
    /// A comparison page, or a part of it: a side, or a file (`compare.page`).
    Comparison(Box<crate::compare::ComparisonTarget>),
    /// The file of a Merge page, with its result when it is not saved (`merge-view.mark-resolved`).
    MergeResult(Box<crate::merge_view::MergeResolution>),
    /// The resolutions accepted on the review of the conflicts (`review.write`).
    Resolutions(Box<crate::review::Resolutions>),
    /// The text of a File page, to save (`file-view.save`).
    FileSave(Box<crate::file_view::FileSave>),
    /// The texts of the File pages with edits not saved, to save at once (`changes.save`).
    FileSaves(Vec<crate::file_view::FileSave>),
    /// A file of the working tree at the line of the cursor of its page (`file-view.open-external`).
    FileAt(crate::file_view::FileAt),
    /// A submodule, by its folder from the top of the repository: `vendor/a/b` (`submodule.section`).
    Submodule(String),
    /// A worktree, by its folder.
    Worktree(std::path::PathBuf),
    /// What the page of new worktrees saves, or the place it removes them from or edits them in
    /// (`worktree-setup.save`, `worktree-setup.remove-from`, `worktree-setup.edit-settings-file`).
    SetupSettings(Box<crate::worktree_setup::SetupSave>),
    /// A file or a folder on disk, by its whole path: an entry of the Files page, or the file of a File page
    /// (`file-history.file`, `file-history.folder`, `compare.path-with`).
    Path(std::path::PathBuf),
    /// Some lines of the file of a File page (`file-history.lines`).
    Lines(Box<crate::file_history::LinesOfFile>),
    /// The file of a File page and the line of its cursor, for the operations of the blame.
    LineOfFile(Box<crate::blame::LineOfFile>),
}

/// The settings the features read, with the defaults of `docs/features/18-settings.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "each is a setting of its own, on or off")]
pub struct FeatureSettings {
    /// `commandPreview`: when the commands are shown before they run.
    pub command_preview: CommandPreview,
    /// `submodules.updateOnCheckout`: after a checkout, update the submodules to the commits it records.
    pub update_submodules_on_checkout: bool,
    /// `branches.format`: a format for the names of new branches; empty when there is none.
    pub branch_format: String,
    /// `branches.openIn`: what a double click on a branch does.
    pub branch_open_in: OpenIn,
    /// `pull.rebase`: **Pull** adds `--rebase`.
    pub pull_rebase: bool,
    /// `pull.autostash`: **Pull** adds `--autostash`.
    pub pull_autostash: bool,
    /// `identities`: the names and emails to choose from when switching identity, in their order.
    pub identities: Vec<IdentityProfile>,
    /// `cherryPick.recordOrigin`: cherry-picks end their message with `(cherry picked from commit …)`, as `-x` does.
    pub cherry_pick_record_origin: bool,
    /// `conflicts.autoResolve`: offer to resolve the conflicts that need no choice.
    pub conflicts_auto_resolve: bool,
    /// `conflicts.ai`: offer to ask the AI provider about the conflicts the rules leave.
    pub conflicts_ai: bool,
    /// `editor.external`: the command that opens a file in another editor, with `${file}` and `${line}`; empty for the
    /// program the system has for the file.
    pub external_editor: String,
    /// The user's settings file, which the operations that save settings write (`worktree-setup.save`); none when the
    /// system does not say where the user's folders are.
    pub user_settings_file: Option<std::path::PathBuf>,
    /// Whether the user trusts the folder of the window, so that the commands it asks to run are run
    /// (`worktree-setup.command-steps`).
    pub trust: crate::worktree_setup::Trust,
    /// The blame of the files and of the line under the cursor (`blame.enabled`, `blame.ignoreWhitespaceAndMoves`,
    /// `fileBlame.everyFile`), and the user's settings file, where their switches are written.
    pub blame: BlameSettings,
}

/// The settings of the blame the features switch, and the file they are written in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "each is a setting of its own, on or off")]
pub struct BlameSettings {
    /// `blame.enabled`: the line under the cursor says who last changed it.
    pub is_enabled: bool,
    /// `blame.ignoreWhitespaceAndMoves`: blame with `-w -M`.
    pub ignores_whitespace: bool,
    /// `fileBlame.everyFile`: every file opened shows its blame.
    pub is_on_every_file: bool,
    /// The user's settings file; none when the window has no place for it.
    pub user_file: Option<std::path::PathBuf>,
}

impl Default for BlameSettings {
    fn default() -> Self {
        BlameSettings {
            is_enabled: true,
            ignores_whitespace: false,
            is_on_every_file: false,
            user_file: None,
        }
    }
}

/// An identity of the setting `identities`: `{ "name", "email", "signingKey"? }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityProfile {
    /// `name`.
    pub name: String,
    /// `email`.
    pub email: String,
    /// `signingKey`, when it has one.
    pub signing_key: Option<String>,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        FeatureSettings {
            command_preview: CommandPreview::Off,
            update_submodules_on_checkout: true,
            branch_format: String::new(),
            branch_open_in: OpenIn::Checkout,
            pull_rebase: true,
            pull_autostash: true,
            identities: Vec::new(),
            cherry_pick_record_origin: false,
            conflicts_auto_resolve: false,
            conflicts_ai: false,
            external_editor: String::new(),
            user_settings_file: None,
            trust: crate::worktree_setup::Trust::NotTrusted,
            blame: BlameSettings::default(),
        }
    }
}

/// `branches.openIn`: what a double click on a local branch, or on a remote branch, does (`branch.open`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenIn {
    /// `checkout`: it is checked out.
    #[default]
    Checkout,
    /// `worktree`: it opens in a worktree of its own.
    Worktree,
}

/// `commandPreview`: when the engine shows a plan before it runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandPreview {
    /// `off`: only the plans that can lose work, or that ask to be confirmed.
    #[default]
    Off,
    /// `all`: every plan that changes something.
    All,
}
