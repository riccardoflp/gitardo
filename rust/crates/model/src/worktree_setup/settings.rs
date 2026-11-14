//! Which link and copy patterns and which commands apply to new worktrees of a repository, and where each comes from
//! (`worktree-setup.settings-in-effect`).

/// Where a value in effect comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingSource {
    /// The repository's `.gitardo/settings.json`.
    Repository,
    /// The user's settings file.
    User,
    /// VS Code's matching setting in the repository's `.vscode/settings.json`.
    VsCode,
    /// Nothing sets it.
    Nothing,
}

/// A list in effect, and where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InEffect {
    /// The patterns or the commands.
    pub values: Vec<String>,
    /// Where they come from.
    pub source: SettingSource,
}

/// What each place sets for one key: none when the key is not there, a list (maybe empty) when it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Places<'a> {
    /// The repository's `.gitardo/settings.json`.
    pub repository: Option<&'a [String]>,
    /// The user's settings file.
    pub user: Option<&'a [String]>,
}

/// What [`settings_in_effect`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SettingsInput<'a> {
    /// `worktrees.linkFolders`.
    pub link_folders: Places<'a>,
    /// `worktrees.copyFiles`.
    pub copy_files: Places<'a>,
    /// `worktrees.setupCommands`.
    pub setup_commands: Places<'a>,
    /// `git.worktreeSymlinkFolders` in the repository's `.vscode/settings.json`.
    pub vscode_link_folders: Option<&'a [String]>,
    /// `git.worktreeIncludeFiles` in the repository's `.vscode/settings.json`.
    pub vscode_copy_files: Option<&'a [String]>,
}

/// What new worktrees get, as set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSettings {
    /// The folders linked.
    pub link_folders: InEffect,
    /// The files and folders copied.
    pub copy_files: InEffect,
    /// The commands run.
    pub setup_commands: InEffect,
}

/// The patterns trimmed, without empty ones, without line breaks and without the same one twice.
fn patterns(values: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for value in values.iter().map(|value| value.trim()) {
        if !value.is_empty() && !value.contains(['\n', '\r']) && !kept.iter().any(|known| known == value) {
            kept.push(value.to_owned());
        }
    }
    kept
}

/// The commands trimmed, without empty ones and without line breaks, in their order.
fn commands(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty() && !value.contains(['\n', '\r']))
        .map(str::to_owned)
        .collect()
}

/// The list of one key: the repository's when the key is there, then the user's, then VS Code's when it has a
/// pattern.
fn pick(places: Places<'_>, vscode: Option<&[String]>, clean: fn(&[String]) -> Vec<String>) -> InEffect {
    if let Some(values) = places.repository {
        return InEffect {
            values: clean(values),
            source: SettingSource::Repository,
        };
    }
    if let Some(values) = places.user {
        return InEffect {
            values: clean(values),
            source: SettingSource::User,
        };
    }
    match vscode.map(clean) {
        Some(values) if !values.is_empty() => InEffect {
            values,
            source: SettingSource::VsCode,
        },
        _ => InEffect {
            values: Vec::new(),
            source: SettingSource::Nothing,
        },
    }
}

/// The link and copy patterns and the commands new worktrees get, each key on its own, with where each comes from.
///
/// Feature: worktree-setup.settings-in-effect
#[must_use]
pub fn settings_in_effect(input: &SettingsInput<'_>) -> SetupSettings {
    SetupSettings {
        link_folders: pick(input.link_folders, input.vscode_link_folders, patterns),
        copy_files: pick(input.copy_files, input.vscode_copy_files, patterns),
        setup_commands: pick(input.setup_commands, None, commands),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{InEffect, Places, SettingSource, SettingsInput, settings_in_effect};

    fn list(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn worktree_setup_settings_in_effect__an_empty_list_in_the_repository_links_nothing() {
        let (empty, user) = (list(&[]), list(&["node_modules/"]));
        let input = SettingsInput {
            link_folders: Places {
                repository: Some(&empty),
                user: Some(&user),
            },
            ..SettingsInput::default()
        };
        assert_eq!(
            settings_in_effect(&input).link_folders,
            InEffect {
                values: Vec::new(),
                source: SettingSource::Repository
            }
        );
    }

    #[test]
    fn worktree_setup_settings_in_effect__vs_code_setting_alone_is_taken() {
        let vscode = list(&["node_modules/"]);
        let input = SettingsInput {
            vscode_link_folders: Some(&vscode),
            ..SettingsInput::default()
        };
        assert_eq!(
            settings_in_effect(&input).link_folders,
            InEffect {
                values: list(&["node_modules/"]),
                source: SettingSource::VsCode
            }
        );
        let blank = list(&["  "]);
        let input = SettingsInput {
            vscode_link_folders: Some(&blank),
            ..SettingsInput::default()
        };
        assert_eq!(settings_in_effect(&input).link_folders.source, SettingSource::Nothing);
    }

    #[test]
    fn worktree_setup_settings_in_effect__each_key_comes_from_its_own_place() {
        let (copies, links) = (list(&["/.env"]), list(&["node_modules/"]));
        let input = SettingsInput {
            copy_files: Places {
                repository: None,
                user: Some(&copies),
            },
            vscode_link_folders: Some(&links),
            vscode_copy_files: Some(&links),
            ..SettingsInput::default()
        };
        let settings = settings_in_effect(&input);
        assert_eq!(
            (settings.copy_files.values, settings.copy_files.source),
            (copies.clone(), SettingSource::User)
        );
        assert_eq!(settings.link_folders.source, SettingSource::VsCode);
        assert_eq!(settings.setup_commands.source, SettingSource::Nothing);
    }

    #[test]
    fn worktree_setup_settings_in_effect__patterns_are_trimmed_without_empty_ones_or_twice_the_same() {
        let (copies, commands) = (
            list(&[" /.env ", "", "/.env", "a\nb"]),
            list(&[" npm ci ", "", "npm ci", "make\nall"]),
        );
        let input = SettingsInput {
            copy_files: Places {
                repository: Some(&copies),
                user: None,
            },
            setup_commands: Places {
                repository: None,
                user: Some(&commands),
            },
            ..SettingsInput::default()
        };
        let settings = settings_in_effect(&input);
        assert_eq!(settings.copy_files.values, ["/.env"]);
        assert_eq!(settings.setup_commands.values, ["npm ci", "npm ci"]);
    }
}
