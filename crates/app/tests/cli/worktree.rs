//! The worktrees: made, removed, renamed, pruned and opened, in a repository and its folder of worktrees beside it,
//! both deleted with the test.

use std::fs;
use std::path::{Path, PathBuf};

use testkit::{Remote, Repository};

use crate::{cli, cli_done};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository
}

/// The folder of the worktrees when `worktrees.folder` is not set: beside the repository's.
fn trees(repository: &Repository) -> PathBuf {
    repository.path().with_file_name("repository.worktrees")
}

/// A worktree of a new branch made by git, in the folder of the worktrees.
fn added(repository: &Repository, branch: &str) -> PathBuf {
    let folder = trees(repository).join(branch);
    let _ = repository.git(&["worktree", "add", "-b", branch, &folder.to_string_lossy()]);
    folder
}

/// The target of a worktree, by its folder.
fn on(folder: &Path) -> String {
    format!("worktree:{}", folder.to_string_lossy().replace('\\', "/"))
}

/// The worktrees git lists.
fn listed(repository: &Repository) -> String {
    repository.git(&["worktree", "list", "--porcelain"])
}

/// The upstream of a branch, empty when it has none.
fn upstream(repository: &Repository, branch: &str) -> String {
    let reference = format!("refs/heads/{branch}");
    repository
        .git(&["for-each-ref", "--format=%(upstream)", &reference])
        .trim()
        .to_owned()
}

#[test]
fn worktree_create__a_new_worktree_from_a_remote_branch_has_no_upstream() {
    let repository = repository();
    let _remote = Remote::new(&repository);
    let _ = repository.git(&["push", "--quiet", "origin", "main:release"]);
    let _ = repository.git(&["fetch", "--quiet", "origin"]);
    let ended = cli_done(&repository, &["worktree.create", "start=origin/release", "name=try"]);
    assert!(ended.out.contains("Worktree created in"), "{}", ended.out);
    assert!(trees(&repository).join("try/a.txt").is_file());
    assert_eq!(upstream(&repository, "try"), "");
}

#[test]
fn worktree_create__backing_out_creates_nothing() {
    let repository = repository();
    let ended = cli(&repository, &["worktree.create", "start=main"]);
    assert_eq!(ended.code, Some(2));
    assert!(!trees(&repository).exists());
    assert_eq!(listed(&repository).matches("worktree ").count(), 1);
}

#[test]
fn worktree_remove__a_clean_worktree_goes_with_its_branch_and_leaves_no_empty_folder() {
    let repository = repository();
    let folder = added(&repository, "feature/login");
    let target = on(&folder);
    let ended = cli_done(
        &repository,
        &["--yes", "--target", &target, "worktree.remove", "branch=delete"],
    );
    assert!(ended.out.contains("git branch -d feature/login"), "{}", ended.out);
    assert!(!folder.exists());
    assert!(
        !trees(&repository).join("feature").exists(),
        "the folder `feature` was left empty"
    );
    assert_eq!(repository.git(&["branch", "--list", "feature/login"]), "");
}

#[test]
fn worktree_remove__a_worktree_with_a_modified_file_is_kept_and_force_is_offered() {
    let repository = repository();
    let folder = added(&repository, "feature/login");
    fs::write(folder.join("a.txt"), "changed").unwrap();
    let target = on(&folder);
    let ended = cli(
        &repository,
        &["--yes", "--target", &target, "worktree.remove", "branch=keep"],
    );
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(
        ended.out.contains("Next: Remove anyway (worktree.remove)"),
        "{}",
        ended.out
    );
    assert_eq!(fs::read_to_string(folder.join("a.txt")).unwrap(), "changed");
}

#[test]
fn worktree_remove__a_branch_not_merged_is_kept_and_git_says_so() {
    let repository = repository();
    let folder = added(&repository, "feature/login");
    fs::write(folder.join("b.txt"), "b").unwrap();
    let in_worktree = |arguments: &[&str]| {
        let mut all = vec!["-C", folder.to_str().unwrap()];
        all.extend_from_slice(arguments);
        let _ = repository.git(&all);
    };
    in_worktree(&["add", "b.txt"]);
    in_worktree(&["commit", "--quiet", "-m", "Add b"]);
    let target = on(&folder);
    let ended = cli(
        &repository,
        &["--yes", "--target", &target, "worktree.remove", "branch=delete"],
    );
    assert_eq!(ended.code, Some(1));
    assert!(ended.errors.contains("not fully merged"), "{}", ended.errors);
    assert!(!folder.exists());
    assert!(
        repository
            .git(&["branch", "--list", "feature/login"])
            .contains("feature/login")
    );
}

#[cfg(windows)]
#[test]
fn worktree_remove__a_junction_to_the_windows_folder_is_unlinked_first_and_its_files_kept() {
    let mut repository = repository();
    let _ignored = repository.commit("Ignore the modules", &[(".gitignore", "node_modules\n")]);
    fs::create_dir_all(repository.path().join("node_modules")).unwrap();
    crate::write(&repository, "node_modules/left-pad.js", "module");
    let folder = added(&repository, "feature/login");
    let link = folder.join("node_modules");
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(
            repository
                .path()
                .join("node_modules")
                .to_string_lossy()
                .replace('/', "\\"),
        )
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let target = on(&folder);
    let ended = cli_done(
        &repository,
        &["--yes", "--target", &target, "worktree.remove", "branch=keep"],
    );
    assert!(ended.out.contains("unlink "), "{}", ended.out);
    assert!(!folder.exists());
    assert!(repository.path().join("node_modules/left-pad.js").is_file());
}

