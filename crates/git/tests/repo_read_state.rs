//! `repo.read-state` against throwaway repositories: each line of the card's acceptance list but the last, which belongs
//! to the application.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "repo_read_state/golden.rs"]
mod golden;
#[path = "repo_read_state/speed.rs"]
mod speed;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::state::{Branch, ChangeKind, ConflictKind, ConflictedFile, FileChange, Operation, RepositoryState};
use testkit::{CONFLICT_FILE, FIRST_COMMIT_TIME, Remote, Repository};

/// The state of the repository in a folder, as the window reads it.
fn read_folder(folder: &Path) -> RepositoryState {
    let request = StateRequest {
        folder: folder.to_path_buf(),
    };
    read_state(&Git::new(folder), &request).unwrap()
}

/// The state of a throwaway repository.
fn read(repository: &Repository) -> RepositoryState {
    read_folder(repository.path())
}

/// Writes a file of the working tree, making its folders.
fn write(repository: &Repository, name: &str, content: &str) {
    let path = repository.path().join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Runs a git command that stops half way and fails, as a cherry-pick stopped on a conflict does, with the
/// configuration of the throwaway repository: `Repository::git` would fail the test.
fn run_stopping(repository: &Repository, arguments: &[&str]) {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(repository.path())
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .env(
            "GIT_CONFIG_GLOBAL",
            repository.path().parent().unwrap().join("global.gitconfig"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "Test Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.com")
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap();
    assert!(!output.status.success(), "git {arguments:?} did not stop");
}

/// The local branch of a name.
fn branch<'a>(state: &'a RepositoryState, name: &str) -> &'a Branch {
    state.branches.iter().find(|branch| branch.name == name).unwrap()
}

/// A date of the throwaway repositories, so many minutes after the first commit.
fn minutes_after_first_commit(minutes: i64) -> Timestamp {
    Timestamp(i64::try_from(FIRST_COMMIT_TIME).unwrap() + minutes * 60)
}

#[test]
fn repo_read_state__an_empty_repository_has_no_commit_the_branch_to_come_no_reflog_and_its_untracked_files() {
    let repository = Repository::new();
    write(&repository, "notes.txt", "a\n");
    write(&repository, "src/parser.rs", "fn parse() {}\n");
    let state = read(&repository);
    assert_eq!(state.head.commit, None);
    assert_eq!(state.head.branch.as_deref(), Some("refs/heads/main"));
    assert_eq!(state.reflog_subjects, Vec::<String>::new());
    assert_eq!(state.working_tree.untracked, ["notes.txt", "src/parser.rs"]);
    assert_eq!(state.branches, Vec::new());
    assert_eq!(state.worktrees.len(), 1);
    assert_eq!(state.worktrees[0].commit, None);
}

#[test]
fn repo_read_state__an_empty_repository_has_the_branch_init_default_branch_names() {
    let repository = Repository::new();
    let _ = repository.git(&["-c", "init.defaultBranch=trunk", "init", "--quiet", "../trunk"]);
    let state = read_folder(&repository.path().parent().unwrap().join("trunk"));
    assert_eq!(state.head.branch.as_deref(), Some("refs/heads/trunk"));
    assert_eq!(state.checked_out_branch(), Some("trunk"));
    assert!(!state.has_commit());
}

#[test]
fn repo_read_state__detached_at_a_commit_the_state_has_that_commit_and_no_branch() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _ = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    repository.checkout(&first);
    let state = read(&repository);
    assert_eq!(
        state.head.commit.as_ref().map(model::CommitId::as_str),
        Some(first.as_str())
    );
    assert_eq!(state.head.branch, None);
    assert!(state.branches.iter().all(|branch| !branch.is_checked_out));
    assert_eq!(state.worktrees[0].branch, None);
}

