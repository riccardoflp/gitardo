//! `repo.read-shallow` against clones of a throwaway repository on disk, made with `--depth` through `file://` so that
//! git honors it: whether the history is shallow, and where it ends.

use std::path::{Path, PathBuf};

use git::Git;
use git::read::repo::{ShallowRequest, read_shallow};
use model::CommitId;
use model::state::Shallow;
use testkit::Repository;

/// A repository of some commits, each changing one file.
fn history(commits: usize) -> (Repository, Vec<String>) {
    let mut origin = Repository::new();
    let hashes = (1..=commits)
        .map(|number| {
            let hash = origin.commit(&format!("Change {number}"), &[("file.txt", &number.to_string())]);
            hash.trim().to_owned()
        })
        .collect();
    (origin, hashes)
}

/// The URL of a repository on disk, as `file://`.
fn file_url(repository: &Repository) -> String {
    let path = repository.path().to_string_lossy().replace('\\', "/");
    format!("file:///{}", path.trim_start_matches('/'))
}

/// Clones a repository beside it, with git's arguments before the URL, and gives the folder of the clone.
fn clone(origin: &Repository, name: &str, options: &[&str]) -> PathBuf {
    let folder = origin.path().parent().unwrap().join(name);
    let url = file_url(origin);
    let mut arguments = vec!["clone", "--quiet"];
    arguments.extend(options);
    arguments.extend([url.as_str(), folder.to_str().unwrap()]);
    let _cloned = origin.git(&arguments);
    folder
}

/// Runs git in a folder, in the environment of the tests.
fn git_in(origin: &Repository, folder: &Path, arguments: &[&str]) {
    let output = origin
        .command("git")
        .current_dir(folder)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// What the read says of a folder.
fn shallow_of(folder: &Path) -> Shallow {
    let request = ShallowRequest {
        folder: folder.to_path_buf(),
    };
    read_shallow(&Git::new(folder), &request).unwrap()
}

#[test]
fn repo_read_shallow__a_clone_with_depth_2_of_five_commits_is_shallow_and_ends_at_the_older_of_its_two_commits() {
    let (origin, hashes) = history(5);
    let folder = clone(&origin, "shallow", &["--depth", "2"]);
    let shallow = shallow_of(&folder);
    assert!(shallow.is_shallow());
    assert_eq!(shallow.ends, [CommitId::parse(&hashes[3]).unwrap()]);
}

#[test]
fn repo_read_shallow__a_clone_of_the_whole_history_is_not_shallow() {
    let (origin, _hashes) = history(3);
    let folder = clone(&origin, "whole", &[]);
    assert_eq!(shallow_of(&folder), Shallow::default());
}

#[test]
fn repo_read_shallow__a_worktree_of_a_shallow_repository_reads_the_same_commits_as_its_main_worktree() {
    let (origin, _hashes) = history(5);
    let folder = clone(&origin, "shallow", &["--depth", "2"]);
    let worktree = folder.parent().unwrap().join("shallow-other");
    git_in(
        &origin,
        &folder,
        &["worktree", "add", "--quiet", "-b", "other", worktree.to_str().unwrap()],
    );
    let main = shallow_of(&folder);
    assert!(main.is_shallow());
    assert_eq!(shallow_of(&worktree), main);
}

#[test]
fn repo_read_shallow__after_git_fetch_unshallow_in_a_terminal_the_next_read_says_it_is_not_shallow() {
    let (origin, _hashes) = history(5);
    let folder = clone(&origin, "shallow", &["--depth", "2"]);
    assert!(shallow_of(&folder).is_shallow());
    git_in(&origin, &folder, &["fetch", "--quiet", "--unshallow", "origin"]);
    assert!(!shallow_of(&folder).is_shallow());
}

#[test]
fn repo_read_state__the_state_of_a_shallow_clone_says_where_its_history_ends() {
    let (origin, hashes) = history(4);
    let folder = clone(&origin, "shallow", &["--depth", "1"]);
    let state = super::read_folder(&folder);
    assert_eq!(state.shallow.ends, [CommitId::parse(&hashes[3]).unwrap()]);
}
