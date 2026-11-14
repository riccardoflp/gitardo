//! Cherry-picking one commit or several, getting past those that change nothing, and reverting.

use testkit::Repository;

use crate::{cli, cli_done, revision, write};

/// `main` with `Add a`, and `feature` from it with `Add b`, `Change a` and `Add c`; `main` checked out.
fn with_feature() -> (Repository, [String; 3]) {
    let mut repository = Repository::new();
    let _base = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let first = repository.commit("Add b", &[("b.txt", "b\n")]);
    let second = repository.commit("Change a", &[("a.txt", "feature\n")]);
    let third = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository.checkout("main");
    (repository, [first, second, third])
}

fn subjects(repository: &Repository, count: usize) -> Vec<String> {
    repository
        .git(&["log", "--format=%s", &format!("-{count}")])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn is_under_way(repository: &Repository) -> bool {
    repository.path().join(".git/CHERRY_PICK_HEAD").exists() || repository.path().join(".git/sequencer").exists()
}

#[test]
fn cherry_pick_commit__a_commit_that_conflicts_in_a_txt_asks_first_and_names_it() {
    let (mut repository, [_, second, _]) = with_feature();
    let _main = repository.commit("Change a on main", &[("a.txt", "main\n")]);
    let target = format!("commit:{second}");
    let ended = cli(&repository, &["--target", &target, "cherry-pick.commit"]);
    assert_eq!(ended.code, Some(2), "{}", ended.errors);
    assert!(
        ended.out.contains("This will stop with conflicts in 1 file:\na.txt"),
        "{}",
        ended.out
    );
    assert!(!is_under_way(&repository));
}

#[test]
fn cherry_pick_commit__a_commit_that_does_not_conflict_runs_without_a_dialog() {
    let (repository, [first, _, _]) = with_feature();
    let target = format!("commit:{first}");
    let ended = cli_done(&repository, &["--target", &target, "cherry-pick.commit"]);
    assert!(
        ended.out.contains(&format!("> git cherry-pick {first}\n")),
        "{}",
        ended.out
    );
    assert_eq!(subjects(&repository, 1), ["Add b"]);
}

#[test]
fn cherry_pick_commit__a_merge_commit_is_picked_against_its_first_parent() {
    let (mut repository, _) = with_feature();
    let _other = repository.git(&["checkout", "--quiet", "-b", "other", "main"]);
    let _ours = repository.commit("Add o", &[("o.txt", "o\n")]);
    repository.merge("feature");
    let merge = revision(&repository, "HEAD");
    repository.checkout("main");
    let target = format!("commit:{merge}");
    let ended = cli_done(&repository, &["--target", &target, "cherry-pick.commit"]);
    assert!(
        ended.out.contains(&format!("> git cherry-pick -m 1 {merge}\n")),
        "{}",
        ended.out
    );
    assert!(repository.path().join("c.txt").exists());
}

#[test]
fn cherry_pick_commit__undo_resets_the_branch_to_the_commit_before_the_pick() {
    let (repository, [first, _, _]) = with_feature();
    let before = revision(&repository, "HEAD");
    let target = format!("commit:{first}");
    let _picked = cli_done(&repository, &["--target", &target, "cherry-pick.commit"]);
    let ended = cli_done(&repository, &["--yes", "undo.last"]);
    assert!(
        ended.out.contains(&format!("> git reset --keep {before}")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), before);
}

#[test]
fn cherry_pick_skip_empty__a_commit_already_applied_is_skipped_with_a_warning() {
    let (repository, [first, _, _]) = with_feature();
    let _applied = repository.git(&["cherry-pick", &first]);
    let before = revision(&repository, "HEAD");
    let target = format!("commit:{first}");
    let ended = cli_done(&repository, &["--target", &target, "cherry-pick.commit"]);
    let short = first.get(..7).unwrap_or_default();
    assert!(
        ended.out.contains(&format!(
            "{short} changes nothing on top of HEAD, so the cherry-pick skips it."
        )),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), before);
    assert!(!is_under_way(&repository));
}

