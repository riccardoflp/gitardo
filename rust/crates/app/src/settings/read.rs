//! Reading the settings of the window: the two files, the repository's first for the settings it may set, a value of
//! the wrong kind left for the default and said as a problem. What the operations read of them is read by the crate
//! `settings`, which the engine uses too; the theme, the fonts and the timeline are read here.

use std::path::Path;

use ::settings::Lookup;
use ::settings::keys::{self, Scope};
use engine::LogLevel;
use model::changes::OpenOnClick;
use render::ThemeName;
use views::graph::{Switches, TimelineSwitch};

use super::{FileBlameLook, FontChoice, ProblemReason, Settings, SettingsFiles, TerminalChoice};

/// The settings of the user's file and of the repository's, when a repository is shown. A file that is not there sets
/// nothing; one that cannot be read or parsed sets nothing and is said as a problem.
///
/// Feature: settings.read
#[must_use]
pub fn read(user_file: &Path, repository_file: Option<&Path>) -> Settings {
    read_files(&SettingsFiles {
        user: Some(user_file.to_path_buf()),
        repository: repository_file.map(Path::to_path_buf),
    })
}

/// The settings of the files of a window, the user's left out when the system does not say where it is.
#[must_use]
pub fn read_files(files: &SettingsFiles) -> Settings {
    settings_of(Lookup::read(files))
}

/// The settings of the texts of the two files, without reading anything.
#[must_use]
pub fn parse(user: Option<&str>, repository: Option<&str>) -> Settings {
    settings_of(Lookup::parse(user, repository))
}

/// The settings in force, taken from the two files in the order the settings page lists the problems.
fn settings_of(mut lookup: Lookup) -> Settings {
    let defaults = Settings::default();
    let theme = lookup
        .choice(keys::THEME, &keys::THEMES)
        .map_or(defaults.theme, theme_name);
    let fonts = FontChoice {
        interface: lookup
            .string(keys::FONT_INTERFACE, Scope::User)
            .unwrap_or(defaults.fonts.interface),
        code: lookup
            .string(keys::FONT_CODE, Scope::User)
            .unwrap_or(defaults.fonts.code),
        size: font_size(&mut lookup).unwrap_or(defaults.fonts.size),
    };
    let file_icons = file_icons(&mut lookup);
    let read_on_startup = lookup.boolean(keys::READ_ON_STARTUP, Scope::UserAndRepository);
    let keep_open = lookup.boolean(keys::KEEP_OPEN, Scope::User);
    let page_size = ::settings::page_size(&mut lookup);
    let switches = timeline_switches(&mut lookup);
    let sort_order = ::settings::sort_order(&mut lookup);
    let collapse_unchanged = lookup.boolean(keys::COLLAPSE_UNCHANGED, Scope::User);
    let open_on_click = match lookup.choice(keys::OPEN_ON_CLICK, &keys::OPEN_ON_CLICKS) {
        Some("diff") => OpenOnClick::Diff,
        _ => OpenOnClick::File,
    };
    let log_level = match lookup.choice(keys::LOG_LEVEL, &keys::LOG_LEVELS) {
        Some("debug") => LogLevel::Debug,
        _ => LogLevel::Info,
    };
    let features = ::settings::feature_settings(&mut lookup);
    let file_blame = file_blame_look(&mut lookup);
    let word_wrap = word_wrap(&mut lookup);
    let markdown_open_as_preview = lookup.boolean(keys::MARKDOWN_OPEN_AS_PREVIEW, Scope::User);
    let terminal = terminal_choice(&mut lookup);
    let values = lookup.written_values();
    Settings {
        log_level,
        features,
        file_blame,
        terminal,
        values,
        theme,
        fonts,
        file_icons,
        switches,
        sort_order,
        collapse_unchanged: collapse_unchanged.unwrap_or(defaults.collapse_unchanged),
        open_on_click,
        word_wrap,
        markdown_open_as_preview: markdown_open_as_preview.unwrap_or(defaults.markdown_open_as_preview),
        read_on_startup: read_on_startup.unwrap_or(defaults.read_on_startup),
        keep_open: keep_open.unwrap_or(defaults.keep_open),
        page_size,
        problems: lookup.into_problems(),
    }
}

/// `fileIcons`, from the user's file: how the icons of files and folders are drawn (`file-view.file-icons`).
fn file_icons(lookup: &mut Lookup) -> ui::FileIconStyle {
    match lookup.choice(keys::FILE_ICONS, &keys::FILE_ICON_STYLES) {
        Some("monochrome") => ui::FileIconStyle::Monochrome,
        Some("simple") => ui::FileIconStyle::Simple,
        _ => ui::FileIconStyle::Symbols,
    }
}

/// `editor.wordWrap`, from the user's file: whether the File page wraps a line longer than the page.
fn word_wrap(lookup: &mut Lookup) -> ui::LineWrap {
    match lookup.choice(keys::WORD_WRAP, &keys::WORD_WRAPS) {
        Some("on") => ui::LineWrap::Words,
        _ => ui::LineWrap::Cut,
    }
}

