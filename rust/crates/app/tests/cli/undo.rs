use testkit::Repository;

use crate::{checked_out, cli, cli_done, revision};

fn repository() -> (Repository, String, String) {
    let mut repository = Repository::new();
    let first = repository.commit("Add a", &[("a.txt", "a")]);
    let second = repository.commit("Fix the parser", &[("b.txt", "b")]);
    (repository, first, second)
}

#[test]
fn undo_last__resets_a_commit_made_in_a_terminal_softly() {
    let (repository, first, _second) = repository();
    let ended = cli_done(&repository, &["--yes", "undo.last"]);
    assert!(
        ended.out.contains("Undo the commit \"Fix the parser\""),
        "{}",
        ended.out
    );
    assert!(
        ended.out.contains(&format!("> git reset --soft {first}")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), first);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "b.txt\n");
}

#[test]
fn undo_last__shows_what_it_will_do_before_it_runs() {
    let (repository, _first, second) = repository();
    let ended = cli(&repository, &["undo.last"]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.out.contains("> git reset --soft"), "{}", ended.out);
    assert!(ended.errors.contains("--yes"), "{}", ended.errors);
    assert_eq!(revision(&repository, "HEAD"), second);
}

#[test]
fn undo_last__checks_out_again_what_was_checked_out_before() {
    let (repository, _first, _second) = repository();
    repository.branch("feature");
    repository.checkout("feature");
    let ended = cli_done(&repository, &["--yes", "undo.last"]);
    assert!(ended.out.contains("> git checkout main"), "{}", ended.out);
    assert_eq!(checked_out(&repository), "main");
}

#[test]
fn undo_last__undo_after_undo_puts_the_commit_back() {
    let (repository, first, second) = repository();
    let _undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "HEAD"), first);
    let _redone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "HEAD"), second);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn undo_last__takes_a_rebase_back_to_where_the_branch_reflog_had_it() {
    let (mut repository, first, second) = repository();
    let _base_branch = repository.git(&["checkout", "--quiet", "-b", "base", &first]);
    let _base = repository.commit("Add c", &[("c.txt", "c")]);
    repository.checkout("main");
    let _rebased = repository.git(&["rebase", "--quiet", "base"]);
    assert_ne!(revision(&repository, "HEAD"), second);
    let ended = cli_done(&repository, &["--yes", "undo.last"]);
    assert!(
        ended.out.contains(&format!("> git reset --keep {second}")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), second);
}
