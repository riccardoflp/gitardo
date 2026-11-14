//! `repo.read-repositories` against folders of throwaway repositories.

use std::fs;
use std::path::PathBuf;

use git::Git;
use git::read::repo::{RepositoriesRequest, read_repositories};
use model::repo::Repository as Found;
use testkit::Repository;

use crate::{beside, name_of, repository_beside};

/// The repositories a workspace of these folders gives.
fn read(folders: &[PathBuf]) -> Vec<Found> {
    let request = RepositoriesRequest {
        folders: folders.to_vec(),
    };
    read_repositories(&Git::new(folders.first().unwrap()), &request).unwrap()
}

/// The last names of the folders of the repositories found.
fn names(found: &[Found]) -> Vec<String> {
    found.iter().map(|repository| name_of(&repository.folder)).collect()
}

/// Adds a repository beside the throwaway one as the submodule `lib/core`, and commits it.
fn add_submodule(repository: &mut Repository) {
    let library = repository_beside(repository, "core");
    let _ = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        library.to_str().unwrap(),
        "lib/core",
    ]);
    let _ = repository.commit("Add the core library", &[]);
}

#[test]
fn repo_read_repositories__a_folder_that_is_a_repository_gives_it_alone() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let found = read(&[repository.path().to_path_buf()]);
    assert_eq!(names(&found), ["repository"]);
    assert_eq!(found[0].superproject, None);
}

#[test]
fn repo_read_repositories__a_folder_of_repositories_gives_each_by_name() {
    let repository = Repository::new();
    let workspace = beside(&repository).join("workspace");
    for name in ["b", "a"] {
        let _ = repository_beside(&repository, &format!("workspace/{name}"));
    }
    fs::create_dir(workspace.join("not a repository")).unwrap();
    assert_eq!(names(&read(&[workspace])), ["a", "b"]);
}

#[test]
fn repo_read_repositories__a_folder_of_projects_in_a_home_kept_in_git_gives_the_projects_first() {
    let mut home = Repository::new();
    let _ = home.commit("Keep the settings", &[(".profile", "1\n")]);
    let projects = home.path().join("projects");
    let _ = home.git(&["init", "--quiet", "projects/projects-x"]);
    assert_eq!(names(&read(&[projects])), ["projects-x", "repository"]);
    assert_eq!(names(&read(&[home.path().to_path_buf()])), ["repository"]);
}

#[test]
fn repo_read_repositories__a_submodule_checked_out_follows_its_repository() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add_submodule(&mut repository);
    let found = read(&[repository.path().to_path_buf()]);
    assert_eq!(names(&found), ["repository", "core"]);
    assert!(found[1].folder.to_string_lossy().ends_with("repository/lib/core"));
    assert_eq!(found[1].superproject.as_deref(), Some(found[0].folder.as_path()));
}

#[test]
fn repo_read_repositories__a_submodule_not_initialized_is_left_out() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add_submodule(&mut repository);
    let clone = beside(&repository).join("clone");
    let _ = repository.git(&["clone", "--quiet", ".", clone.to_str().unwrap()]);
    assert!(clone.join("lib/core").is_dir() && !clone.join("lib/core/.git").exists());
    assert_eq!(names(&read(&[clone])), ["clone"]);
}

#[test]
fn repo_read_repositories__two_folders_inside_one_repository_give_it_once() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("src/parser.rs", "1\n"), ("docs/guide.md", "1\n")]);
    let folders = [repository.path().join("src"), repository.path().join("docs")];
    assert_eq!(names(&read(&folders)), ["repository"]);
}

#[test]
fn repo_read_repositories__a_folder_that_cannot_be_listed_gives_nothing() {
    let repository = Repository::new();
    let gone = beside(&repository).join("gone");
    let request = RepositoriesRequest { folders: vec![gone] };
    assert_eq!(read_repositories(&Git::new(repository.path()), &request).unwrap(), []);
}

#[cfg(windows)]
#[test]
fn repo_read_repositories__on_windows_two_spellings_of_a_folder_are_one_repository() {
    let repository = Repository::new();
    let backslashes = PathBuf::from(repository.path().to_string_lossy().replace('/', "\\"));
    let other = PathBuf::from(repository.path().to_string_lossy().replace('\\', "/").to_lowercase());
    assert_ne!(backslashes, other);
    let found = read(&[backslashes, other]);
    assert_eq!(found.len(), 1);
    assert!(found[0].folder.ends_with("repository"));
}
