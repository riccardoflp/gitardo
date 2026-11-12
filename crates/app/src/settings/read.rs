//! Reading the settings: the two files, the repository's first for the settings it may set, a value of the wrong kind
//! left for the default and said as a problem.

use std::io::ErrorKind;
use std::path::Path;

use std::collections::BTreeMap;

use engine::LogLevel;
use json::Value;
use model::contract::{CommandPreview, FeatureSettings, OpenIn};
use model::graph::PageSize;
use model::refs::SortOrder;
use render::ThemeName;
use views::graph::{Switches, TimelineSwitch};

use super::keys::{self, Scope};
use super::{FontChoice, ProblemReason, SettingProblem, Settings, SettingsFile, SettingsFiles};

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
    let mut problems = Vec::new();
    let user = files
        .user
        .as_deref()
        .and_then(|file| file_text(file, SettingsFile::User, &mut problems));
    let repository_file = files.repository.as_deref();
    let repository = repository_file.and_then(|file| file_text(file, SettingsFile::Repository, &mut problems));
    let mut settings = parse(user.as_deref(), repository.as_deref());
    problems.append(&mut settings.problems);
    settings.problems = problems;
    settings
}

/// The settings of the texts of the two files, without reading anything.
#[must_use]
pub fn parse(user: Option<&str>, repository: Option<&str>) -> Settings {
    let (user, user_problem) = document(user, SettingsFile::User);
    let (repository, repository_problem) = document(repository, SettingsFile::Repository);
    let mut lookup = Lookup {
        user,
        repository,
        problems: user_problem.into_iter().chain(repository_problem).collect(),
    };
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
        size: lookup.font_size().unwrap_or(defaults.fonts.size),
    };
    let read_on_startup = lookup.boolean(keys::READ_ON_STARTUP, Scope::UserAndRepository);
    let page_size = lookup.number(keys::PAGE_SIZE, Scope::UserAndRepository);
    let switches = timeline_switches(&mut lookup);
    let sort_order = lookup.choice(keys::SORT_ORDER, &keys::SORT_ORDERS).map(|order| {
        if order == "date" {
            SortOrder::Date
        } else {
            SortOrder::Name
        }
    });
    let follows_graph_filter = lookup.boolean(keys::FOLLOW_GRAPH_FILTER, Scope::User);
    let collapse_unchanged = lookup.boolean(keys::COLLAPSE_UNCHANGED, Scope::User);
    let log_level = match lookup.choice(keys::LOG_LEVEL, &keys::LOG_LEVELS) {
        Some("debug") => LogLevel::Debug,
        _ => LogLevel::Info,
    };
    let features = feature_settings(&mut lookup);
    let values = written_values(&lookup);
    Settings {
        log_level,
        features,
        values,
        theme,
        fonts,
        switches,
        sort_order,
        follows_graph_filter: follows_graph_filter.unwrap_or(defaults.follows_graph_filter),
        collapse_unchanged: collapse_unchanged.unwrap_or(defaults.collapse_unchanged),
        read_on_startup: read_on_startup.unwrap_or(defaults.read_on_startup),
        page_size: page_size.map_or(defaults.page_size, |size| PageSize::from_setting(Some(size))),
        problems: lookup.problems,
    }
}

/// The settings the operations read: `commandPreview`, `branches.format`, `branches.openIn`,
/// `submodules.updateOnCheckout`, `identities`, `pull.rebase` and `pull.autostash`.
fn feature_settings(lookup: &mut Lookup) -> FeatureSettings {
    let defaults = FeatureSettings::default();
    let command_preview = match lookup.choice(keys::COMMAND_PREVIEW, &keys::COMMAND_PREVIEWS) {
        Some("all") => CommandPreview::All,
        _ => CommandPreview::Off,
    };
    FeatureSettings {
        command_preview,
        update_submodules_on_checkout: lookup
            .boolean(keys::UPDATE_SUBMODULES_ON_CHECKOUT, Scope::UserAndRepository)
            .unwrap_or(defaults.update_submodules_on_checkout),
        branch_format: lookup
            .string(keys::BRANCH_FORMAT, Scope::UserAndRepository)
            .unwrap_or(defaults.branch_format),
        branch_open_in: match lookup.choice(keys::BRANCH_OPEN_IN, &keys::BRANCH_OPEN_INS) {
            Some("worktree") => OpenIn::Worktree,
            _ => OpenIn::Checkout,
        },
        pull_rebase: lookup
            .boolean(keys::PULL_REBASE, Scope::UserAndRepository)
            .unwrap_or(defaults.pull_rebase),
        pull_autostash: lookup
            .boolean(keys::PULL_AUTOSTASH, Scope::UserAndRepository)
            .unwrap_or(defaults.pull_autostash),
        identities: lookup
            .first((keys::IDENTITIES, Scope::User), identity_profiles)
            .unwrap_or(defaults.identities),
    }
}

/// `identities`: each entry with a name and an email that are text, in order; the others are left out.
fn identity_profiles(value: &Value) -> Result<Vec<model::contract::IdentityProfile>, ProblemReason> {
    let entries = value
        .as_array()
        .ok_or(ProblemReason::WrongType { expected: "a list" })?;
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
            Some(model::contract::IdentityProfile {
                name: text("name")?,
                email: text("email")?,
                signing_key: text("signingKey"),
            })
        })
        .collect())
}

