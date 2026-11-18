//! The files the tasks of a worktree are read from (`tasks.read`): its `.vscode/tasks.json`, every `package.json`
//! outside `node_modules` and the folders git ignores, and the lock files that name a package manager.

use std::ffi::OsString;
use std::fs;
use std::io;

use model::tasks::{LOCK_FILES, MODULES_FOLDER, PACKAGE_FILE, PackageFile, TASKS_FILE, TaskFiles};

use crate::{Git, GitError};

/// The files of the worktree git runs in that define tasks, as they are on disk, edits not committed included. A
/// file git lists that is no longer on disk is left out, and so is `.vscode/tasks.json` when there is none.
///
/// # Errors
///
/// When git cannot be started or fails, or a file that is there cannot be read.
pub fn read_task_files(git: &Git) -> Result<TaskFiles, GitError> {
    let listed = git.run_read(&task_files_arguments())?;
    let (package_paths, lock_files) = parse_task_files(&listed);
    let mut packages = Vec::new();
    for path in package_paths {
        if let Some(text) = text_on_disk(git, &path)? {
            packages.push(PackageFile { path, text });
        }
    }
    Ok(TaskFiles {
        tasks_json: text_on_disk(git, TASKS_FILE)?,
        packages,
        lock_files,
    })
}

/// `git ls-files -z --cached --others --exclude-standard -- <package.json and the lock files, in any folder>`: the
/// files git tracks, and those it does not track nor ignore.
#[must_use]
pub fn task_files_arguments() -> Vec<OsString> {
    let listing = ["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--"];
    let names = [PACKAGE_FILE]
        .into_iter()
        .chain(LOCK_FILES.iter().map(|(name, _)| *name));
    listing
        .into_iter()
        .map(OsString::from)
        .chain(names.map(|name| OsString::from(format!(":(glob)**/{name}"))))
        .collect()
}

/// The `package.json` files and the lock files [`task_files_arguments`] listed, each ended by a NUL, each once and in
/// order: those under a `node_modules` are of packages installed, and are left out.
#[must_use]
pub fn parse_task_files(output: &[u8]) -> (Vec<String>, Vec<String>) {
    let text = String::from_utf8_lossy(output);
    let mut paths: Vec<&str> = text.split('\0').filter(|path| !path.is_empty()).collect();
    paths.sort_unstable();
    paths.dedup();
    let mut packages = Vec::new();
    let mut lock_files = Vec::new();
    for path in paths {
        let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
        if folder.split('/').any(|part| part == MODULES_FOLDER) {
            continue;
        }
        if name == PACKAGE_FILE {
            packages.push(path.to_owned());
        } else if LOCK_FILES.iter().any(|(lock_file, _)| *lock_file == name) {
            lock_files.push(path.to_owned());
        }
    }
    (packages, lock_files)
}

