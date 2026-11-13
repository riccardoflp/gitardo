//! The settings of Gitardo Desktop (`docs/features/18-settings.md`): read from the user's `settings.json` in Gitardo's
//! configuration folder and from `.gitardo/settings.json` of the repository shown, both JSON with comments.
//!
//! Only the settings this window uses are read here: the theme and the fonts (`settings.theme`), `readOnStartup`,
//! `graph.pageSize`, the switches of the timeline, `branches.sortOrder`, `branches.followGraphFilter`, `log.level`, and
//! `commandPreview`, `branches.format` and `submodules.updateOnCheckout`, which the operations read.
//! Each card that uses another setting adds it to [`keys`]. Every value the files set is kept as written too, for the
//! settings page (`settings.edit`).

mod folder;
pub mod keys;
mod read;
mod rows;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;

use engine::LogLevel;
use model::changes::OpenOnClick;
use model::contract::FeatureSettings;
use model::graph::PageSize;
use model::refs::SortOrder;
use render::ThemeName;
use views::graph::Switches;

pub use folder::{configuration_folder, repository_settings_file, user_settings_file};
pub use read::{parse, read, read_files};
pub use rows::{problem_words, setting_rows, theme_value};

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
    /// `graph.showStashes`, `graph.firstParent`, `graph.traceSelected`, `graph.columns`, `graph.dateSeparators` and
    /// `graph.combineUpstream`: the switches of the timeline.
    pub switches: Switches,
    /// `branches.sortOrder`: the order of the branches, the remote branches and the tags; none while it is not set,
    /// which is by name.
    pub sort_order: Option<SortOrder>,
    /// `branches.followGraphFilter`: the branch rail lists only the branches the timeline shows.
    pub follows_graph_filter: bool,
    /// `log.level`: whether the Command log records the reads too.
    pub log_level: LogLevel,
    /// `commandPreview`, `branches.format` and `submodules.updateOnCheckout`: the settings the operations read.
    pub features: FeatureSettings,
    /// `diff.collapseUnchanged`: a new Diff page starts with its unchanged lines folded.
    pub collapse_unchanged: bool,
    /// `changes.openOnClick`: what a click on a file of the Changes page opens.
    pub open_on_click: OpenOnClick,
    /// `conflicts.autoResolve` and `conflicts.ai`: who proposes resolutions in the review of the conflicts.
    pub conflicts: ConflictSettings,
    /// Every value the files set, by key, the repository's file first: what the settings page shows.
    pub values: BTreeMap<String, Vec<(SettingsFile, json::Value)>>,
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
            switches: Switches::DEFAULT,
            sort_order: None,
            follows_graph_filter: true,
            log_level: LogLevel::Info,
            features: FeatureSettings::default(),
            collapse_unchanged: true,
            open_on_click: OpenOnClick::File,
            conflicts: ConflictSettings::default(),
            values: BTreeMap::new(),
            problems: Vec::new(),
        }
    }
}

/// Who proposes resolutions in the review of the conflicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ConflictSettings {
    /// `conflicts.autoResolve`: the rules propose.
    pub auto_resolve: bool,
    /// `conflicts.ai`: the AI provider may be asked about the conflicts the rules leave.
    pub ai: bool,
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

/// The value `branches.sortOrder` is written as.
#[must_use]
pub fn sort_order_value(order: SortOrder) -> json::Value {
    let [name, date] = keys::SORT_ORDERS;
    json::Value::String(String::from(match order {
        SortOrder::Name => name,
        SortOrder::Date => date,
    }))
}

/// The key a switch of the timeline is written under, and the value it is written as, in the user's settings file:
/// a column writes the whole list of `graph.columns`.
#[must_use]
pub fn switch_setting(switches: Switches, switch: views::graph::TimelineSwitch) -> (&'static str, json::Value) {
    use views::graph::TimelineSwitch;
    let key = match switch {
        TimelineSwitch::ShowStashes => keys::SHOW_STASHES,
        TimelineSwitch::FirstParent => keys::FIRST_PARENT,
        TimelineSwitch::TraceSelected => keys::TRACE_SELECTED,
        TimelineSwitch::DateSeparators => keys::DATE_SEPARATORS,
        TimelineSwitch::CombineUpstream => keys::COMBINE_UPSTREAM,
        TimelineSwitch::Author | TimelineSwitch::Date | TimelineSwitch::Hash => {
            let columns = [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash]
                .into_iter()
                .zip(keys::COLUMN_NAMES)
                .filter(|(column, _)| switches.is_on(*column))
                .map(|(_, name)| json::Value::String(name.to_owned()))
                .collect();
            return (keys::COLUMNS, json::Value::Array(columns));
        }
    };
    (key, json::Value::Bool(switches.is_on(switch)))
}
