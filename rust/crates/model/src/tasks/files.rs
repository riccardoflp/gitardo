//! What the tasks of a worktree are read from (`tasks.read`): the files found in it, and what the variables of
//! `tasks.json` stand for there.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::PackageManager;

/// The system the tasks run on, which chooses among the overrides `windows`, `linux` and `osx` of `tasks.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSystem {
    /// Windows: `windows`.
    Windows,
    /// Linux and the other systems that are neither Windows nor macOS: `linux`.
    Linux,
    /// macOS: `osx`.
    MacOs,
}

impl TaskSystem {
    /// The system Gitardo runs on.
    #[must_use]
    pub fn current() -> Self {
        if cfg!(windows) {
            TaskSystem::Windows
        } else if cfg!(target_os = "macos") {
            TaskSystem::MacOs
        } else {
            TaskSystem::Linux
        }
    }

    /// The member of `tasks.json` that holds what a task overrides on this system.
    #[must_use]
    pub fn override_name(self) -> &'static str {
        match self {
            TaskSystem::Windows => "windows",
            TaskSystem::Linux => "linux",
            TaskSystem::MacOs => "osx",
        }
    }

    /// `${pathSeparator}`: what separates the folders of a path.
    #[must_use]
    pub fn path_separator(self) -> &'static str {
        match self {
            TaskSystem::Windows => "\\",
            TaskSystem::Linux | TaskSystem::MacOs => "/",
        }
    }
}

/// A `package.json` of the worktree, outside `node_modules` and the folders git ignores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFile {
    /// The file from the top of the worktree, with `/`: `package.json`, `extension/package.json`.
    pub path: String,
    /// Its text.
    pub text: String,
}

/// The files the tasks are read from, as they are on disk.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskFiles {
    /// The text of `.vscode/tasks.json`; none when the worktree has none.
    pub tasks_json: Option<String>,
    /// Every `package.json`, in any order.
    pub packages: Vec<PackageFile>,
    /// The lock files of the package managers, from the top of the worktree with `/`: `extension/pnpm-lock.yaml`.
    pub lock_files: Vec<String>,
}

/// Everything the tasks are read from: no file, no environment and no system is asked but through this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksContext {
    /// The top folder of the worktree shown, which is `${workspaceFolder}` and `${cwd}`.
    pub folder: PathBuf,
    /// The system the tasks run on.
    pub system: TaskSystem,
    /// The home folder of the user, which is `${userHome}`.
    pub home: PathBuf,
    /// The environment of Gitardo, for `${env:NAME}`.
    pub environment: BTreeMap<String, String>,
    /// The files found in the worktree.
    pub files: TaskFiles,
}

/// The names of the lock files, with the package manager each names, in the order they count when a folder has
/// several: `pnpm`, `yarn`, `bun`, then `npm`.
pub const LOCK_FILES: [(&str, PackageManager); 6] = [
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    ("yarn.lock", PackageManager::Yarn),
    ("bun.lockb", PackageManager::Bun),
    ("bun.lock", PackageManager::Bun),
    ("package-lock.json", PackageManager::Npm),
    ("npm-shrinkwrap.json", PackageManager::Npm),
];

/// The file of the scripts.
pub const PACKAGE_FILE: &str = "package.json";

/// The file of the tasks of a worktree, from its top.
pub const TASKS_FILE: &str = ".vscode/tasks.json";

/// The folder whose `package.json` files are those of the packages installed, never listed.
pub const MODULES_FOLDER: &str = "node_modules";

/// The package manager that runs the scripts of the `package.json` of a folder, from the top of the worktree with `/`
/// and empty for the top: the one the lock file in that folder names, or the nearest above it; `npm` with none.
#[must_use]
pub fn package_manager(folder: &str, lock_files: &[String]) -> PackageManager {
    let mut folder = folder.trim_matches('/');
    loop {
        let found = LOCK_FILES.iter().find(|(name, _)| {
            lock_files.iter().any(|file| {
                let (inside, file_name) = file.rsplit_once('/').unwrap_or(("", file));
                inside == folder && file_name == *name
            })
        });
        if let Some((_, manager)) = found {
            return *manager;
        }
        if folder.is_empty() {
            return PackageManager::Npm;
        }
        folder = folder.rsplit_once('/').map_or("", |(above, _)| above);
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageManager, package_manager};

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn the_lock_file_beside_the_package_names_its_manager() {
        let cases = [
            ("pnpm-lock.yaml", PackageManager::Pnpm),
            ("yarn.lock", PackageManager::Yarn),
            ("bun.lockb", PackageManager::Bun),
            ("bun.lock", PackageManager::Bun),
            ("package-lock.json", PackageManager::Npm),
        ];
        for (lock, manager) in cases {
            assert_eq!(package_manager("", &files(&[lock])), manager, "{lock}");
            let nested = format!("extension/{lock}");
            assert_eq!(package_manager("extension", &files(&[&nested])), manager, "{nested}");
        }
        assert_eq!(package_manager("extension", &[]), PackageManager::Npm);
    }

    #[test]
    fn the_nearest_lock_file_above_counts_and_one_beside_another_package_does_not() {
        let locks = files(&["yarn.lock", "packages/api/pnpm-lock.yaml"]);
        assert_eq!(package_manager("packages/api", &locks), PackageManager::Pnpm);
        assert_eq!(package_manager("packages/api/tools", &locks), PackageManager::Pnpm);
        assert_eq!(package_manager("packages/web", &locks), PackageManager::Yarn);
        assert_eq!(package_manager("", &locks), PackageManager::Yarn);
        let beside_another = files(&["packages/api/pnpm-lock.yaml"]);
        assert_eq!(package_manager("packages/web", &beside_another), PackageManager::Npm);
    }

    #[test]
    fn pnpm_counts_before_yarn_bun_and_npm_in_one_folder() {
        let locks = files(&["package-lock.json", "bun.lockb", "yarn.lock", "pnpm-lock.yaml"]);
        assert_eq!(package_manager("", &locks), PackageManager::Pnpm);
        assert_eq!(
            package_manager("", &files(&["package-lock.json", "bun.lock"])),
            PackageManager::Bun
        );
    }
}
