//! What the settings page shows of the settings read (`settings.edit`): each setting with the value the file of the tab
//! shown sets, or the value in force, where the value in force comes from, and why a value written for it was not
//! taken (`settings.read`).

use render::ThemeName;
use views::settings::{SettingRow, SettingSource, SettingsTarget};
use views::text::settings::{self as words, SettingInfo, SettingKind, settings};

use super::{ProblemReason, SettingProblem, Settings, SettingsFile, keys};

/// Every setting, in the order of the card, as the settings read say it, for the tab of a file.
#[must_use]
pub fn setting_rows(read: &Settings, target: SettingsTarget) -> Vec<SettingRow> {
    settings().map(|info| setting_row(read, info, target)).collect()
}

/// A setting: the first value taken of the files that may set it, the repository's first, or its default; and the
/// value of the file of the tab, which its control shows.
fn setting_row(read: &Settings, info: &'static SettingInfo, target: SettingsTarget) -> SettingRow {
    let problems: Vec<&SettingProblem> = read.problems.iter().filter(|problem| problem.key == info.key).collect();
    let is_taken = |file: SettingsFile| {
        (info.is_repository || file == SettingsFile::User) && !problems.iter().any(|problem| problem.file == file)
    };
    let written = read.values.get(info.key);
    let of_file = |file: SettingsFile| {
        written
            .and_then(|values| values.iter().find(|(written_in, _)| *written_in == file))
            .map(|(_, value)| value)
    };
    let taken = written.and_then(|values| values.iter().find(|(file, _)| is_taken(*file)));
    let (in_force, source) = match taken {
        Some((file, value)) => (Some(value), source(*file)),
        None => (None, SettingSource::Default),
    };
    let file = match target {
        SettingsTarget::User => SettingsFile::User,
        SettingsTarget::Repository => SettingsFile::Repository,
    };
    let shown = of_file(file).filter(|_| is_taken(file)).or(in_force);
    let value = shown.map_or_else(|| info.default.to_owned(), json::to_compact_text);
    let parsed = shown.cloned().or_else(|| json::parse(info.default).ok());
    let set_by_repository =
        (target == SettingsTarget::User && info.is_repository && is_taken(SettingsFile::Repository))
            .then(|| of_file(SettingsFile::Repository).map(json::to_compact_text))
            .flatten();
    SettingRow {
        info,
        text: match &parsed {
            Some(parsed) => parsed.as_str().map_or_else(|| value.clone(), str::to_owned),
            None => String::new(),
        },
        entries: parsed.as_ref().map(|value| entries(info, value)).unwrap_or_default(),
        value,
        source,
        problem: problems.first().map(|problem| problem_words(problem)),
        is_set_here: of_file(file).is_some(),
        set_by_repository,
    }
}

/// The entries of a list of strings, each in a cell, or of `identities`, each its name, its email and its signing key.
fn entries(info: &SettingInfo, value: &json::Value) -> Vec<Vec<String>> {
    let items = value.as_array().unwrap_or_default();
    let text = |item: &json::Value, name: &str| {
        item.get(name)
            .and_then(json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    match info.kind {
        SettingKind::Strings => items
            .iter()
            .map(|item| vec![item.as_str().unwrap_or_default().to_owned()])
            .collect(),
        SettingKind::Identities => items
            .iter()
            .map(|item| vec![text(item, "name"), text(item, "email"), text(item, "signingKey")])
            .collect(),
        _ => Vec::new(),
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
