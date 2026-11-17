//! What the settings page shows of the settings read (`settings.edit`): each setting with the value in force, where it
//! comes from, and why a value written for it was not taken (`settings.read`).

use render::ThemeName;
use views::settings::{SettingRow, SettingSource};
use views::text::settings::{self as words, SettingInfo, settings};

use super::{ProblemReason, SettingProblem, Settings, SettingsFile, keys};

/// Every setting, in the order of the card, as the settings read say it.
#[must_use]
pub fn setting_rows(read: &Settings) -> Vec<SettingRow> {
    settings().map(|info| setting_row(read, info)).collect()
}

/// A setting: the first value taken of the files that may set it, the repository's first, or its default.
fn setting_row(settings: &Settings, info: &'static SettingInfo) -> SettingRow {
    let problems: Vec<&SettingProblem> = settings
        .problems
        .iter()
        .filter(|problem| problem.key == info.key)
        .collect();
    let taken = settings.values.get(info.key).and_then(|values| {
        values.iter().find(|(file, _)| {
            (info.is_repository || *file == SettingsFile::User) && !problems.iter().any(|problem| problem.file == *file)
        })
    });
    let (value, source) = match taken {
        Some((file, value)) => (json::to_compact_text(value), source(*file)),
        None => (info.default.to_owned(), SettingSource::Default),
    };
    SettingRow {
        info,
        value,
        source,
        problem: problems.first().map(|problem| problem_words(problem)),
    }
}

/// Where a value comes from, as the page says it.
fn source(file: SettingsFile) -> SettingSource {
    match file {
        SettingsFile::User => SettingSource::User,
        SettingsFile::Repository => SettingSource::Repository,
    }
}

/// Why a value was not taken, in the words of the page: `User settings: not a number. The default is used.`
#[must_use]
pub fn problem_words(problem: &SettingProblem) -> String {
    let file = match problem.file {
        SettingsFile::User => words::FROM_USER,
        SettingsFile::Repository => words::FROM_REPOSITORY,
    };
    let reason = match &problem.reason {
        ProblemReason::Unreadable(said) => format!("not JSON with comments ({said})"),
        ProblemReason::WrongType { expected } => format!("not {expected}"),
        ProblemReason::NotAChoice { choices } => format!("not one of {}", choices.join(", ")),
        ProblemReason::OutOfRange { least, most } => format!("not between {least} and {most}"),
        ProblemReason::UserOnly => String::from("only the user settings can set it"),
    };
    words::problem(file, &reason)
}

/// The value of `theme` in force: `system`.
#[must_use]
pub fn theme_value(theme: ThemeName) -> &'static str {
    let index = match theme {
        ThemeName::System => 0,
        ThemeName::Dark => 1,
        ThemeName::Light => 2,
        ThemeName::HighContrastDark => 3,
        ThemeName::HighContrastLight => 4,
    };
    keys::THEMES.get(index).copied().unwrap_or_default()
}