#[test]
fn repo_read_state__a_branch_and_a_tag_of_the_same_name_are_both_read_the_branch_as_checked_out() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.tag("release");
    let second = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    let _ = repository.git(&["checkout", "--quiet", "-b", "release"]);
    let state = read(&repository);
    assert_eq!(state.head.branch.as_deref(), Some("refs/heads/release"));
    let release = branch(&state, "release");
    assert!(release.is_checked_out);
    assert_eq!(release.commit.as_str(), second);
    assert!(!branch(&state, "main").is_checked_out);
    assert_eq!(state.tags.len(), 1);
    assert_eq!(state.tags[0].name, "release");
    assert_eq!(state.tags[0].commit.as_str(), first);
}

#[test]
fn repo_read_state__a_branch_two_ahead_of_its_upstream_and_one_behind_reads_so() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    let _ = repository.commit("Change on the remote", &[("remote.txt", "1\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "main"]);
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let _ = repository.commit("First change here", &[("local.txt", "1\n")]);
    let _ = repository.commit("Second change here", &[("local.txt", "2\n")]);
    let state = read(&repository);
    let upstream = branch(&state, "main").upstream.clone().unwrap();
    assert_eq!(upstream.reference, "refs/remotes/origin/main");
    assert_eq!(upstream.remote, "origin");
    assert_eq!(upstream.remote_branch, "refs/heads/main");
    assert_eq!((upstream.ahead, upstream.behind, upstream.is_gone), (2, 1, false));
}

#[test]
fn repo_read_state__a_branch_whose_upstream_was_deleted_and_pruned_reads_as_gone() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let remote = Remote::new(&repository);
    repository.branch("feature");
    let _ = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _ = remote.git(&["branch", "--quiet", "-D", "feature"]);
    let _ = repository.git(&["fetch", "--quiet", "--prune"]);
    let state = read(&repository);
    let upstream = branch(&state, "feature").upstream.clone().unwrap();
    assert_eq!(upstream.reference, "refs/remotes/origin/feature");
    assert!(upstream.is_gone);
    let remote_branches: Vec<&str> = state
        .remote_branches
        .iter()
        .map(|branch| branch.name.as_str())
        .collect();
    assert_eq!(remote_branches, ["origin/main"]);
}

#[test]
fn repo_read_state__an_annotated_tag_is_read_at_its_commit_with_the_date_it_was_made() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    // The tag gets the date of the next commit, one minute after the first.
    let _ = repository.git(&["tag", "--annotate", "v1.0.0", "--message", "The first release"]);
    let _ = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    let state = read(&repository);
    let tag = state.tags.iter().find(|tag| tag.name == "v1.0.0").unwrap();
    assert_eq!(tag.commit.as_str(), first);
    assert_eq!(tag.date, minutes_after_first_commit(1));
    assert_eq!(branch(&state, "main").date, minutes_after_first_commit(1));
}

#[test]
fn repo_read_state__origin_head_is_not_among_the_remote_branches() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    let _ = repository.git(&["remote", "set-head", "origin", "main"]);
    assert_eq!(
        repository.git(&["symbolic-ref", "refs/remotes/origin/HEAD"]),
        "refs/remotes/origin/main\n"
    );
    let state = read(&repository);
    assert_eq!(state.remote_branches.len(), 1);
    assert_eq!(state.remote_branches[0].name, "origin/main");
    assert_eq!(state.remote_branches[0].remote, "origin");
    assert_eq!(state.remote_branches[0].commit.as_str(), first);
    assert_eq!(state.remotes, ["origin"]);
}

#[test]
fn repo_read_state__a_file_staged_and_changed_again_is_both_staged_and_not_staged() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the parser",
        &[("src/parser.rs", "1\n"), ("old name.txt", "a\nb\nc\n")],
    );
    write(&repository, "src/parser.rs", "2\n");
    let _ = repository.git(&["add", "--", "src/parser.rs"]);
    write(&repository, "src/parser.rs", "3\n");
    let _ = repository.git(&["mv", "old name.txt", "new name.txt"]);
    let state = read(&repository);
    let parser = FileChange {
        path: String::from("src/parser.rs"),
        original_path: None,
        kind: ChangeKind::Modified,
    };
    let renamed = FileChange {
        path: String::from("new name.txt"),
        original_path: Some(String::from("old name.txt")),
        kind: ChangeKind::Renamed,
    };
    assert_eq!(state.working_tree.staged, [renamed, parser.clone()]);
    assert_eq!(state.working_tree.unstaged, [parser]);
    assert_eq!(state.working_tree.file_count(), 2);
}

