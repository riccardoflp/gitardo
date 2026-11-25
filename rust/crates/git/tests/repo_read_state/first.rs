//! `repo.read-first`: what the first step of the state runs, by the commands git traced, and that the second step
//! runs none of them again.

use std::fs;
use std::path::{Path, PathBuf};

use git::Git;
use git::read::repo::{StateRequest, read_first, read_rest, read_state};
use git::read::worktree::CountedWorktrees;
use testkit::Repository;

/// The commands git started, as `GIT_TRACE` wrote them: one line for each process.
fn started(trace: &Path) -> Vec<String> {
    fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            line.split_once("trace: built-in: git ")
                .map(|(_, command)| command.to_owned())
        })
        .collect()
}

/// A repository with three other worktrees, `w1` with three files not committed, a stash and a file not committed
/// here; with the folder that holds them all.
fn with_three_worktrees() -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let parent = repository.path().parent().unwrap().to_path_buf();
    for index in 0..3 {
        repository.branch(&format!("b{index}"));
        let _ = repository.git(&[
            "worktree",
            "add",
            "--quiet",
            &format!("../w{index}"),
            &format!("b{index}"),
        ]);
    }
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(parent.join("w1").join(name), "new\n").unwrap();
    }
    fs::write(repository.path().join("parser.rs"), "2\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet"]);
    fs::write(repository.path().join("here.txt"), "c\n").unwrap();
    (repository, parent)
}

/// The request of the state of a throwaway repository.
fn request(repository: &Repository) -> StateRequest {
    StateRequest {
        folder: repository.path().to_path_buf(),
    }
}

#[test]
fn repo_read_first__with_three_other_worktrees_it_starts_eight_processes_at_most_and_no_status_elsewhere() {
    let (repository, parent) = with_three_worktrees();
    let trace = parent.join("trace.txt");
    let git = Git::new(repository.path()).with_variable("GIT_TRACE", &trace);
    let first = read_first(&git, &request(&repository)).unwrap();
    let commands = started(&trace);
    assert!(commands.len() <= 8, "eight processes of git at most: {commands:?}");
    let statuses: Vec<&String> = commands
        .iter()
        .filter(|command| command.starts_with("status"))
        .collect();
    assert_eq!(statuses.len(), 1, "git status runs here only: {commands:?}");
    assert_eq!(first.state.worktrees.len(), 4);
    let counts: Vec<Option<usize>> = first
        .state
        .worktrees
        .iter()
        .map(|worktree| worktree.uncommitted)
        .collect();
    assert_eq!(counts, [Some(1), None, None, None], "no other worktree has a count");
}

#[test]
fn repo_read_first__it_reads_what_the_whole_state_reads_of_the_same_things() {
    let (repository, _parent) = with_three_worktrees();
    let git = Git::new(repository.path());
    let first = read_first(&git, &request(&repository)).unwrap().state;
    let whole = read_state(&git, &request(&repository)).unwrap();
    assert_eq!(first.folder, whole.folder);
    assert_eq!(first.git_folder, whole.git_folder);
    assert_eq!(first.branches, whole.branches);
    assert_eq!(first.remote_branches, whole.remote_branches);
    assert_eq!(first.tags, whole.tags);
    assert_eq!(first.head, whole.head, "the same commit and branch checked out");
    assert_eq!(first.stashes, whole.stashes);
    assert_eq!(first.stashes.len(), 1);
    assert_eq!(first.working_tree, whole.working_tree, "the same files not committed");
    assert_eq!(first.operation, whole.operation);
    let folders = |state: &model::state::RepositoryState| -> Vec<PathBuf> {
        state.worktrees.iter().map(|worktree| worktree.folder.clone()).collect()
    };
    assert_eq!(folders(&first), folders(&whole));
}

#[test]
fn repo_read_first__the_state_read_from_what_it_found_starts_none_of_its_commands_again() {
    let (repository, parent) = with_three_worktrees();
    let trace = parent.join("trace.txt");
    let git = Git::new(repository.path()).with_variable("GIT_TRACE", &trace);
    let first = read_first(&git, &request(&repository)).unwrap();
    let before = started(&trace).len();
    let (rest, _counted) = read_rest(&git, first, &CountedWorktrees::default()).unwrap();
    let commands: Vec<String> = started(&trace).split_off(before);
    let first_step = [
        "rev-parse --show-toplevel",
        "for-each-ref",
        "rev-parse --verify --quiet HEAD",
        "symbolic-ref",
        "stash",
        "worktree",
        "log -g",
    ];
    for again in first_step {
        assert!(
            !commands.iter().any(|command| command.starts_with(again)),
            "`git {again}` started again: {commands:?}"
        );
    }
    let statuses: Vec<&String> = commands
        .iter()
        .filter(|command| command.starts_with("status"))
        .collect();
    assert_eq!(statuses.len(), 3, "one git status in each other worktree: {commands:?}");
    assert!(
        statuses.iter().all(|command| !command.contains("--branch")),
        "none here"
    );
    let whole = read_state(&Git::new(repository.path()), &request(&repository)).unwrap();
    assert_eq!(rest, whole, "the two steps give the state a whole read gives");
    let w1 = rest
        .worktrees
        .iter()
        .find(|worktree| worktree.folder.ends_with("w1"))
        .unwrap();
    assert_eq!(w1.uncommitted, Some(3));
}
