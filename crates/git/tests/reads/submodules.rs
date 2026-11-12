//! `submodule.read-list` against throwaway repositories with submodules.

use git::Git;
use git::read::submodule::{
    SubmoduleListRequest, declared_arguments, parse_declared, parse_recorded, parse_staged, read_list,
    recorded_arguments, staged_arguments,
};
use model::submodule::{Distance, Submodule};
use testkit::Repository;

use crate::golden::{Case, check, record};
use crate::{beside, repository_beside};

/// Adds a repository beside the throwaway one, made with one commit, as a submodule in a folder.
fn add_submodule(repository: &Repository, library: &str, folder: &str) {
    let library = repository_beside(repository, library);
    let _ = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        library.to_str().unwrap(),
        folder,
    ]);
}

/// A repository with one commit and the submodule `lib`, committed.
fn with_lib() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add_submodule(&repository, "library", "lib");
    let _ = repository.commit("Add the library", &[]);
    repository
}

/// The submodules of a repository.
fn read(repository: &Repository) -> Vec<Submodule> {
    let request = SubmoduleListRequest {
        folder: repository.path().to_path_buf(),
    };
    read_list(&Git::new(repository.path()), &request).unwrap()
}

/// How far a submodule is from its recorded commit.
fn distance(submodule: &Submodule) -> Option<Distance> {
    submodule.checkout.as_ref().unwrap().distance
}

#[test]
fn submodule_read_list__a_submodule_on_its_recorded_commit_is_0_ahead_and_0_behind() {
    let repository = with_lib();
    let submodules = read(&repository);
    assert_eq!(submodules.len(), 1);
    assert_eq!(submodules[0].folder, "lib");
    assert_eq!(distance(&submodules[0]), Some(Distance { ahead: 0, behind: 0 }));
    assert_eq!(submodules[0].checkout.as_ref().unwrap().commit, submodules[0].recorded);
}

#[test]
fn submodule_read_list__two_commits_made_in_it_are_2_ahead_with_its_branch_or_none_when_detached() {
    let repository = with_lib();
    for message in ["First change in the library", "Second change in the library"] {
        let _ = repository.git(&["-C", "lib", "commit", "--quiet", "--allow-empty", "-m", message]);
    }
    let submodules = read(&repository);
    assert_eq!(distance(&submodules[0]), Some(Distance { ahead: 2, behind: 0 }));
    assert_eq!(submodules[0].checkout.as_ref().unwrap().branch.as_deref(), Some("main"));
    let _ = repository.git(&["-C", "lib", "checkout", "--quiet", "--detach"]);
    assert_eq!(read(&repository)[0].checkout.as_ref().unwrap().branch, None);
}

#[test]
fn submodule_read_list__a_submodule_whose_folder_is_empty_has_nothing_checked_out() {
    let repository = with_lib();
    let clone = beside(&repository).join("clone");
    let _ = repository.git(&["clone", "--quiet", ".", clone.to_str().unwrap()]);
    let request = SubmoduleListRequest { folder: clone.clone() };
    let submodules = read_list(&Git::new(&clone), &request).unwrap();
    assert_eq!(submodules.len(), 1);
    assert!(submodules[0].recorded.is_some());
    assert_eq!(submodules[0].checkout, None);
}

#[test]
fn submodule_read_list__a_recorded_commit_it_does_not_have_is_not_known_how_far() {
    let mut repository = with_lib();
    let missing = "1234567890abcdef1234567890abcdef12345678";
    let _ = repository.git(&["update-index", "--cacheinfo", &format!("160000,{missing},lib")]);
    let _ = repository.commit("Record a commit the library does not have", &[]);
    let submodules = read(&repository);
    assert_eq!(submodules[0].recorded.as_ref().unwrap().as_str(), missing);
    assert!(submodules[0].checkout.is_some());
    assert_eq!(distance(&submodules[0]), None);
}

#[test]
fn submodule_read_list__a_submodule_of_a_submodule_follows_it_with_it_as_the_one_that_has_it() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let inner = repository_beside(&repository, "b");
    let outer = repository_beside(&repository, "a");
    let outer_text = outer.to_str().unwrap();
    let _ = repository.git(&[
        "-C",
        outer_text,
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        inner.to_str().unwrap(),
        "b",
    ]);
    let _ = repository.git(&["-C", outer_text, "commit", "--quiet", "-m", "Add b"]);
    add_submodule_at(&repository, outer_text, "vendor/a");
    let _ = repository.commit("Add a", &[]);
    let folders: Vec<(String, Option<String>)> = read(&repository)
        .into_iter()
        .map(|submodule| (submodule.folder, submodule.parent))
        .collect();
    assert_eq!(
        folders,
        [
            (String::from("vendor/a"), None),
            (String::from("vendor/a/b"), Some(String::from("vendor/a")))
        ]
    );
}

/// Adds a repository that has submodules as a submodule, with its own checked out.
fn add_submodule_at(repository: &Repository, url: &str, folder: &str) {
    let allow = ["-c", "protocol.file.allow=always"];
    let mut add = allow.to_vec();
    add.extend(["submodule", "add", "--quiet", url, folder]);
    let _ = repository.git(&add);
    let mut update = allow.to_vec();
    update.extend(["submodule", "update", "--quiet", "--init", "--recursive"]);
    let _ = repository.git(&update);
}

#[test]
fn submodule_read_list__a_submodule_added_but_not_committed_has_no_recorded_commit_and_the_staged_one() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add_submodule(&repository, "library", "lib");
    let submodules = read(&repository);
    assert_eq!(submodules[0].recorded, None);
    let staged = submodules[0].staged.clone().unwrap();
    assert_eq!(submodules[0].checkout.as_ref().unwrap().commit, Some(staged));
}

/// A repository with a submodule in a folder with a space, committed, and one added after.
fn awkward_submodules() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add_submodule(&repository, "first library", "vendor/a b");
    let _ = repository.commit("Add the first library", &[]);
    add_submodule(&repository, "second", "lib");
    repository
}

#[test]
fn submodule_read_list__golden_a_folder_with_a_space() {
    let folders = [String::from("vendor/a b"), String::from("lib")];
    let declared = Case {
        read: "submodule-list",
        name: "declared",
    };
    record(declared, awkward_submodules, &declared_arguments());
    check(declared, parse_declared);
    let recorded = Case {
        read: "submodule-list",
        name: "recorded",
    };
    record(recorded, awkward_submodules, &recorded_arguments(&folders));
    check(recorded, |output| parse_recorded(output).unwrap());
    let staged = Case {
        read: "submodule-list",
        name: "staged",
    };
    record(staged, awkward_submodules, &staged_arguments(&folders));
    check(staged, |output| parse_staged(output).unwrap());
}