#[test]
fn repo_read_state__a_file_in_conflict_is_only_in_conflict() {
    let mut repository = Repository::new();
    repository.conflict();
    let state = read(&repository);
    let conflicted = ConflictedFile {
        path: String::from(CONFLICT_FILE),
        kind: ConflictKind::BothModified,
    };
    assert_eq!(state.working_tree.conflicted, [conflicted]);
    assert_eq!(state.working_tree.staged, Vec::new());
    assert_eq!(state.working_tree.unstaged, Vec::new());
    assert_eq!(state.operation, Some(Operation::Merge));
}

#[test]
fn repo_read_state__stopped_in_a_cherry_pick_the_operation_is_a_cherry_pick() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "base\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let picked = repository.commit("Change the parser on feature", &[("parser.rs", "feature\n")]);
    repository.checkout("main");
    let _ = repository.commit("Change the parser on main", &[("parser.rs", "main\n")]);
    run_stopping(&repository, &["cherry-pick", &picked]);
    let state = read(&repository);
    assert_eq!(state.operation, Some(Operation::CherryPick));
    assert_eq!(state.working_tree.conflicted.len(), 1);
}

#[test]
fn repo_read_state__between_two_commits_of_a_revert_of_a_series_the_operation_is_a_revert() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let second = repository.commit("Change the parser", &[("parser.rs", "2\n")]);
    let third = repository.commit("Add the notes", &[("notes.txt", "3\n")]);
    let _ = repository.commit("Change the parser again", &[("parser.rs", "4\n")]);
    run_stopping(&repository, &["revert", "--no-edit", &second, &third]);
    write(&repository, "parser.rs", "5\n");
    let _ = repository.git(&["add", "--", "parser.rs"]);
    let _ = repository.git(&["commit", "--quiet", "--no-edit"]);
    let state = read(&repository);
    assert!(!state.git_folder.join("REVERT_HEAD").exists());
    assert!(state.git_folder.join("sequencer/todo").exists());
    assert_eq!(state.operation, Some(Operation::Revert));
}

#[test]
fn repo_read_state__during_git_am_the_operation_is_a_patch_not_a_rebase() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("other");
    let _ = repository.commit("Change the parser", &[("parser.rs", "2\n")]);
    let patch = repository.path().parent().unwrap().join("change.patch");
    fs::write(&patch, repository.git(&["format-patch", "-1", "--stdout"])).unwrap();
    repository.checkout("other");
    let _ = repository.commit("Change the parser otherwise", &[("parser.rs", "3\n")]);
    run_stopping(&repository, &["am", patch.to_str().unwrap()]);
    let state = read(&repository);
    assert!(state.git_folder.join("rebase-apply").is_dir());
    assert_eq!(state.operation, Some(Operation::Patch));
}

#[test]
fn repo_read_state__another_worktree_with_three_files_not_committed_counts_3_a_missing_one_nothing() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("other");
    repository.branch("gone");
    let _ = repository.git(&["worktree", "add", "--quiet", "../other", "other"]);
    let _ = repository.git(&["worktree", "add", "--quiet", "../gone", "gone"]);
    let folder = repository.path().parent().unwrap();
    fs::write(folder.join("other/parser.rs"), "2\n").unwrap();
    fs::write(folder.join("other/notes.txt"), "a\n").unwrap();
    fs::create_dir(folder.join("other/docs")).unwrap();
    fs::write(folder.join("other/docs/guide.md"), "b\n").unwrap();
    fs::remove_dir_all(folder.join("gone")).unwrap();
    write(&repository, "here.txt", "c\n");
    let state = read(&repository);
    let names: Vec<&str> = state
        .worktrees
        .iter()
        .map(|worktree| worktree.folder.file_name().unwrap().to_str().unwrap())
        .collect();
    assert_eq!(names, ["repository", "gone", "other"]);
    let [main, gone, other] = state.worktrees.as_slice() else {
        panic!("three worktrees")
    };
    assert!(main.is_main && main.is_current && !main.is_missing);
    assert_eq!(main.uncommitted, Some(1));
    assert!(!other.is_main && !other.is_current && !other.is_missing);
    assert_eq!(other.branch.as_deref(), Some("refs/heads/other"));
    assert_eq!(other.uncommitted, Some(3));
    assert!(gone.is_missing);
    assert_eq!(gone.uncommitted, None);
}

