//! `repo.read-state` with other worktrees: `git status` runs in another worktree only when something in its own git
//! folder changed since the read before (M15).

use std::fs;
use std::path::Path;

use git::Git;
use git::read::repo::{StateRequest, read_state_counting};
use git::read::worktree::CountedWorktrees;
use model::state::RepositoryState;
use testkit::Repository;

/// How many times `git status` started, as `GIT_TRACE` wrote it.
fn statuses(trace: &Path) -> usize {
    fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("trace: built-in: git status"))
        .count()
}

/// The count of what is not committed in each worktree, by the name of its folder.
fn counts(state: &RepositoryState) -> Vec<(String, Option<usize>)> {
    state
        .worktrees
        .iter()
        .map(|worktree| {
            let name = worktree.folder.file_name().unwrap().to_string_lossy().into_owned();
            (name, worktree.uncommitted)
        })
        .collect()
}

#[test]
fn repo_read_state__with_five_other_worktrees_a_read_after_a_change_here_runs_git_status_here_only() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let parent = repository.path().parent().unwrap().to_path_buf();
    for index in 0..5 {
        repository.branch(&format!("b{index}"));
        let _ = repository.git(&[
            "worktree",
            "add",
            "--quiet",
            &format!("../w{index}"),
            &format!("b{index}"),
        ]);
    }
    fs::write(parent.join("w2").join("notes.txt"), "a\n").unwrap();
    let trace = parent.join("trace.txt");
    let git = Git::new(repository.path()).with_variable("GIT_TRACE", &trace);
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let (first, counted) = read_state_counting(&git, &request, &CountedWorktrees::default()).unwrap();
    assert_eq!(statuses(&trace), 6, "the first read counts every worktree");
    assert!(counts(&first).contains(&(String::from("w2"), Some(1))));

    fs::write(repository.path().join("here.txt"), "c\n").unwrap();
    fs::write(parent.join("w3").join("unseen.txt"), "d\n").unwrap();
    let (second, counted) = read_state_counting(&git, &request, &counted).unwrap();
    assert_eq!(statuses(&trace), 7, "git status runs here only");
    let others = |state: &RepositoryState| counts(state).into_iter().skip(1).collect::<Vec<_>>();
    assert_eq!(others(&second), others(&first), "each other worktree keeps its count");

    fs::write(parent.join("w1").join("added.txt"), "e\n").unwrap();
    let _ = Git::new(parent.join("w1")).run_read(&["add", "added.txt"]).unwrap();
    let (third, _) = read_state_counting(&git, &request, &counted).unwrap();
    assert_eq!(statuses(&trace), 9, "and in the worktree whose index changed");
    assert!(counts(&third).contains(&(String::from("w1"), Some(1))));

    let (refreshed, _) = read_state_counting(&git, &request, &CountedWorktrees::default()).unwrap();
    assert_eq!(statuses(&trace), 15, "Refresh counts them all");
    assert!(counts(&refreshed).contains(&(String::from("w3"), Some(1))));
}
