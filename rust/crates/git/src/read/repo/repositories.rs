//! The repositories of a workspace: the one each folder is in, the ones directly inside it, and their submodules.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use model::repo::Repository;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// How deep submodules of submodules are looked for, so that a repository that names itself as its own submodule does
/// not make the read go on for ever.
const SUBMODULE_DEPTH: usize = 5;

/// Whether the case of a letter counts in a folder: not on Windows, where `C:\Work\App` and `c:/work/app` are one.
const CASE_COUNTS: bool = !cfg!(windows);

/// Which workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoriesRequest {
    /// The folders of the workspace, in their order.
    pub folders: Vec<PathBuf>,
}

/// The repositories a window can show, for the folders of a workspace in their order: for each folder, the repository
/// it is in and the ones directly inside it, the ones inside first when the folder is inside a repository further up
/// (a folder of projects in a home kept in git); each followed at once by its submodules that are checked out, and
/// theirs. A repository is listed once however many folders lead to it. A folder that cannot be listed, or a
/// `.gitmodules` git cannot read, gives nothing.
///
/// Feature: repo.read-repositories
///
/// # Errors
///
/// When git cannot be started.
pub fn read_repositories(git: &Git, request: &RepositoriesRequest) -> Result<Vec<Repository>, GitError> {
    let mut found: Vec<Repository> = Vec::new();
    for folder in &request.folders {
        for repository in folder_repositories(git, folder)? {
            let key = folder_key(&repository.folder);
            if !found.iter().any(|known| folder_key(&known.folder) == key) {
                found.push(repository);
            }
        }
    }
    Ok(found)
}

/// `git rev-parse --show-toplevel`: the top folder of the working tree a folder is in.
#[must_use]
pub fn top_level_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--show-toplevel"])
}

/// The top folder that `git rev-parse --show-toplevel` printed; none when it printed nothing.
#[must_use]
pub fn parse_top_level(output: &[u8]) -> Option<PathBuf> {
    let text = String::from_utf8_lossy(output);
    let folder = text.lines().next().unwrap_or_default();
    (!folder.is_empty()).then(|| PathBuf::from(folder))
}

/// `git config -z --file .gitmodules --get-regexp ^submodule\..*\.path$`: the folder of each submodule, each key and
/// its value with a line break between them and a NUL after.
#[must_use]
pub fn submodule_paths_arguments() -> Vec<OsString> {
    arguments(&[
        "config",
        "-z",
        "--file",
        ".gitmodules",
        "--get-regexp",
        r"^submodule\..*\.path$",
    ])
}

/// The folders of the submodules that `git config --get-regexp` printed, from the top of the repository.
#[must_use]
pub fn parse_submodule_paths(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .filter_map(|record| record.split_once('\n'))
        .map(|(_, path)| path.trim_end_matches('/').to_owned())
        .filter(|path| !path.is_empty())
        .collect()
}

/// The repositories one folder of the workspace gives, in their order.
fn folder_repositories(git: &Git, folder: &Path) -> Result<Vec<Repository>, GitError> {
    if !folder.is_dir() {
        return Ok(Vec::new());
    }
    let own = top_level(git, folder)?;
    let mut inside = Vec::new();
    for child in children_with_git(folder) {
        inside.extend(top_level(git, &child)?);
    }
    let is_inside_another = own.as_ref().is_some_and(|top| folder_key(top) != folder_key(folder));
    let tops: Vec<PathBuf> = if is_inside_another && !inside.is_empty() {
        inside.into_iter().chain(own).collect()
    } else {
        own.into_iter().chain(inside).collect()
    };
    let mut repositories = Vec::new();
    for top in tops {
        let common_folder = common_folder(git, &top);
        repositories.push(Repository {
            folder: top.clone(),
            superproject: None,
            common_folder,
        });
        add_submodules(git, &top, SUBMODULE_DEPTH, &mut repositories)?;
    }
    Ok(repositories)
}

/// The top folder of the working tree a folder is in, none when it is in none.
fn top_level(git: &Git, folder: &Path) -> Result<Option<PathBuf>, GitError> {
    let output = git.in_directory(folder).read_output(&top_level_arguments())?;
    Ok(if output.is_success() {
        parse_top_level(&output.stdout)
    } else {
        None
    })
}

/// The git folder the worktrees of a repository share (`repo.common-folder`), which puts a worktree in the tab of its
/// repository (`repo.group-tabs`); none when git cannot say.
fn common_folder(git: &Git, top: &Path) -> Option<PathBuf> {
    super::read_common_folder(&git.in_directory(top), &super::CommonFolderRequest).ok()
}

/// The folders directly inside a folder that have a `.git`, a folder or a file, by name.
fn children_with_git(folder: &Path) -> Vec<PathBuf> {
    // A folder that cannot be listed holds nothing this can show: the card says so.
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|child| child.is_dir() && child.join(".git").exists())
        .collect();
    children.sort();
    children
}

/// Adds the submodules of a repository that are checked out, each followed by its own, so many levels down.
fn add_submodules(git: &Git, top: &Path, depth: usize, repositories: &mut Vec<Repository>) -> Result<(), GitError> {
    if depth == 0 || !top.join(".gitmodules").is_file() {
        return Ok(());
    }
    let output = git.in_directory(top).read_output(&submodule_paths_arguments())?;
    // A `.gitmodules` git cannot read gives no submodule; it is no error.
    if !output.is_success() {
        return Ok(());
    }
    for path in parse_submodule_paths(&output.stdout) {
        let folder = PathBuf::from(format!(
            "{}/{path}",
            top.to_string_lossy().trim_end_matches(['/', '\\'])
        ));
        if folder.join(".git").exists() {
            repositories.push(Repository {
                folder: folder.clone(),
                superproject: Some(top.to_path_buf()),
                common_folder: None,
            });
            add_submodules(git, &folder, depth - 1, repositories)?;
        }
    }
    Ok(())
}

/// What a folder is compared by: its spelling with forward slashes and no slash at its end, in lower case on Windows.
fn folder_key(folder: &Path) -> String {
    let text = folder.to_string_lossy().replace('\\', "/");
    let text = text.trim_end_matches('/');
    if CASE_COUNTS {
        text.to_owned()
    } else {
        text.to_lowercase()
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::Path;

    use super::{folder_key, parse_submodule_paths, parse_top_level};

    #[test]
    fn repo_read_repositories__on_windows_the_case_and_the_slashes_do_not_count() {
        if cfg!(windows) {
            assert_eq!(
                folder_key(Path::new(r"C:\Work\App")),
                folder_key(Path::new("c:/work/app/"))
            );
        } else {
            assert_ne!(folder_key(Path::new("/Work/App")), folder_key(Path::new("/work/app")));
        }
    }

    #[test]
    fn repo_read_repositories__reads_the_top_folder_and_the_folders_of_submodules() {
        assert_eq!(
            parse_top_level(b"C:/Work/My App\n").unwrap(),
            Path::new("C:/Work/My App")
        );
        assert_eq!(parse_top_level(b""), None);
        let output = b"submodule.core.path\nlib/core\0submodule.a b.path\nvendor/a b\0";
        assert_eq!(parse_submodule_paths(output), ["lib/core", "vendor/a b"]);
    }
}
