//! Which changes ask for the tasks of the worktree shown to be read again (`tasks.read`), beside the read of the
//! repository they may also ask for.

use std::path::Path;

use super::{TreeChange, parts, same_name};
use crate::tasks::{LOCK_FILES, MODULES_FOLDER, PACKAGE_FILE, TASKS_FILE};

/// What a change of a file of the working tree, by its path from the top, needs before it asks for the tasks to be
/// read again: a read at once for `.vscode/tasks.json`, which is read even where git ignores it, and for the whole
/// working tree; a question to git for a `package.json` and for a lock file of a package manager, which count only
/// where git does not ignore them; nothing for the other files, for what is under `node_modules` and for what is in
/// the folder of another worktree nested in this one (`nested`, as [`super::nested_worktrees`] gives them).
///
/// A lock file comes or goes with a name that ends in `.lock` for `yarn` and `bun`: it asks for no read of the
/// repository ([`super::watch_changes`]), and still asks for the tasks, whose package manager it names.
#[must_use]
pub fn sort_tasks_change(path: &Path, nested: &[String]) -> TreeChange {
    let parts = parts(path);
    let is_nested = nested.iter().any(|folder| {
        let folder: Vec<&str> = folder.split('/').collect();
        parts.len() >= folder.len() && parts.iter().zip(&folder).all(|(part, name)| same_name(part, name))
    });
    if is_nested {
        return TreeChange::Nothing;
    }
    let tasks_file: Vec<&str> = TASKS_FILE.split('/').collect();
    let is_tasks_file =
        parts.len() == tasks_file.len() && parts.iter().zip(&tasks_file).all(|(part, name)| same_name(part, name));
    let Some((name, folders)) = parts.split_last() else {
        return TreeChange::Read;
    };
    if is_tasks_file {
        return TreeChange::Read;
    }
    let is_installed = folders.iter().any(|folder| same_name(folder, MODULES_FOLDER));
    let is_of_packages = same_name(name, PACKAGE_FILE) || LOCK_FILES.iter().any(|(lock, _)| same_name(name, lock));
    if is_of_packages && !is_installed {
        TreeChange::AskGit
    } else {
        TreeChange::Nothing
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::sort_tasks_change;
    use crate::repo::{Change, TreeChange, watch_changes};

    fn sort(path: &str) -> TreeChange {
        let nested = [String::from(".claude/worktrees/feature")];
        sort_tasks_change(&PathBuf::from(path), &nested)
    }

    #[test]
    fn tasks_read__a_change_of_tasks_json_asks_for_the_tasks_again_without_refresh() {
        assert_eq!(sort(".vscode/tasks.json"), TreeChange::Read);
        assert_eq!(sort(".vscode\\tasks.json"), TreeChange::Read);
        assert_eq!(sort("."), TreeChange::Read, "the whole working tree");
        assert_eq!(sort(".vscode/settings.json"), TreeChange::Nothing);
        assert_eq!(
            sort("docs/.vscode/tasks.json"),
            TreeChange::Nothing,
            "only the one at the top"
        );
        assert_eq!(sort("src/main.rs"), TreeChange::Nothing);
    }

    #[test]
    fn repo_watch_changes__a_package_json_or_a_lock_file_asks_for_the_tasks_where_git_does_not_ignore_it() {
        for path in [
            "package.json",
            "extension/package.json",
            "extension/pnpm-lock.yaml",
            "yarn.lock",
            "bun.lockb",
            "bun.lock",
            "package-lock.json",
        ] {
            assert_eq!(sort(path), TreeChange::AskGit, "{path}");
        }
        assert_eq!(sort("node_modules/left-pad/package.json"), TreeChange::Nothing);
        assert_eq!(sort("extension/node_modules/.pnpm/lock.yaml"), TreeChange::Nothing);
        assert_eq!(sort("Cargo.lock"), TreeChange::Nothing);
    }

    #[test]
    fn repo_watch_changes__a_yarn_lock_asks_for_the_tasks_though_a_lock_asks_for_no_read() {
        let lock = PathBuf::from("yarn.lock");
        assert_eq!(watch_changes(&Change::WorkingTree(lock.clone())), None);
        assert_eq!(sort_tasks_change(&lock, &[]), TreeChange::AskGit);
    }

    #[test]
    fn repo_watch_changes__the_files_of_tasks_of_a_nested_worktree_ask_for_nothing() {
        assert_eq!(
            sort(".claude/worktrees/feature/.vscode/tasks.json"),
            TreeChange::Nothing
        );
        assert_eq!(sort(".claude/worktrees/feature/package.json"), TreeChange::Nothing);
        assert_eq!(sort(".claude/worktrees/other/package.json"), TreeChange::AskGit);
    }
}
