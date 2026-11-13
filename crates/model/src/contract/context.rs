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