#[test]
fn repo_read_state__read_in_another_worktree_marks_it_and_counts_the_main_one() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("other");
    let _ = repository.git(&["worktree", "add", "--quiet", "../other", "other"]);
    write(&repository, "here.txt", "c\n");
    let other_folder = repository.path().parent().unwrap().join("other");
    fs::write(other_folder.join("notes.txt"), "a\n").unwrap();
    let state = read_folder(&other_folder);
    assert_eq!(state.folder, other_folder);
    assert_eq!(state.git_folder, repository.path().join(".git/worktrees/other"));
    assert_eq!(state.head.branch.as_deref(), Some("refs/heads/other"));
    assert_eq!(state.checked_out_branch(), Some("other"));
    let [main, other] = state.worktrees.as_slice() else {
        panic!("two worktrees")
    };
    assert!(main.is_main && !main.is_current);
    assert_eq!(main.uncommitted, Some(1));
    assert!(other.is_current && !other.is_main);
    assert_eq!(other.uncommitted, Some(1));
    assert!(branch(&state, "other").is_checked_out);
    assert!(!branch(&state, "main").is_checked_out);
}

#[test]
fn repo_read_state__no_read_takes_index_lock_and_one_held_does_not_stop_it() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", "2\n");
    let lock = repository.path().join(".git/index.lock");
    fs::write(&lock, "held by a commit in a terminal").unwrap();
    let state = read(&repository);
    assert_eq!(state.working_tree.unstaged.len(), 1);
    assert_eq!(fs::read_to_string(&lock).unwrap(), "held by a commit in a terminal");
    fs::remove_file(&lock).unwrap();
}

#[test]
fn repo_read_state__no_read_writes_the_index_even_when_git_status_would_refresh_it() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    // The file is the same, its date is not: a `git status` that may take the lock refreshes the index and writes it.
    let file = fs::File::options()
        .write(true)
        .open(repository.path().join("parser.rs"))
        .unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(FIRST_COMMIT_TIME))
        .unwrap();
    drop(file);
    let index = repository.path().join(".git/index");
    let before = fs::read(&index).unwrap();
    let state = read(&repository);
    assert!(state.working_tree.is_clean());
    assert_eq!(fs::read(&index).unwrap(), before);
    let _ = repository.git(&["status", "--porcelain"]);
    assert_ne!(
        fs::read(&index).unwrap(),
        before,
        "git status without GIT_OPTIONAL_LOCKS=0 writes the index"
    );
}

#[test]
fn repo_read_state__reads_the_stashes_the_identity_and_the_reflog_from_a_folder_inside() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("src/parser.rs", "1\n")]);
    let _ = repository.git(&["config", "user.name", "Zoë Example"]);
    let _ = repository.git(&["config", "user.email", "zoe@example.com"]);
    write(&repository, "src/parser.rs", "2\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "before [the] merge"]);
    let state = read_folder(&repository.path().join("src"));
    assert_eq!(state.folder, repository.path());
    assert_eq!(state.git_folder, repository.path().join(".git"));
    assert_eq!(state.stashes.len(), 1);
    assert_eq!(state.stashes[0].index, 0);
    assert_eq!(state.stashes[0].message, "On main: before [the] merge");
    assert_eq!(state.identity.name.as_deref(), Some("Zoë Example"));
    assert_eq!(state.identity.email.as_deref(), Some("zoe@example.com"));
    assert_eq!(
        state.reflog_subjects,
        ["reset: moving to HEAD", "commit (initial): Add the parser"]
    );
    assert!(state.working_tree.is_clean());
    assert_eq!(state.operation, None);
}
