//! `repo.worktree-summary` against throwaway repositories: a worktree on a branch with changes and a commit to push, a
//! worktree detached, and one whose folder was deleted.

use std::fs;

use git::read::summary::worktree_summary;
use model::repo::SummaryHead;
use testkit::Repository;

use crate::{beside, write_in};

/// A repository with two files, and a worktree `login` on `feature/login` beside it, following `main`.
fn with_login() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n"), ("lexer.rs", "1\n")]);
    repository.branch("feature/login");
    let _ = repository.git(&["worktree", "add", "--quiet", "../login", "feature/login"]);
    let login = beside(&repository).join("login");
    let _ = repository.git(&[
        "-C",
        login.to_str().unwrap(),
        "branch",
        "--quiet",
        "--set-upstream-to=main",
    ]);
    repository
}

#[test]
fn repo_worktree_summary__a_worktree_on_feature_login_with_three_files_and_a_commit_ahead_reads_so() {
    let repository = with_login();
    let login = beside(&repository).join("login");
    let in_login = |arguments: &[&str]| {
        let mut all = vec!["-C", login.to_str().unwrap()];
        all.extend_from_slice(arguments);
        let _ = repository.git(&all);
    };
    write_in(&login, "form.rs", "1\n");
    in_login(&["add", "--", "form.rs"]);
    in_login(&["commit", "--quiet", "-m", "Add the form"]);
    write_in(&login, "parser.rs", "2\n");
    write_in(&login, "lexer.rs", "2\n");
    write_in(&login, "notes.txt", "new\n");
    let summary = worktree_summary(&login).unwrap();
    assert_eq!(summary.head, SummaryHead::Branch(String::from("feature/login")));
    assert_eq!(summary.changed, 3);
    assert_eq!((summary.ahead(), summary.behind()), (1, 0));
}

#[test]
fn repo_worktree_summary__a_worktree_detached_at_a_commit_reads_as_detached_at_its_short_hash() {
    let repository = with_login();
    let login = beside(&repository).join("login");
    let _ = repository.git(&["-C", login.to_str().unwrap(), "checkout", "--quiet", "--detach"]);
    let hash = repository.git(&["rev-parse", "--short=7", "HEAD"]);
    let summary = worktree_summary(&login).unwrap();
    assert_eq!(summary.head, SummaryHead::Detached(hash.trim().to_owned()));
}

#[test]
fn repo_worktree_summary__a_worktree_whose_folder_was_deleted_is_missing() {
    let repository = with_login();
    let login = beside(&repository).join("login");
    fs::remove_dir_all(&login).unwrap();
    assert_eq!(worktree_summary(&login).unwrap().head, SummaryHead::Missing);
}
