//! Where the files of the settings and of what is kept between runs are: Gitardo's folder of the user's configuration,
//! as `docs/features/18-settings.md` names it, and `.gitardo/settings.json` of a repository.

use std::env;
use std::path::{Path, PathBuf};

/// The name of Gitardo's folder under the configuration folder of Windows and macOS.
const FOLDER_NAME: &str = "Gitardo Desktop";
/// The name of Gitardo's folder under the configuration folder of Linux.
const LINUX_FOLDER_NAME: &str = "gitardo-desktop";
/// The name of the settings file, in Gitardo's folder and in `.gitardo` of a repository.
const SETTINGS_FILE: &str = "settings.json";
/// The folder of a repository that holds its settings for Gitardo.
const REPOSITORY_FOLDER: &str = ".gitardo";

/// Gitardo's folder of the user's configuration: `%APPDATA%\Gitardo Desktop` on Windows, `~/Library/Application
/// Support/Gitardo Desktop` on macOS, `$XDG_CONFIG_HOME/gitardo-desktop` (or `~/.config/gitardo-desktop`) on Linux.
/// None when the environment does not say where the user's folders are.
#[must_use]
pub fn configuration_folder() -> Option<PathBuf> {
    let variable = |name: &str| env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        variable("APPDATA").map(|folder| folder.join(FOLDER_NAME))
    } else if cfg!(target_os = "macos") {
        variable("HOME").map(|home| home.join("Library/Application Support").join(FOLDER_NAME))
    } else {
        variable("XDG_CONFIG_HOME")
            .or_else(|| variable("HOME").map(|home| home.join(".config")))
            .map(|folder| folder.join(LINUX_FOLDER_NAME))
    }
}

/// The user's settings file, in Gitardo's folder.
#[must_use]
pub fn user_settings_file(configuration: &Path) -> PathBuf {
    configuration.join(SETTINGS_FILE)
}

/// The settings file of a repository: `.gitardo/settings.json` at its top.
#[must_use]
pub fn repository_settings_file(repository: &Path) -> PathBuf {
    repository.join(REPOSITORY_FOLDER).join(SETTINGS_FILE)
}