/// The text of a file of the worktree as it is on disk; none when it is not there.
fn text_on_disk(git: &Git, path: &str) -> Result<Option<String>, GitError> {
    let full = git.directory().join(path);
    match fs::read(&full) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GitError::File { path: full, source }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use std::fs;
    use std::path::Path;

    use super::{parse_task_files, read_task_files, task_files_arguments};
    use crate::{Git, command_line};

    const PACKAGE: &str = "{ \"scripts\": { \"build\": \"node build.mjs\" } }\n";

    fn write(folder: &Path, path: &str, text: &str) {
        let file = folder.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    fn paths(files: &model::tasks::TaskFiles) -> Vec<&str> {
        files.packages.iter().map(|file| file.path.as_str()).collect()
    }

    #[test]
    fn asks_git_for_the_package_files_and_the_lock_files_of_every_folder() {
        assert_eq!(
            command_line(&task_files_arguments()),
            "git ls-files -z --cached --others --exclude-standard -- :(glob)**/package.json \
             :(glob)**/pnpm-lock.yaml :(glob)**/yarn.lock :(glob)**/bun.lockb :(glob)**/bun.lock \
             :(glob)**/package-lock.json :(glob)**/npm-shrinkwrap.json"
        );
    }

    #[test]
    fn keeps_each_file_once_and_leaves_out_those_of_packages_installed() {
        let listed = b"package.json\0extension/package.json\0extension/pnpm-lock.yaml\0extension/package.json\0\
            node_modules/left-pad/package.json\0extension/node_modules/vitest/package.json\0yarn.lock\0\
            extension/node_modules/vitest/yarn.lock\0";
        let (packages, lock_files) = parse_task_files(listed);
        assert_eq!(packages, ["extension/package.json", "package.json"]);
        assert_eq!(lock_files, ["extension/pnpm-lock.yaml", "yarn.lock"]);
        assert_eq!(parse_task_files(b""), (Vec::new(), Vec::new()));
    }

    #[test]
    fn tasks_read__the_files_are_those_of_the_worktree_outside_node_modules_and_what_git_ignores() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit(
            "Add the packages",
            &[
                (".gitignore", "out/\n"),
                (".vscode/tasks.json", "{ \"tasks\": [] }\n"),
                ("package.json", PACKAGE),
                ("package-lock.json", "{}\n"),
                ("extension/package.json", PACKAGE),
                ("node_modules/left-pad/package.json", PACKAGE),
            ],
        );
        let folder = repository.path().to_path_buf();
        write(&folder, "web/package.json", "{ \"scripts\": { \"dev\": \"vite\" } }\n");
        write(&folder, "web/pnpm-lock.yaml", "lockfileVersion: 9\n");
        write(&folder, "web/node_modules/vite/package.json", PACKAGE);
        write(&folder, "out/package.json", PACKAGE);
        fs::remove_file(folder.join("extension").join("package.json")).unwrap();
        let files = read_task_files(&Git::new(&folder)).unwrap();
        assert_eq!(files.tasks_json.as_deref(), Some("{ \"tasks\": [] }\n"));
        assert_eq!(
            paths(&files),
            ["package.json", "web/package.json"],
            "one not committed yet is read; one deleted, ignored or installed is not"
        );
        assert_eq!(
            files.packages.last().unwrap().text,
            "{ \"scripts\": { \"dev\": \"vite\" } }\n"
        );
        assert_eq!(files.lock_files, ["package-lock.json", "web/pnpm-lock.yaml"]);
    }

    #[test]
    fn tasks_read__a_linked_worktree_reads_its_own_files_as_they_are_on_disk() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit(
            "Add the tasks",
            &[(".vscode/tasks.json", "{ \"tasks\": [] }\n"), ("package.json", PACKAGE)],
        );
        let linked = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &linked.to_string_lossy()]);
        let edited = "{ \"tasks\": [{ \"label\": \"build\", \"type\": \"shell\", \"command\": \"make\" }] }\n";
        write(&linked, ".vscode/tasks.json", edited);
        write(&linked, "extension/package.json", PACKAGE);
        let of_linked = read_task_files(&Git::new(&linked)).unwrap();
        assert_eq!(
            of_linked.tasks_json.as_deref(),
            Some(edited),
            "the edit that is not committed"
        );
        assert_eq!(paths(&of_linked), ["extension/package.json", "package.json"]);
        let of_main = read_task_files(&Git::new(repository.path())).unwrap();
        assert_eq!(of_main.tasks_json.as_deref(), Some("{ \"tasks\": [] }\n"));
        assert_eq!(
            paths(&of_main),
            ["package.json"],
            "the linked worktree inside it is not its own"
        );
    }

    #[test]
    fn tasks_read__a_worktree_with_no_file_of_tasks_has_none() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
        let files = read_task_files(&Git::new(repository.path())).unwrap();
        assert_eq!(files, model::tasks::TaskFiles::default());
    }
}