/// Every value the two files set, by key, the repository's first.
fn written_values(lookup: &Lookup) -> BTreeMap<String, Vec<(SettingsFile, Value)>> {
    let mut values: BTreeMap<String, Vec<(SettingsFile, Value)>> = BTreeMap::new();
    let files = [
        (SettingsFile::Repository, lookup.repository.as_ref()),
        (SettingsFile::User, lookup.user.as_ref()),
    ];
    for (file, document) in files {
        for member in document.and_then(Value::as_object).unwrap_or_default() {
            values
                .entry(member.name.clone())
                .or_default()
                .push((file, member.value.clone()));
        }
    }
    values
}

/// The text of a settings file; none when it is not there, or cannot be read, which is a problem.
fn file_text(path: &Path, file: SettingsFile, problems: &mut Vec<SettingProblem>) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            problems.push(SettingProblem {
                key: String::new(),
                file,
                reason: ProblemReason::Unreadable(error.to_string()),
            });
            None
        }
    }
}

/// The value of a file's text, or the problem of a text that is not JSON with comments.
fn document(text: Option<&str>, file: SettingsFile) -> (Option<Value>, Option<SettingProblem>) {
    match text.map(json::parse_with_comments) {
        None => (None, None),
        Some(Ok(value)) => (Some(value), None),
        Some(Err(error)) => (
            None,
            Some(SettingProblem {
                key: String::new(),
                file,
                reason: ProblemReason::Unreadable(error.to_string()),
            }),
        ),
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
        (keys::TRACE_SELECTED, TimelineSwitch::TraceSelected),
        (keys::DATE_SEPARATORS, TimelineSwitch::DateSeparators),
        (keys::COMBINE_UPSTREAM, TimelineSwitch::CombineUpstream),
    ];
    for (key, switch) in booleans {
        if let Some(is_on) = lookup.boolean(key, Scope::User) {
            switches = switches.with(switch, is_on);
        }
    }
    if let Some(columns) = lookup.columns() {
        let names = [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash];
        for (name, switch) in keys::COLUMN_NAMES.iter().zip(names) {
            switches = switches.with(switch, columns.iter().any(|column| column == name));
        }
    }
    switches
}

/// The two files as read, and the problems found in them so far.
struct Lookup {
    /// The user's file, when it was read.
    user: Option<Value>,
    /// The repository's file, when it was read.
    repository: Option<Value>,
    /// The values not taken.
    problems: Vec<SettingProblem>,
}

impl Lookup {
    /// The values of a key, in the order they are tried: the repository's first for a setting it may set, then the
    /// user's. A setting the repository may not set and does is a problem.
    fn values(&mut self, key: &str, scope: Scope) -> Vec<(Value, SettingsFile)> {
        let mut found = Vec::new();
        if let Some(value) = self.repository.as_ref().and_then(|document| document.get(key)) {
            match scope {
                Scope::UserAndRepository => found.push((value.clone(), SettingsFile::Repository)),
                Scope::User => self.problem(key, SettingsFile::Repository, ProblemReason::UserOnly),
            }
        }
        if let Some(value) = self.user.as_ref().and_then(|document| document.get(key)) {
            found.push((value.clone(), SettingsFile::User));
        }
        found
    }

    /// Notes a value not taken.
    fn problem(&mut self, key: &str, file: SettingsFile, reason: ProblemReason) {
        self.problems.push(SettingProblem {
            key: key.to_owned(),
            file,
            reason,
        });
    }

    /// The first value of a key that a test takes, the others noted as problems with the reason it gives.
    fn first<T>(
        &mut self,
        (key, scope): (&str, Scope),
        take: impl Fn(&Value) -> Result<T, ProblemReason>,
    ) -> Option<T> {
        for (value, file) in self.values(key, scope) {
            match take(&value) {
                Ok(taken) => return Some(taken),
                Err(reason) => self.problem(key, file, reason),
            }
        }
        None
    }

    /// A string.
    fn string(&mut self, key: &str, scope: Scope) -> Option<String> {
        self.first((key, scope), |value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(ProblemReason::WrongType { expected: "a string" })
        })
    }

    /// One of some choices, which the user's file sets.
    fn choice(&mut self, key: &str, choices: &'static [&'static str]) -> Option<&'static str> {
        self.first((key, Scope::User), |value| {
            let text = value
                .as_str()
                .ok_or(ProblemReason::WrongType { expected: "a string" })?;
            choices
                .iter()
                .find(|choice| **choice == text)
                .copied()
                .ok_or(ProblemReason::NotAChoice { choices })
        })
    }

    /// `graph.columns`: the names it lists among `author`, `date` and `hash`; none, which shows all three, when it is
    /// not a list.
    fn columns(&mut self) -> Option<Vec<String>> {
        self.first((keys::COLUMNS, Scope::User), |value| {
            value
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .ok_or(ProblemReason::WrongType { expected: "a list" })
        })
    }

    /// A boolean.
    fn boolean(&mut self, key: &str, scope: Scope) -> Option<bool> {
        self.first((key, scope), |value| {
            value.as_bool().ok_or(ProblemReason::WrongType {
                expected: "true or false",
            })
        })
    }

    /// A number.
    fn number(&mut self, key: &str, scope: Scope) -> Option<f64> {
        self.first((key, scope), |value| {
            value
                .as_number()
                .and_then(json::Number::as_f64)
                .ok_or(ProblemReason::WrongType { expected: "a number" })
        })
    }

    /// `font.size`, a number within [`keys::FONT_SIZES`].
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the size is within 8 and 24 once checked, which an f32 holds exactly enough"
    )]
    fn font_size(&mut self) -> Option<f32> {
        let (least, most) = keys::FONT_SIZES;
        self.first((keys::FONT_SIZE_KEY, Scope::User), |value| {
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
}
