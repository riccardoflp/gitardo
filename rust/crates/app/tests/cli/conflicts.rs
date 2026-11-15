//! An operation stopped on conflicts, from `gitardo-cli`: continued, a step skipped, aborted, and a file marked as
//! resolved.

use testkit::{CONFLICT_FILE, Repository};

use crate::{cli, cli_done, revision, write};

/// Runs a git command that stops on conflicts, which fails.
fn stopping(repository: &Repository, arguments: &[&str]) {
    let output = repository.command("git").args(arguments).output();
    assert!(
        output.is_ok_and(|output| !output.status.success()),
        "git {} did not stop",
        arguments.join(" ")
    );
}

/// The subjects of the newest commits, newest first.
fn subjects(repository: &Repository, count: usize) -> Vec<String> {
    repository
        .git(&["log", &format!("-{count}"), "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `a.txt` committed on `main`, then changed on `feature` in the second of three commits and on `main` too, `main`
/// checked out; the commits of `feature`, oldest first.
fn three_commits_one_in_conflict() -> (Repository, [String; 3]) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change a on main", &[("a.txt", "main\n")]);
    repository.checkout("feature");
    let first = repository.commit("Add b", &[("b.txt", "b\n")]);
    let second = repository.commit("Change a", &[("a.txt", "feature\n")]);
    let third = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository.checkout("main");
    (repository, [first, second, third])
}

#[test]
fn conflicts_continue_operation__a_merge_resolved_and_staged_continues_and_makes_the_merge_commit() {
    let mut repository = Repository::new();
    repository.conflict();
    write(&repository, CONFLICT_FILE, "both\n");
    let _ = repository.git(&["add", "--", CONFLICT_FILE]);
    let ended = cli_done(&repository, &["conflicts.continue-operation"]);
    assert!(
        ended.out.contains("git -c core.editor=true merge --continue"),
        "{}",
        ended.out
    );
    let parents = repository.git(&["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 3, "HEAD is not a merge commit");
}

#[test]
fn conflicts_continue_operation__a_cherry_pick_resolved_as_head_has_it_is_skipped_and_says_so() {
    let (repository, [_, second, _]) = three_commits_one_in_conflict();
    let before = revision(&repository, "HEAD");
    stopping(&repository, &["cherry-pick", &second]);
    write(&repository, "a.txt", "main\n");
    let _ = repository.git(&["add", "--", "a.txt"]);
    let ended = cli_done(&repository, &["conflicts.continue-operation"]);
    let short = second.get(..7).unwrap_or_default();
    assert!(
        ended.out.contains(&format!(
            "{short} changes nothing on top of HEAD, so the cherry-pick skips it."
        )),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), before);
}

#[test]
fn conflicts_skip_step__a_cherry_pick_of_3_commits_stopped_on_the_second_skips_it_and_picks_the_third() {
    let (repository, [first, second, third]) = three_commits_one_in_conflict();
    stopping(&repository, &["cherry-pick", &first, &second, &third]);
    let asked = cli(&repository, &["conflicts.skip-step"]);
    assert_eq!(asked.code, Some(2), "{}\n{}", asked.out, asked.errors);
    assert!(
        asked.out.contains("What was resolved in this step is thrown away."),
        "{}",
        asked.out
    );
    let ended = cli_done(&repository, &["--yes", "conflicts.skip-step"]);
    assert!(ended.out.contains("git cherry-pick --skip"), "{}", ended.out);
    assert_eq!(subjects(&repository, 3), ["Add c", "Add b", "Change a on main"]);
}

#[test]
fn conflicts_abort_operation__a_merge_is_aborted_only_once_confirmed() {
    let mut repository = Repository::new();
    repository.conflict();
    let asked = cli(&repository, &["conflicts.abort-operation"]);
    assert_eq!(asked.code, Some(2), "{}\n{}", asked.out, asked.errors);
    assert!(asked.out.contains("Abort merge"), "{}", asked.out);
    let _ended = cli_done(&repository, &["--yes", "conflicts.abort-operation"]);
    assert_eq!(
        std::fs::read_to_string(repository.path().join(CONFLICT_FILE)).unwrap_or_default(),
        "ours\n"
    );
    assert!(!repository.path().join(".git").join("MERGE_HEAD").exists());
}

#[test]
fn conflicts_mark_resolved__a_file_in_conflict_is_staged_with_git_add() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("src/app.rs", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change the app on main", &[("src/app.rs", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Change the app on feature", &[("src/app.rs", "feature\n")]);
    repository.checkout("main");
    stopping(&repository, &["merge", "feature"]);
    write(&repository, "src/app.rs", "both\n");
    let ended = cli_done(&repository, &["--target", "file:src/app.rs", "conflicts.mark-resolved"]);
    assert!(ended.out.contains("git add -- src/app.rs"), "{}", ended.out);
    assert_eq!(repository.git(&["diff", "--name-only", "--diff-filter=U"]), "");
}
