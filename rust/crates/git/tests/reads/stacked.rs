//! `rebase.read-stacked-branches` and `repo.read-git-version` against throwaway repositories.

use git::Git;
use git::read::rebase::{StackedRequest, read_stacked_branches};
use git::read::repo::{GitVersionRequest, read_git_version};
use model::rebase::StackedBranches;
use testkit::Repository;

/// `main`, then `p1`, `p2` and `p3`, each one commit above the one before, with `p3` checked out.
fn stack() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    for (index, name) in ["p1", "p2", "p3"].into_iter().enumerate() {
        let _ = repository.git(&["checkout", "--quiet", "-b", name]);
        let file = format!("part{index}.txt");
        let _ = repository.commit(&format!("Add part {index}"), &[(file.as_str(), "part\n")]);
    }
    repository
}

/// The branches of the stack under `p3` for a rebase onto `main`, read with the configuration of the tests alone.
fn read(repository: &Repository) -> StackedBranches {
    let folder = repository.path().parent().unwrap();
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
        .with_variable("GIT_CONFIG_NOSYSTEM", "1");
    let request = StackedRequest {
        base: Some(String::from("main")),
        tip: String::from("p3"),
    };
    read_stacked_branches(&git, &request).unwrap()
}

/// The names read, in order.
fn names(read: &StackedBranches) -> Vec<&str> {
    read.branches.iter().map(|branch| branch.name.as_str()).collect()
}

#[test]
fn rebase_read_stacked_branches__a_rebase_of_p3_onto_main_gives_p1_and_p2_each_with_its_commit() {
    let repository = stack();
    let read = read(&repository);
    assert_eq!(names(&read), ["p1", "p2"]);
    assert_eq!(
        read.branches[0].commit.as_str(),
        repository.git(&["rev-parse", "p1"]).trim()
    );
    assert_eq!(
        read.branches[1].commit.as_str(),
        repository.git(&["rev-parse", "p2"]).trim()
    );
}

#[test]
fn rebase_read_stacked_branches__a_branch_at_the_base_itself_is_not_given() {
    let repository = stack();
    let _ = repository.git(&["branch", "at-base", "main"]);
    assert_eq!(names(&read(&repository)), ["p1", "p2"]);
}

#[test]
fn rebase_read_stacked_branches__a_branch_at_the_tip_of_the_branch_rewritten_is_given() {
    let repository = stack();
    let _ = repository.git(&["branch", "p3-copy", "p3"]);
    assert_eq!(names(&read(&repository)), ["p1", "p2", "p3-copy"]);
}

#[test]
fn rebase_read_stacked_branches__a_branch_of_another_worktree_is_given_with_its_folder() {
    let repository = stack();
    let folder = repository.path().parent().unwrap().join("p2-there");
    let _ = repository.git(&["checkout", "--quiet", "--detach"]);
    let _ = repository.git(&["worktree", "add", "--quiet", &folder.to_string_lossy(), "p2"]);
    let _ = repository.git(&["checkout", "--quiet", "p3"]);
    let read = read(&repository);
    let p2 = read.branches.iter().find(|branch| branch.name == "p2").unwrap();
    let there = p2.worktree.as_ref().unwrap();
    assert_eq!(there.file_name(), folder.file_name(), "{there:?}");
    assert!(
        read.branches
            .iter()
            .find(|branch| branch.name == "p1")
            .unwrap()
            .worktree
            .is_none()
    );
}

#[test]
fn rebase_read_stacked_branches__git_s_setting_is_given_as_set_to_true_or_as_not_set() {
    let repository = stack();
    assert_eq!(read(&repository).update_refs, None);
    let _ = repository.git(&["config", "rebase.updateRefs", "true"]);
    assert_eq!(read(&repository).update_refs, Some(true));
}

#[test]
fn repo_read_git_version__the_git_of_the_tests_gives_its_version() {
    let repository = Repository::new();
    let version = read_git_version(&Git::new(repository.path()), &GitVersionRequest).unwrap();
    assert!(version.major >= 2, "{version:?}");
    assert!(
        version
            .said
            .starts_with(&format!("{}.{}.", version.major, version.minor))
    );
}
