//! The reads of the working tree and of what a repository holds, against throwaway repositories, and their parsers on
//! output recorded in awkward repositories (`tests/golden/<read>`).

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "reads/conflicts.rs"]
mod conflicts;
#[path = "reads/github.rs"]
mod github;
#[path = "reads/golden.rs"]
mod golden;
#[path = "reads/remotes_and_stashes.rs"]
mod remotes_and_stashes;
#[path = "reads/repositories.rs"]
mod repositories;
#[path = "reads/submodules.rs"]
mod submodules;
#[path = "reads/working_tree.rs"]
mod working_tree;
#[path = "reads/worktree_setup.rs"]
mod worktree_setup;
#[path = "reads/worktree_setup_explain.rs"]
mod worktree_setup_explain;
#[path = "reads/worktrees.rs"]
mod worktrees;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use testkit::Repository;

/// Writes a file of the working tree, making its folders.
fn write(repository: &Repository, name: &str, content: &str) {
    write_in(repository.path(), name, content);
}

/// Writes a file in a folder, making the folders it is in.
fn write_in(folder: &Path, name: &str, content: &str) {
    let path = folder.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// The folder that holds the throwaway repository, and the other repositories a test makes beside it.
fn beside(repository: &Repository) -> PathBuf {
    repository.path().parent().unwrap().to_path_buf()
}

/// Runs git in the repository's folder with the environment of its helpers, and returns how it ended, for a command
/// that may fail.
fn run(repository: &Repository, arguments: &[&str]) -> Output {
    repository.command("git").args(arguments).output().unwrap()
}

/// The last name of a folder.
fn name_of(folder: &Path) -> String {
    folder.file_name().unwrap().to_string_lossy().into_owned()
}

/// Makes a repository beside the throwaway one, in a folder of that name, with one commit of a file, and returns its
/// folder.
fn repository_beside(repository: &Repository, folder: &str) -> PathBuf {
    let path = beside(repository).join(folder);
    let path_text = path.to_str().unwrap();
    let _ = repository.git(&["init", "--quiet", "-b", "main", path_text]);
    write_in(&path, "README.md", &format!("{folder}\n"));
    let _ = repository.git(&["-C", path_text, "add", "--", "README.md"]);
    let _ = repository.git(&["-C", path_text, "commit", "--quiet", "-m", "Add the readme"]);
    path
}

/// Makes a junction on Windows, a symbolic link elsewhere, from a path to a folder.
fn link_folder(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        // `mklink` takes a forward slash for the start of an option.
        let windows_path = |path: &Path| path.to_string_lossy().replace('/', "\\");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(windows_path(link))
            .arg(windows_path(target))
            .output()
            .unwrap();
        assert!(status.status.success(), "{status:?}");
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, link).unwrap();
}
