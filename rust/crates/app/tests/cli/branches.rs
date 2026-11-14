use testkit::{Remote, Repository};

use crate::{checked_out, cli, cli_done, revision, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository
}

#[test]
fn branch_rename__keeps_the_upstream_of_the_branch() {
    let repository = repository();
    let _remote = Remote::new(&repository);
    let _feature = repository.git(&["branch", "feature"]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let ended = cli_done(
        &repository,
        &["--target", "branch:feature", "branch.rename", "name=feature/login"],
    );
    assert!(
        ended.out.contains("> git branch -m feature feature/login"),
        "{}",
        ended.out
    );
    assert_eq!(
        repository
            .git(&["rev-parse", "--abbrev-ref", "feature/login@{upstream}"])
            .trim(),
        "origin/feature"
    );
}

#[test]
fn branch_rename__keeps_the_branch_checked_out_checked_out() {
    let repository = repository();
    let ended = cli_done(&repository, &["--target", "branch:main", "branch.rename", "name=trunk"]);
    assert!(ended.out.contains("> git branch -m main trunk"), "{}", ended.out);
    assert_eq!(checked_out(&repository), "trunk");
}

#[test]
fn branch_delete__a_branch_merged_into_its_upstream_asks_then_deletes_with_d() {
    let repository = repository();
    let _remote = Remote::new(&repository);
    let _feature = repository.git(&["branch", "feature"]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let asked = cli(&repository, &["--target", "branch:feature", "branch.delete"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.out);
    assert!(asked.out.contains("Its commits are merged"), "{}", asked.out);
    let ended = cli_done(&repository, &["--yes", "--target", "branch:feature", "branch.delete"]);
    assert!(ended.out.contains("> git branch -d feature"), "{}", ended.out);
    assert_eq!(repository.git(&["branch", "--list", "feature"]), "");
}

#[test]
fn branch_delete__a_branch_not_merged_is_deleted_with_capital_d_after_the_warning() {
    let mut repository = repository();
    let _feature = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let _own = repository.commit("Add b", &[("b.txt", "b")]);
    repository.checkout("main");
    let asked = cli(&repository, &["--target", "branch:feature", "branch.delete"]);
    assert_eq!(asked.code, Some(2), "it warns first: {}", asked.out);
    assert!(asked.out.contains("not fully merged"), "{}", asked.out);
    let ended = cli_done(&repository, &["--yes", "--target", "branch:feature", "branch.delete"]);
    assert!(ended.out.contains("> git branch -D feature"), "{}", ended.out);
}

/// `main`, and `feature` one commit past it following `origin/feature`, which is one commit further.
fn behind() -> (Repository, Remote) {
    let mut repository = repository();
    let remote = Remote::new(&repository);
    let _feature = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let _own = repository.commit("Add b", &[("b.txt", "b")]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _next = repository.commit("Add c", &[("c.txt", "c")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "feature"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    repository.checkout("main");
    (repository, remote)
}

#[test]
fn branch_fast_forward__fetches_the_upstream_into_the_branch_and_leaves_the_files() {
    let (repository, _remote) = behind();
    write(&repository, "a.txt", "changed here");
    let ended = cli_done(&repository, &["--target", "branch:feature", "branch.fast-forward"]);
    assert!(
        ended
            .out
            .contains("> git fetch origin refs/heads/feature:refs/heads/feature"),
        "{}",
        ended.out
    );
    assert_eq!(
        revision(&repository, "feature"),
        revision(&repository, "origin/feature")
    );
    assert_eq!(checked_out(&repository), "main");
    assert_eq!(repository.git(&["status", "--porcelain"]), " M a.txt\n");
}

#[test]
fn branch_fast_forward__pulls_in_the_worktree_that_has_the_branch() {
    let (repository, _remote) = behind();
    let folder = repository.path().with_file_name(format!(
        "{}-feature",
        repository
            .path()
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned())
    ));
    let folder_text = folder.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", &folder_text, "feature"]);
    let ended = cli_done(&repository, &["--target", "branch:feature", "branch.fast-forward"]);
    assert!(
        ended
            .out
            .contains("pull --ff-only --no-rebase --no-autostash origin refs/heads/feature"),
        "{}",
        ended.out
    );
    assert_eq!(
        revision(&repository, "feature"),
        revision(&repository, "origin/feature")
    );
    assert!(folder.join("c.txt").exists(), "the files of the worktree moved with it");
    let _removed = repository.git(&["worktree", "remove", "--force", &folder_text]);
}

#[test]
fn branch_fast_forward__shows_why_git_refuses_after_a_force_push() {
    let (repository, remote) = behind();
    let before = revision(&repository, "feature");
    // The remote's `feature` goes back to the first commit, as a force push would, since the last fetch.
    let first = revision(&repository, "main");
    let _forced = remote.git(&["update-ref", "refs/heads/feature", &first]);
    let ended = cli(&repository, &["--target", "branch:feature", "branch.fast-forward"]);
    let said = format!("{}{}", ended.out, ended.errors);
    assert_eq!(ended.code, Some(1), "{said}");
    assert!(said.contains("non-fast-forward") || said.contains("rejected"), "{said}");
    assert_eq!(revision(&repository, "feature"), before);
}

#[test]
fn branch_clean_up__deletes_the_branches_ticked_merged_and_gone() {
    let mut repository = repository();
    let _remote = Remote::new(&repository);
    let _merged = repository.git(&["branch", "a"]);
    let _gone = repository.git(&["checkout", "--quiet", "-b", "b"]);
    let _own = repository.commit("Add b", &[("b.txt", "b")]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "b"]);
    repository.checkout("main");
    let _deleted = repository.git(&["push", "--quiet", "origin", "--delete", "b"]);
    let asked = cli(&repository, &["branch.clean-up"]);
    assert_eq!(asked.code, Some(2), "{}", asked.errors);
    assert!(asked.errors.contains("branches=a"), "{}", asked.errors);
    assert!(asked.errors.contains("branches=b"), "{}", asked.errors);
    let ended = cli_done(&repository, &["--yes", "branch.clean-up", "branches=a,b"]);
    assert!(ended.out.contains("> git branch -d a"), "{}", ended.out);
    assert!(ended.out.contains("> git branch -D b"), "{}", ended.out);
    assert_eq!(repository.git(&["branch", "--list", "a", "b"]), "");
}

#[test]
fn branch_clean_up__removes_the_worktree_of_a_merged_branch_first() {
    let repository = repository();
    let _merged = repository.git(&["branch", "a"]);
    let folder = repository.path().with_file_name(format!(
        "{}-a",
        repository
            .path()
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned())
    ));
    let folder_text = folder.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", &folder_text, "a"]);
    let ended = cli_done(&repository, &["--yes", "branch.clean-up", "branches=a"]);
    let removed = ended.out.find("> git worktree remove");
    let deleted = ended.out.find("> git branch -d a");
    assert!(removed.is_some() && removed < deleted, "{}", ended.out);
    assert!(!folder.exists());
}

#[test]
fn branch_clean_up__says_so_when_there_is_nothing_to_clean_up() {
    let repository = repository();
    let ended = cli(&repository, &["branch.clean-up"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("No local branches are merged or have lost their upstream."),
        "{}",
        ended.errors
    );
}