/// `fileBlame.format`, `fileBlame.groupLines` and `fileBlame.age.*`, from the user's file.
fn file_blame_look(lookup: &mut Lookup) -> FileBlameLook {
    let defaults = FileBlameLook::default();
    FileBlameLook {
        format: lookup
            .string(keys::FILE_BLAME_FORMAT, Scope::User)
            .unwrap_or(defaults.format),
        group_lines: lookup
            .boolean(keys::FILE_BLAME_GROUP_LINES, Scope::User)
            .unwrap_or(defaults.group_lines),
        age_enabled: lookup
            .boolean(keys::FILE_BLAME_AGE_ENABLED, Scope::User)
            .unwrap_or(defaults.age_enabled),
        recent_color: lookup
            .string(keys::FILE_BLAME_AGE_RECENT_COLOR, Scope::User)
            .unwrap_or(defaults.recent_color),
        old_color: lookup
            .string(keys::FILE_BLAME_AGE_OLD_COLOR, Scope::User)
            .unwrap_or(defaults.old_color),
        old_after_days: lookup
            .number(keys::FILE_BLAME_AGE_OLD_AFTER_DAYS, Scope::User)
            .unwrap_or(defaults.old_after_days),
    }
}

/// The theme of a value of `theme`, one of [`keys::THEMES`].
fn theme_name(value: &str) -> ThemeName {
    match value {
        "dark" => ThemeName::Dark,
        "light" => ThemeName::Light,
        "highContrastDark" => ThemeName::HighContrastDark,
        "highContrastLight" => ThemeName::HighContrastLight,
        _ => ThemeName::System,
    }
}

/// The switches of the timeline: each from its setting of the user's file, or its default.
fn timeline_switches(lookup: &mut Lookup) -> Switches {
    let mut switches = Switches::DEFAULT;
    let booleans = [
        (keys::SHOW_STASHES, TimelineSwitch::ShowStashes),
        (keys::FIRST_PARENT, TimelineSwitch::FirstParent),
        (keys::ONE_LANE, TimelineSwitch::OneLane),
        (keys::COMPACT_ROWS, TimelineSwitch::CompactRows),
        (keys::TRACE_SELECTED, TimelineSwitch::TraceSelected),
        (keys::DATE_SEPARATORS, TimelineSwitch::DateSeparators),
        (keys::COMBINE_UPSTREAM, TimelineSwitch::CombineUpstream),
    ];
    for (key, switch) in booleans {
        if let Some(is_on) = lookup.boolean(key, Scope::User) {
            switches = switches.with(switch, is_on);
        }
    }
    if let Some(columns) = columns(lookup) {
        let names = [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash];
        for (name, switch) in keys::COLUMN_NAMES.iter().zip(names) {
            switches = switches.with(switch, columns.iter().any(|column| column == name));
        }
    }
    switches
}

/// `graph.columns`: the names it lists among `author`, `date` and `hash`; none, which shows all three, when it is not a
/// list.
fn columns(lookup: &mut Lookup) -> Option<Vec<String>> {
    lookup.first((keys::COLUMNS, Scope::User), |value| {
        value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .ok_or(ProblemReason::WrongType { expected: "a list" })
    })
}

/// `terminal.shellPath` and `terminal.scrollbackLines`, from the user's file.
fn terminal_choice(lookup: &mut Lookup) -> TerminalChoice {
    let defaults = TerminalChoice::default();
    let most = keys::SCROLLBACK_MOST;
    let scrollback = lookup.first((keys::TERMINAL_SCROLLBACK, Scope::User), |value| {
        let lines = value
            .as_number()
            .and_then(json::Number::as_f64)
            .ok_or(ProblemReason::WrongType { expected: "a number" })?;
        if (0.0..=most).contains(&lines) {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "between 0 and a million"
            )]
            let lines = lines as usize;
            Ok(lines)
        } else {
            Err(ProblemReason::OutOfRange { least: 0.0, most })
        }
    });
    TerminalChoice {
        shell_path: lookup
            .string(keys::TERMINAL_SHELL_PATH, Scope::User)
            .unwrap_or(defaults.shell_path),
        scrollback: scrollback.unwrap_or(defaults.scrollback),
    }
}

/// `font.size`, a number within [`keys::FONT_SIZES`].
#[allow(
    clippy::cast_possible_truncation,
    reason = "the size is within 8 and 24 once checked, which an f32 holds exactly enough"
)]
fn font_size(lookup: &mut Lookup) -> Option<f32> {
    let (least, most) = keys::FONT_SIZES;
    lookup.first((keys::FONT_SIZE_KEY, Scope::User), |value| {
        let size = value
            .as_number()
            .and_then(json::Number::as_f64)
            .ok_or(ProblemReason::WrongType { expected: "a number" })?;
        if (least..=most).contains(&size) {
            Ok(size as f32)
        } else {
            Err(ProblemReason::OutOfRange { least, most })
        }
    })
}
