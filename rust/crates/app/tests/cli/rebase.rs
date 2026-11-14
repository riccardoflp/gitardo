use testkit::Repository;

use crate::{checked_out, cli, cli_done, revision, write};

/// `main` at `B` on `A`, and `feature` at `F` on `A`, checked out.
fn diverged() -> (Repository, String, String) {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    repository.branch("feature");
    let b = repository.commit("Add b", &[("b.txt", "b")]);
    repository.checkout("feature");
    let f = repository.commit("Add f", &[("f.txt", "f")]);
    (repository, b, f)
}

/// The subjects of the commits of `HEAD`, newest first.
fn subjects(repository: &Repository) -> Vec<String> {
    repository
        .git(&["log", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The rows of the interactive rebase page, with `<1>`, `<2>`… standing for the hashes given.
fn rows(text: &str, hashes: &[&String]) -> String {
    let mut rows = format!("todo={text}");
    for (index, hash) in hashes.iter().enumerate() {
        rows = rows.replace(&format!("<{}>", index + 1), hash);
    }
    rows
}

/// `A`, then `B` with a body, `C` and `D` checked out: the full hashes of `B`, `C` and `D`.
fn four() -> (Repository, [String; 3]) {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    let b = repository.commit("Add b\n\nThe body of b.", &[("b.txt", "b")]);
    let c = repository.commit("Old words\n\nThe body of c.", &[("c.txt", "c")]);
    let d = repository.commit("Add d", &[("d.txt", "d")]);
    (repository, [b, c, d])
}

#[test]
fn rebase_onto__the_branch_checked_out_onto_another_branch() {
    let (repository, b, _f) = diverged();
    let ended = cli_done(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert!(ended.out.contains("> git rebase main"), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD~1"), b);
    assert_eq!(checked_out(&repository), "feature");
}

#[test]
fn rebase_onto__the_branch_dropped_on_a_commit_rebases_onto_its_full_hash() {
    let (repository, b, _f) = diverged();
    let target = format!("commit:{b}");
    let ended = cli_done(&repository, &["--target", &target, "rebase.onto"]);
    assert!(ended.out.contains(&format!("> git rebase {b}")), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD~1"), b);
}

#[test]
fn rebase_onto__undo_after_a_clean_rebase_puts_the_branch_back() {
    let (repository, _b, f) = diverged();
    let _rebased = cli_done(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert_ne!(revision(&repository, "HEAD"), f);
    let _undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "HEAD"), f);
}

#[test]
fn rebase_onto__stops_on_a_conflict_and_leaves_the_repository_as_git_leaves_it() {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    repository.branch("feature");
    let _b = repository.commit("Change a on main", &[("a.txt", "main")]);
    repository.checkout("feature");
    let _f = repository.commit("Change a on feature", &[("a.txt", "feature")]);
    let ended = cli(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(repository.path().join(".git/rebase-merge").is_dir());
    assert!(repository.git(&["status", "--porcelain"]).contains("UU a.txt"));
    let state = cli_done(&repository, &["state"]);
    assert!(state.out.contains("Operation   Rebase in progress"), "{}", state.out);
}

#[test]
fn rebase_branch_onto__rebases_a_branch_not_checked_out_and_checks_it_out() {
    let (repository, b, _f) = diverged();
    repository.checkout("main");
    let target = format!("commit:{b}");
    let ended = cli_done(
        &repository,
        &["--target", &target, "rebase.branch-onto", "branch=feature"],
    );
    assert!(
        ended.out.contains(&format!("> git rebase {b} feature")),
        "{}",
        ended.out
    );
    assert_eq!(checked_out(&repository), "feature");
    assert_eq!(revision(&repository, "feature~1"), b);
}

#[test]
fn rebase_interactive__a_reword_keeps_the_body_under_the_new_subject() {
    let (repository, [b, c, d]) = four();
    let todo = rows("pick <1>\nreword <2> Better words\npick <3>\n", &[&b, &c, &d]);
    let ended = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    assert!(ended.out.contains("exec git commit --amend -F"), "{}", ended.out);
    assert_eq!(
        repository.git(&["log", "-1", "--format=%B", "HEAD~1"]),
        "Better words\n\nThe body of c.\n\n"
    );
    assert_eq!(subjects(&repository), ["Add d", "Better words", "Add b", "Add a"]);
}

#[test]
fn rebase_interactive__reorders_two_commits_with_new_hashes() {
    let (repository, [b, c, d]) = four();
    let todo = rows("pick <2>\npick <1>\npick <3>\n", &[&b, &c, &d]);
    let _ended = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    assert_eq!(subjects(&repository), ["Add d", "Add b", "Old words", "Add a"]);
    assert_ne!(revision(&repository, "HEAD"), d);
}

#[test]
fn rebase_interactive__drop_leaves_the_commit_out() {
    let (repository, [b, c, d]) = four();
    let todo = rows("pick <1>\ndrop <2>\npick <3>\n", &[&b, &c, &d]);
    let _ended = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    assert_eq!(subjects(&repository), ["Add d", "Add b", "Add a"]);
    assert!(!repository.path().join("c.txt").exists());
}

#[test]
fn rebase_interactive__squash_keeps_both_messages_and_fixup_the_one_below() {
    let (repository, [b, c, d]) = four();
    let todo = rows("pick <1>\nsquash <2>\nfixup <3>\n", &[&b, &c, &d]);
    let _ended = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    assert_eq!(subjects(&repository), ["Add b", "Add a"]);
    let message = repository.git(&["log", "-1", "--format=%B"]);
    assert!(
        message.contains("The body of b.") && message.contains("Old words"),
        "{message}"
    );
    assert!(!message.contains("Add d"), "{message}");
    assert!(repository.path().join("d.txt").exists());
}

#[test]
fn rebase_interactive__from_the_root_runs_rebase_with_root() {
    let (repository, [b, c, d]) = four();
    let a = revision(&repository, "HEAD~3");
    let todo = rows("pick <1>\npick <2>\npick <3>\nreword <4> First\n", &[&a, &b, &c, &d]);
    let ended = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    assert!(ended.out.contains("rebase -i --root"), "{}", ended.out);
    assert_eq!(subjects(&repository), ["First", "Old words", "Add b", "Add a"]);
}

#[test]
fn rebase_interactive__the_confirmation_shows_the_todo_list_before_anything_runs() {
    let (repository, [b, c, d]) = four();
    let todo = rows("pick <1>\ndrop <2>\npick <3>\n", &[&b, &c, &d]);
    let ended = cli(&repository, &["rebase.interactive", &todo]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.out.contains("with this todo list:"), "{}", ended.out);
    assert!(ended.out.contains(&format!("drop {c}")), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD"), d);
}

#[test]
fn rebase_interactive__the_folder_of_its_files_is_gone_after_a_clean_rebase_and_there_after_a_stop() {
    let (repository, [b, c, d]) = four();
    let folder = repository.path().join(".git/gitardo-rebase");
    let clean = rows("pick <1>\nreword <2> Better words\npick <3>\n", &[&b, &c, &d]);
    let _clean = cli_done(&repository, &["--yes", "rebase.interactive", &clean]);
    assert!(!folder.exists());
    let (b, c, d) = (
        revision(&repository, "HEAD~2"),
        revision(&repository, "HEAD~1"),
        revision(&repository, "HEAD"),
    );
    let stopping = rows("edit <1>\nreword <2> Other words\npick <3>\n", &[&b, &c, &d]);
    let _stopped = cli_done(&repository, &["--yes", "rebase.interactive", &stopping]);
    assert!(folder.join("todo").exists());
    let state = cli_done(&repository, &["state"]);
    assert!(state.out.contains("Stopped     to edit"), "{}", state.out);
}

#[test]
fn rebase_edit_stop__continue_after_amending_makes_the_commits_after_it_again() {
    let (repository, [b, c, d]) = four();
    let todo = rows("edit <1>\npick <2>\npick <3>\n", &[&b, &c, &d]);
    let _stopped = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    write(&repository, "b.txt", "b, amended");
    let _amended = repository.git(&["commit", "--quiet", "--all", "--amend", "--no-edit"]);
    let ended = cli_done(&repository, &["rebase.edit-stop"]);
    assert!(
        ended.out.contains("> git -c core.editor=true rebase --continue"),
        "{}",
        ended.out
    );
    assert_eq!(subjects(&repository), ["Add d", "Old words", "Add b", "Add a"]);
    assert_eq!(repository.git(&["show", "HEAD~2:b.txt"]), "b, amended");
    assert!(!repository.path().join(".git/gitardo-rebase").exists());
}

#[test]
fn rebase_edit_stop__continue_with_a_file_modified_is_refused_and_runs_nothing() {
    let (repository, [b, c, d]) = four();
    let todo = rows("edit <1>\npick <2>\npick <3>\n", &[&b, &c, &d]);
    let _stopped = cli_done(&repository, &["--yes", "rebase.interactive", &todo]);
    write(&repository, "b.txt", "b, changed");
    let ended = cli(&repository, &["rebase.edit-stop"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("Some changes are not committed"),
        "{}",
        ended.errors
    );
    assert!(repository.path().join(".git/rebase-merge").is_dir());
}
