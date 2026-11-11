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
}

/// The settings the features read, with the defaults of `docs/features/18-settings.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSettings {
    /// `commandPreview`: when the commands are shown before they run.
    pub command_preview: CommandPreview,
    /// `submodules.updateOnCheckout`: after a checkout, update the submodules to the commits it records.
    pub update_submodules_on_checkout: bool,
    /// `branches.format`: a format for the names of new branches; empty when there is none.
    pub branch_format: String,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        FeatureSettings {
            command_preview: CommandPreview::Off,
            update_submodules_on_checkout: true,
            branch_format: String::new(),
        }
    }
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
