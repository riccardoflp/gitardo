//! The words of the settings page (`settings.edit`) and of the picker of the themes (`settings.theme`): every setting of
//! `docs/features/18-settings.md`, with its description, its default and the files it may be set in, as the card lists
//! them.

mod table;
mod table_end;

/// What a setting holds, which says how the settings page changes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// `true` or `false`, changed with a switch.
    Toggle,
    /// One of some words, changed by choosing one.
    Choice(&'static [&'static str]),
    /// A whole number between two, typed in a field that refuses one out of its range.
    Number {
        /// The least it takes.
        least: i64,
        /// The most it takes.
        most: i64,
    },
    /// A string, typed in a field that says what it can hold.
    Text {
        /// What the empty field says: the `${…}` the string can hold.
        placeholder: &'static str,
    },
    /// A color, as `#e5534b`.
    Color,
    /// The name of a font.
    Font,
    /// A list of strings, a row for each.
    Strings,
    /// `identities`: a table of names, emails and signing keys.
    Identities,
    /// A list of objects, typed as JSON.
    Json,
}

/// The sections of the settings page, in their order (`settings.sections`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SettingsSection {
    /// **General**.
    General,
    /// **Appearance**.
    Appearance,
    /// **Timeline**.
    Timeline,
    /// **Branches**.
    Branches,
    /// **Pull and commits**.
    PullAndCommits,
    /// **Worktrees**.
    Worktrees,
    /// **Files and blame**.
    FilesAndBlame,
    /// **AI and conflicts**.
    AiAndConflicts,
    /// **Terminal and tools**.
    TerminalAndTools,
}

impl SettingsSection {
    /// Every section, in the order of the page.
    pub const ALL: [SettingsSection; 9] = [
        SettingsSection::General,
        SettingsSection::Appearance,
        SettingsSection::Timeline,
        SettingsSection::Branches,
        SettingsSection::PullAndCommits,
        SettingsSection::Worktrees,
        SettingsSection::FilesAndBlame,
        SettingsSection::AiAndConflicts,
        SettingsSection::TerminalAndTools,
    ];

    /// Its name, on its button and over its settings.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Timeline => "Timeline",
            SettingsSection::Branches => "Branches",
            SettingsSection::PullAndCommits => "Pull and commits",
            SettingsSection::Worktrees => "Worktrees",
            SettingsSection::FilesAndBlame => "Files and blame",
            SettingsSection::AiAndConflicts => "AI and conflicts",
            SettingsSection::TerminalAndTools => "Terminal and tools",
        }
    }
}

/// A setting, as the settings page lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingInfo {
    /// Its key: `graph.pageSize`.
    pub key: &'static str,
    /// What it is, in sentence case, the first line of its row: **Commits read at a time**.
    pub title: &'static str,
    /// The section it is in; none for a setting no longer read, which the page does not list.
    pub section: Option<SettingsSection>,
    /// What it does.
    pub description: &'static str,
    /// Its default, as written in a settings file.
    pub default: &'static str,
    /// Whether the repository's file may set it too.
    pub is_repository: bool,
    /// Whether it is experimental.
    pub is_experimental: bool,
    /// What it holds.
    pub kind: SettingKind,
}

/// Every setting, in the order of the card.
pub fn settings() -> impl Iterator<Item = &'static SettingInfo> {
    table::FIRST.iter().chain(table_end::REST.iter())
}

/// The title of the settings page, in its trail.
pub const PAGE: &str = "Settings";
/// Said beside the name of an experimental setting.
pub const EXPERIMENTAL: &str = "Experimental";
/// Where a value comes from: the default.
pub const FROM_DEFAULT: &str = "Default";
/// Where a value comes from: the user's file.
pub const FROM_USER: &str = "User settings";
/// Where a value comes from: the repository's file.
pub const FROM_REPOSITORY: &str = "Repository settings";
/// The label of the choice of the file the page writes in.
pub const WRITE_IN: &str = "Write changes in";
/// The button that opens the settings file in a File page.
pub const EDIT_IN_FILE: &str = "Edit in settings.json";
/// The tooltip of the control of a setting the repository's file may not set, while changes go to that file.
pub const USER_ONLY: &str = "Only the user settings can set this";
/// The picker of the themes (`settings.theme`).
pub const CHOOSE_THEME: &str = "Choose the theme";
/// **Theme…** in the **…** menu.
pub const THEME_ENTRY: &str = "Theme\u{2026}";
/// **Settings** in the **…** menu.
pub const SETTINGS_ENTRY: &str = "Settings";
/// The themes, in the order of the values of `theme`, as the picker names them.
pub const THEME_NAMES: [&str; 5] = [
    "Follow the system",
    "Dark",
    "Light",
    "High contrast dark",
    "High contrast light",
];
/// The values of `theme`, in the order of [`THEME_NAMES`].
pub const THEME_VALUES: [&str; 5] = ["system", "dark", "light", "highContrastDark", "highContrastLight"];
/// What the picker says of the theme in force.
pub const CURRENT_THEME: &str = "current";

/// Where a value comes from, said under it.
#[must_use]
pub fn source(file: &str) -> String {
    format!("From {}", file.to_lowercase())
}

/// A value not taken from a settings file, said under its setting: the file, then why.
#[must_use]
pub fn problem(file: &str, reason: &str) -> String {
    format!("{file}: {reason}. The default is used.")
}
