//! The JSON of the settings of new worktrees a run is given and saves under the extension (1.13): what VS Code sets in
//! the repository's folder, with where each comes from, and what the tab of what new worktrees get saves.

use model::contract::SettingTarget;
use model::worktree_setup::{InEffect, SettingSource, SettingsPlace, SetupPatterns, SetupSave, SetupSettings};
use serde::{Deserialize, Serialize};

/// What new worktrees get in the folder of the repository a feature runs in, as VS Code resolves its settings there,
/// sent with `feature/run` of the features that save them (`worktree-setup.*`); it stands for what `settings/changed`
/// sent of them, which is the window's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupGivenJson {
    /// `gitardo.worktrees.linkFolders`, or VS Code's `git.worktreeSymlinkFolders` where it is not set.
    pub link_folders: GivenListJson,
    /// `gitardo.worktrees.copyFiles`, or VS Code's `git.worktreeIncludeFiles` where it is not set.
    pub copy_files: GivenListJson,
    /// `gitardo.worktrees.setupCommands`.
    pub setup_commands: GivenListJson,
    /// Whether the repository is in a folder of the workspace, whose settings are then the repository's; else what is
    /// not set anywhere is written in the user's settings.
    pub in_workspace_folder: bool,
}

/// A list of what new worktrees get, and where it is set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GivenListJson {
    /// The patterns or the commands.
    pub values: Vec<String>,
    /// Where they are set.
    pub from: SetupFromJson,
}

/// Where a list of what new worktrees get is set in VS Code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupFromJson {
    /// The settings of the workspace folder, or of the workspace.
    Workspace,
    /// The user's settings.
    User,
    /// VS Code's own setting, for a list of patterns.
    Vscode,
    /// Nowhere.
    #[default]
    None,
}

impl SetupGivenJson {
    /// What new worktrees get, each list with where it comes from: the workspace's settings stand for the repository's.
    #[must_use]
    pub fn settings(&self) -> SetupSettings {
        let list = |given: &GivenListJson| InEffect {
            values: given
                .values
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            source: match given.from {
                SetupFromJson::Workspace => SettingSource::Repository,
                SetupFromJson::User => SettingSource::User,
                SetupFromJson::Vscode => SettingSource::VsCode,
                SetupFromJson::None => SettingSource::Nothing,
            },
        };
        SetupSettings {
            link_folders: list(&self.link_folders),
            copy_files: list(&self.copy_files),
            setup_commands: list(&self.setup_commands),
        }
    }

    /// Where the settings of the repository are written: its workspace folder, or the user's settings.
    #[must_use]
    pub fn repository_target(&self) -> SettingTarget {
        if self.in_workspace_folder {
            SettingTarget::WorkspaceFolder
        } else {
            SettingTarget::User
        }
    }
}

/// Where the tab of what new worktrees get saves: the repository's place, which is its workspace folder under the
/// extension, or the user's settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsPlaceJson {
    /// The repository's.
    #[default]
    Repository,
    /// The user's.
    User,
}

/// What the tab of what new worktrees get saves, and where (1.13, the target `setupSettings` of `worktree-setup.save`
/// and `worktree-setup.remove-from`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupSaveJson {
    /// Where.
    pub place: SettingsPlaceJson,
    /// The patterns of the folders linked.
    #[serde(default)]
    pub link: Vec<String>,
    /// The patterns of what is copied.
    #[serde(default)]
    pub copy: Vec<String>,
    /// The commands.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Whether VS Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` get the same patterns.
    #[serde(default)]
    pub also_vs_code: bool,
}

impl From<SetupSaveJson> for SetupSave {
    fn from(save: SetupSaveJson) -> Self {
        SetupSave {
            place: match save.place {
                SettingsPlaceJson::Repository => SettingsPlace::Repository,
                SettingsPlaceJson::User => SettingsPlace::User,
            },
            patterns: SetupPatterns {
                link: save.link,
                copy: save.copy,
            },
            commands: save.commands,
            also_vs_code: save.also_vs_code,
        }
    }
}