#[test]
fn cherry_pick_skip_empty__three_commits_of_which_the_second_is_applied_end_with_two_new_commits() {
    let (repository, [first, second, third]) = with_feature();
    let _applied = repository.git(&["cherry-pick", &second]);
    let target = format!("commits:{third},{second},{first}");
    let _ended = cli_done(&repository, &["--target", &target, "selection.cherry-pick"]);
    assert_eq!(subjects(&repository, 4), ["Add c", "Add b", "Change a", "Add a"]);
    assert!(!is_under_way(&repository));
}

#[test]
fn cherry_pick_skip_empty__with_a_file_changed_and_not_committed_the_stop_is_left_with_its_error() {
    let (mut repository, [first, _, _]) = with_feature();
    let _other = repository.commit("Add d", &[("d.txt", "d\n")]);
    let _applied = repository.git(&["cherry-pick", &first]);
    write(&repository, "d.txt", "changed\n");
    let target = format!("commit:{first}");
    let ended = cli(&repository, &["--target", &target, "cherry-pick.commit"]);
    assert_eq!(ended.code, Some(1));
    assert!(ended.errors.contains("git cherry-pick"), "{}", ended.errors);
    assert!(is_under_way(&repository));
}

#[test]
fn cherry_pick_skip_empty__with_a_conflict_still_marked_nothing_is_skipped() {
    let (mut repository, [_, second, _]) = with_feature();
    let _main = repository.commit("Change a on main", &[("a.txt", "main\n")]);
    let target = format!("commit:{second}");
    let ended = cli(&repository, &["--yes", "--target", &target, "cherry-pick.commit"]);
    assert_eq!(ended.code, Some(1));
    assert!(is_under_way(&repository));
    assert_eq!(repository.git(&["diff", "--name-only", "--diff-filter=U"]), "a.txt\n");
}

#[test]
fn revert_commit__a_commit_with_one_parent_runs_git_revert_no_edit() {
    let (repository, [first, _, _]) = with_feature();
    repository.checkout("feature");
    let target = format!("commit:{first}");
    let ended = cli_done(&repository, &["--target", &target, "revert.commit"]);
    assert!(
        ended.out.contains(&format!("> git revert --no-edit {first}\n")),
        "{}",
        ended.out
    );
    assert!(!repository.path().join("b.txt").exists());
}

#[test]
fn revert_commit__a_merge_commit_runs_git_revert_no_edit_m_1() {
    let (mut repository, _) = with_feature();
    repository.merge("feature");
    let merge = revision(&repository, "HEAD");
    let target = format!("commit:{merge}");
    let ended = cli_done(&repository, &["--target", &target, "revert.commit"]);
    assert!(
        ended.out.contains(&format!("> git revert --no-edit -m 1 {merge}\n")),
        "{}",
        ended.out
    );
    assert!(!repository.path().join("c.txt").exists());
}

#[test]
fn revert_commit__the_new_commit_says_revert_and_the_subject() {
    let (repository, [first, _, _]) = with_feature();
    repository.checkout("feature");
    let target = format!("commit:{first}");
    let _ended = cli_done(&repository, &["--target", &target, "revert.commit"]);
    assert_eq!(subjects(&repository, 1), ["Revert \"Add b\""]);
}

#[test]
fn selection_cherry_pick__picks_a_b_c_oldest_first_whatever_the_order_selected() {
    let (repository, [first, second, third]) = with_feature();
    let target = format!("commits:{third},{second},{first}");
    let ended = cli_done(&repository, &["--target", &target, "selection.cherry-pick"]);
    assert!(
        ended
            .out
            .contains(&format!("> git cherry-pick {first} {second} {third}\n")),
        "{}",
        ended.out
    );
    assert_eq!(subjects(&repository, 3), ["Add c", "Change a", "Add b"]);
}

#[test]
fn selection_cherry_pick__a_merge_among_them_is_refused_before_git_runs() {
    let (mut repository, [first, _, _]) = with_feature();
    repository.merge("feature");
    let merge = revision(&repository, "HEAD");
    let _back = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let target = format!("commits:{merge},{first}");
    let ended = cli(&repository, &["--target", &target, "selection.cherry-pick"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("A merge commit can only be cherry-picked on its own."),
        "{}",
        ended.errors
    );
    assert!(!is_under_way(&repository));
}
