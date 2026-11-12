//! The settings of Gitardo Desktop (`docs/features/18-settings.md`): read from the user's `settings.json` in Gitardo's
//! configuration folder and from `.gitardo/settings.json` of the repository shown, both JSON with comments.
//!
//! Only the settings this window uses are read here: the theme and the fonts (`settings.theme`), `readOnStartup` and
//! `graph.pageSize`. Each card that uses another setting adds it to [`keys`].

mod folder;
pub mod keys;
mod read;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use model::graph::PageSize;
use render::ThemeName;

pub use folder::{configuration_folder, repository_settings_file, user_settings_file};
pub use read::{parse, read, read_files};

/// The settings in force: those of the repository's file for the settings it may set, then those of the user's file,
/// then the defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// `theme`: the theme chosen, or to follow the system.
    pub theme: ThemeName,
    /// `font.ui`, `font.code`, `font.size`: the fonts of the interface and of code, and the size of the text.
    pub fonts: FontChoice,
    /// `readOnStartup`: read the repository when the window opens.
    pub read_on_startup: bool,
    /// `graph.pageSize`: how many commits the timeline reads at a time.
    pub page_size: PageSize,
    /// The values that were not taken, with which key and why, for the settings page to say.
    pub problems: Vec<SettingProblem>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            theme: ThemeName::System,
            fonts: FontChoice::default(),
            read_on_startup: true,
            page_size: PageSize::DEFAULT,
            problems: Vec::new(),
        }
    }
}

/// The fonts the window is drawn with.
#[derive(Clone, Debug, PartialEq)]
pub struct FontChoice {
    /// The family of the interface: `Inter`, which comes with the program, or the name of a font of the system.
    pub interface: String,
    /// The family of hashes and code: `JetBrains Mono`, which comes with the program, or a font of the system.
    pub code: String,
    /// The size of the text of the interface, in logical pixels; the other sizes follow it.
    pub size: f32,
}

impl Default for FontChoice {
    fn default() -> FontChoice {
        FontChoice {
            interface: String::from(keys::INTERFACE_FONT),
            code: String::from(keys::CODE_FONT),
            size: keys::FONT_SIZE,
        }
    }
}

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

/// The files the settings of a window are read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsFiles {
    /// The user's file; none when the system does not say where the user's folders are.
    pub user: Option<PathBuf>,
    /// The file of the repository shown, when one is shown.
    pub repository: Option<PathBuf>,
}
