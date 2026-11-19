//! Locked worktrees: locked with a reason or none, unlocked, and what **Remove…**, **Rename…** and **Prune** do with
//! one (`worktree.lock`, `worktree.unlock`, `worktree.locked`), in a repository and its folder of worktrees beside it,
//! both deleted with the test.

use std::fs;
use std::path::{Path, PathBuf};

use testkit::Repository;

use crate::{cli, cli_done};

/// A repository with one commit and the worktree of a new branch `feature/login` in the folder of the worktrees.
fn with_login() -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let folder = repository
        .path()
        .with_file_name("repository.worktrees")
        .join("feature/login");
    let _ = repository.git(&["worktree", "add", "-b", "feature/login", &folder.to_string_lossy()]);
    (repository, folder)
}

/// The target of a worktree, by its folder.
fn on(folder: &Path) -> String {
    format!("worktree:{}", folder.to_string_lossy().replace('\\', "/"))
}

/// The worktrees git lists.
fn listed(repository: &Repository) -> String {
    repository.git(&["worktree", "list", "--porcelain"])
}

#[test]
fn worktree_lock__a_reason_is_given_to_git_and_listed_after() {
    let (repository, folder) = with_login();
    let ended = cli_done(
        &repository,
        &["--target", &on(&folder), "worktree.lock", "reason=On the USB drive"],
    );
    assert!(
        ended.out.contains("git worktree lock --reason \"On the USB drive\""),
        "{}",
        ended.out
    );
    assert!(
        listed(&repository).contains("locked On the USB drive"),
        "{}",
        listed(&repository)
    );
}

#[test]
fn worktree_lock__with_the_reason_left_empty_git_locks_it_with_none() {
    let (repository, folder) = with_login();
    let ended = cli_done(&repository, &["--target", &on(&folder), "worktree.lock", "reason="]);
    assert!(!ended.out.contains("--reason"), "{}", ended.out);
    assert!(
        listed(&repository).lines().any(|line| line == "locked"),
        "{}",
        listed(&repository)
    );
}

#[test]
fn worktree_unlock__a_locked_worktree_is_unlocked() {
    let (repository, folder) = with_login();
    let _ = repository.git(&[
        "worktree",
        "lock",
        "--reason",
        "On the USB drive",
        &folder.to_string_lossy(),
    ]);
    let ended = cli_done(&repository, &["--target", &on(&folder), "worktree.unlock"]);
    assert!(ended.out.contains("git worktree unlock"), "{}", ended.out);
    assert!(!listed(&repository).contains("locked"), "{}", listed(&repository));
}

#[test]
fn worktree_unlock__after_unlocking_a_missing_worktree_prune_forgets_it() {
    let (repository, folder) = with_login();
    let _ = repository.git(&[
        "worktree",
        "lock",
        "--reason",
        "On the USB drive",
        &folder.to_string_lossy(),
    ]);
    fs::remove_dir_all(&folder).unwrap();
    let refused = cli(&repository, &["worktree.prune"]);
    assert_ne!(
        refused.code,
        Some(0),
        "not offered while the only missing worktree is locked"
    );
    let _ = cli_done(&repository, &["--target", &on(&folder), "worktree.unlock"]);
    let _ = cli_done(&repository, &["worktree.prune"]);
    assert!(
        !listed(&repository).contains("feature/login"),
        "{}",
        listed(&repository)
    );
}

#[test]
fn worktree_locked__unlock_and_remove_unlocks_then_removes() {
    let (repository, folder) = with_login();
    let _ = repository.git(&[
        "worktree",
        "lock",
        "--reason",
        "On the USB drive",
        &folder.to_string_lossy(),
    ]);
    let target = on(&folder);
    let asked = cli(&repository, &["--target", &target, "worktree.remove", "branch=keep"]);
    assert_eq!(asked.code, Some(2), "{}", asked.errors);
    let said = format!("{}{}", asked.out, asked.errors);
    assert!(said.contains("feature/login is locked: On the USB drive."), "{said}");
    let ended = cli_done(
        &repository,
        &[
            "--yes",
            "--target",
            &target,
            "worktree.remove",
            "branch=keep",
            "unlock=unlock",
        ],
    );
    let unlocked = ended.out.find("git worktree unlock").unwrap_or(usize::MAX);
    let removed = ended.out.find("git worktree remove").unwrap_or(0);
    assert!(unlocked < removed, "{}", ended.out);
    assert!(!folder.exists());
}

#[test]
fn worktree_locked__unlock_and_rename_moves_it_and_locks_the_new_folder_with_the_same_reason() {
    let (repository, folder) = with_login();
    let _ = repository.git(&[
        "worktree",
        "lock",
        "--reason",
        "On the USB drive",
        &folder.to_string_lossy(),
    ]);
    let _ = cli_done(
        &repository,
        &[
            "--target",
            &on(&folder),
            "worktree.rename",
            "unlock=unlock",
            "new name=hotfix/login",
            "rename branch=keep",
        ],
    );
    let list = listed(&repository);
    assert!(
        list.contains("hotfix/login") && list.contains("locked On the USB drive"),
        "{list}"
    );
}
