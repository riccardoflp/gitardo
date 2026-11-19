//! The settings of Gitardo Desktop (`docs/features/18-settings.md`): read from the user's `settings.json` in Gitardo's
//! configuration folder and from `.gitardo/settings.json` of the repository shown, both JSON with comments.
//!
//! Only the settings this window uses are read here: the theme and the fonts (`settings.theme`), `fileIcons`,
//! `readOnStartup`,
//! `graph.pageSize`, the switches of the timeline, `branches.sortOrder`, `log.level`, and `commandPreview`,
//! `branches.format` and `submodules.updateOnCheckout`, which the operations read. `branches.followGraphFilter` is not
//! read since D51: the branch rail has a filter of its own.
//! Each card that uses another setting adds it to [`keys`]. Every value the files set is kept as written too, for the
//! settings page (`settings.edit`).
//!
//! The files, their keys and what the operations read of them are in the crate `settings`, which the engine uses
//! without the window; this module reads the window's own settings with it.

mod read;
mod rows;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use engine::LogLevel;
use model::changes::OpenOnClick;
use model::contract::FeatureSettings;
use model::graph::PageSize;
use model::refs::SortOrder;
use render::ThemeName;
use ui::{FileIconStyle, LineWrap};
use views::graph::Switches;

pub use ::settings::keys;
pub use ::settings::{
    ProblemReason, SettingProblem, SettingsFile, SettingsFiles, configuration_folder, home_folder,
    repository_settings_file, user_settings_file,
};
pub use read::{parse, read, read_files};
pub use rows::{problem_words, setting_rows, theme_value};

/// The settings in force: those of the repository's file for the settings it may set, then those of the user's file,
/// then the defaults.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools, reason = "each is a setting of its own, on or off")]
pub struct Settings {
    /// `theme`: the theme chosen, or to follow the system.
    pub theme: ThemeName,
    /// `font.ui`, `font.code`, `font.size`: the fonts of the interface and of code, and the size of the text.
    pub fonts: FontChoice,
    /// `fileIcons`: how the icons of files and folders are drawn (`file-view.file-icons`).
    pub file_icons: FileIconStyle,
    /// `readOnStartup`: read the repository when the window opens.
    pub read_on_startup: bool,
    /// `repositories.keepOpen`: each repository keeps what it showed while another is shown (`repo.keep-open`).
    pub keep_open: bool,
    /// `graph.pageSize`: how many commits the timeline reads at a time.
    pub page_size: PageSize,
    /// `graph.showStashes`, `graph.firstParent`, `graph.traceSelected`, `graph.columns`, `graph.dateSeparators` and
    /// `graph.combineUpstream`: the switches of the timeline.
    pub switches: Switches,
    /// `branches.sortOrder`: the order of the branches, the remote branches and the tags; none while it is not set,
    /// which is by name.
    pub sort_order: Option<SortOrder>,
    /// `log.level`: whether the Command log records the reads too.
    pub log_level: LogLevel,
    /// `commandPreview`, `branches.format` and `submodules.updateOnCheckout`: the settings the operations read.
    pub features: FeatureSettings,
    /// `diff.collapseUnchanged`: a new Diff page starts with its unchanged lines folded.
    pub collapse_unchanged: bool,
    /// `changes.openOnClick`: what a click on a file of the changes opens; its diff unless a file says `file` (D71).
    pub open_on_click: OpenOnClick,
    /// `editor.wordWrap`: whether the File page wraps a line longer than the page or cuts it at its right edge.
    pub word_wrap: LineWrap,
    /// `markdown.openAsPreview`: a Markdown file opens in its preview rather than its text.
    pub markdown_open_as_preview: bool,
    /// `commit.signOff`: **Add Signed-off-by** starts ticked in the options of each commit (`commit.options`).
    pub commit_sign_off: bool,
    /// `fileBlame.format`, `fileBlame.groupLines` and `fileBlame.age.*`: how the blame beside the lines of a file looks.
    pub file_blame: FileBlameLook,
    /// `terminal.shellPath` and `terminal.scrollbackLines`: the shell the terminals start, and what they keep.
    pub terminal: TerminalChoice,
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
            file_icons: FileIconStyle::Symbols,
            read_on_startup: true,
            keep_open: true,
            page_size: PageSize::DEFAULT,
            switches: Switches::DEFAULT,
            sort_order: None,
            log_level: LogLevel::Info,
            features: FeatureSettings::default(),
            collapse_unchanged: true,
            open_on_click: OpenOnClick::Diff,
            word_wrap: LineWrap::Cut,
            markdown_open_as_preview: false,
            commit_sign_off: false,
            file_blame: FileBlameLook::default(),
            terminal: TerminalChoice::default(),
            values: BTreeMap::new(),
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

/// The shell the terminals of the bottom panel start, and how much they keep (`terminal.view`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalChoice {
    /// `terminal.shellPath`: the program of the shell; empty for the system's own.
    pub shell_path: String,
    /// `terminal.scrollbackLines`: how many lines a terminal keeps above its screen.
    pub scrollback: usize,
}

impl Default for TerminalChoice {
    fn default() -> TerminalChoice {
        TerminalChoice {
            shell_path: String::new(),
            scrollback: keys::SCROLLBACK_LINES,
        }
    }
}

/// How the blame beside the lines of a file looks (`blame.file`).
#[derive(Clone, Debug, PartialEq)]
pub struct FileBlameLook {
    /// `fileBlame.format`: what is said beside each line.
    pub format: String,
    /// `fileBlame.groupLines`: a line of the commit of the line above says nothing.
    pub group_lines: bool,
    /// `fileBlame.age.enabled`: the bar of each line has the color of its age.
    pub age_enabled: bool,
    /// `fileBlame.age.recentColor`: the color of a line just changed.
    pub recent_color: String,
    /// `fileBlame.age.oldColor`: the color of a line changed long ago.
    pub old_color: String,
    /// `fileBlame.age.oldAfterDays`: after how many days a line has the old color.
    pub old_after_days: f64,
}

impl Default for FileBlameLook {
    fn default() -> FileBlameLook {
        FileBlameLook {
            format: String::from(model::blame::DEFAULT_FORMAT),
            group_lines: true,
            age_enabled: true,
            recent_color: String::from(model::blame::RECENT_COLOR),
            old_color: String::from(model::blame::OLD_COLOR),
            old_after_days: model::blame::OLD_AFTER_DAYS,
        }
    }
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
        TimelineSwitch::OneLane => keys::ONE_LANE,
        TimelineSwitch::CompactRows => keys::COMPACT_ROWS,
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
