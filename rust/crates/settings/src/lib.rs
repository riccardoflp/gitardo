//! The settings of Gitardo (`docs/features/18-settings.md`): where their files are, the user's `settings.json` in
//! Gitardo's configuration folder and `.gitardo/settings.json` of a repository, both JSON with comments; the keys of
//! every setting; and what the operations read of them, as [`FeatureSettings`](model::contract::FeatureSettings).
//!
//! The window of Gitardo Desktop reads its own settings (the theme, the fonts, the switches of the timeline) with the
//! same [`Lookup`]; the engine, run without the window, reads only those of the operations
//! (`docs/transition/02-protocol.md`, Settings).

#![forbid(unsafe_code)]

mod branch_inputs;
mod features;
mod folder;
pub mod keys;
mod lookup;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

pub use features::{feature_settings, page_size, sort_order};
pub use folder::{configuration_folder, home_folder, repository_settings_file, user_settings_file};
pub use lookup::Lookup;

/// Which file a value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SettingsFile {
    /// The user's `settings.json`.
    User,
    /// `.gitardo/settings.json` of the repository.
    Repository,
}

/// A value that was not taken.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingProblem {
    /// The key, as written in the file; empty when the whole file could not be read.
    pub key: String,
    /// The file it is in.
    pub file: SettingsFile,
    /// Why it was not taken.
    pub reason: ProblemReason,
}

/// Why a value was not taken.
#[derive(Clone, Debug, PartialEq)]
pub enum ProblemReason {
    /// The file is not JSON with comments: what the parser said.
    Unreadable(String),
    /// The value is not of the kind the setting takes, such as a string where a number goes.
    WrongType {
        /// What the setting takes: `a number`.
        expected: &'static str,
    },
    /// The value is not one of the choices of the setting.
    NotAChoice {
        /// The choices.
        choices: &'static [&'static str],
    },
    /// The number is outside what the setting takes.
    OutOfRange {
        /// The least it takes.
        least: f64,
        /// The most it takes.
        most: f64,
    },
    /// The setting is read only from the user's file, and the repository's set it.
    UserOnly,
}

/// The files the settings are read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsFiles {
    /// The user's file; none when the system does not say where the user's folders are.
    pub user: Option<PathBuf>,
    /// The file of the repository shown, when one is shown.
    pub repository: Option<PathBuf>,
}