#[test]
fn worktree_rename__the_folder_and_its_branch_are_renamed_and_no_empty_folder_is_left() {
    let repository = repository();
    let folder = added(&repository, "feature/login");
    let target = on(&folder);
    let _ = cli_done(
        &repository,
        &[
            "--target",
            &target,
            "worktree.rename",
            "new name=hotfix/login",
            "rename branch=rename",
            "branch name=hotfix/login",
        ],
    );
    assert!(trees(&repository).join("hotfix/login/a.txt").is_file());
    assert!(!trees(&repository).join("feature").exists());
    assert!(
        repository
            .git(&["branch", "--list", "hotfix/login"])
            .contains("hotfix/login")
    );
}

#[test]
fn worktree_prune__a_worktree_deleted_outside_is_offered_pruned_and_no_longer_listed() {
    let repository = repository();
    let refused = cli(&repository, &["worktree.prune"]);
    assert_ne!(refused.code, Some(0), "not offered while no worktree is missing");
    let folder = added(&repository, "feature/login");
    fs::remove_dir_all(&folder).unwrap();
    let ended = cli_done(&repository, &["worktree.prune"]);
    assert!(ended.out.contains("git worktree prune"), "{}", ended.out);
    assert!(!listed(&repository).contains("feature/login"));
}

#[test]
fn worktree_prune__the_branch_it_had_can_then_be_checked_out_here() {
    let repository = repository();
    let folder = added(&repository, "feature/login");
    fs::remove_dir_all(&folder).unwrap();
    let _ = cli_done(&repository, &["worktree.prune"]);
    let _ = cli_done(&repository, &["--target", "branch:feature/login", "branch.checkout"]);
    assert_eq!(crate::checked_out(&repository), "feature/login");
}

#[test]
fn worktree_remove_empty_folders__each_example_gives_the_result_it_says() {
    let repository = repository();
    let login = added(&repository, "feature/login");
    let signup = added(&repository, "feature/signup");
    let _ = cli_done(
        &repository,
        &["--yes", "--target", &on(&login), "worktree.remove", "branch=keep"],
    );
    assert!(
        trees(&repository).join("feature").is_dir(),
        "feature/signup is still there"
    );
    let _ = cli_done(
        &repository,
        &["--yes", "--target", &on(&signup), "worktree.remove", "branch=keep"],
    );
    assert!(!trees(&repository).join("feature").exists(), "feature held only it");
    let elsewhere = repository.path().with_file_name("elsewhere").join("x");
    let _ = repository.git(&["worktree", "add", "-b", "x", &elsewhere.to_string_lossy()]);
    let _ = cli_done(
        &repository,
        &["--yes", "--target", &on(&elsewhere), "worktree.remove", "branch=keep"],
    );
    assert!(
        elsewhere.parent().unwrap().is_dir(),
        "nothing outside the folder of the worktrees is deleted"
    );
}

#[test]
fn worktree_remove_empty_folders__the_folder_of_the_worktrees_stays_when_its_last_worktree_is_removed() {
    let repository = repository();
    let fix = added(&repository, "fix");
    let _ = cli_done(
        &repository,
        &["--yes", "--target", &on(&fix), "worktree.remove", "branch=keep"],
    );
    assert!(!fix.exists());
    assert!(trees(&repository).is_dir(), "the folder of the worktrees stays");
}

#[test]
fn worktree_open_in_worktree__a_branch_is_put_in_its_worktree_once_and_then_opened() {
    let repository = repository();
    repository.branch("feature/login");
    let first = cli_done(
        &repository,
        &["--target", "branch:feature/login", "worktree.open-in-worktree"],
    );
    assert!(first.out.contains("in a new window"), "{}", first.out);
    assert!(trees(&repository).join("feature/login/a.txt").is_file());
    assert_eq!(crate::checked_out(&repository), "main");
    let second = cli_done(
        &repository,
        &["--target", "branch:feature/login", "worktree.open-in-worktree"],
    );
    assert!(!second.out.contains("git worktree add"), "{}", second.out);
    assert!(second.out.contains("in a new window"), "{}", second.out);
}

#[test]
fn worktree_open_in_worktree__a_remote_branch_with_no_local_one_makes_one_that_follows_it() {
    let repository = repository();
    let _remote = Remote::new(&repository);
    let _ = repository.git(&["push", "--quiet", "origin", "main:fix"]);
    let _ = repository.git(&["fetch", "--quiet", "origin"]);
    let _ = cli_done(
        &repository,
        &["--target", "remote-branch:origin/fix", "worktree.open-in-worktree"],
    );
    assert!(trees(&repository).join("fix/a.txt").is_file());
    assert_eq!(upstream(&repository, "fix"), "refs/remotes/origin/fix");
}
