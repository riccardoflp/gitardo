//! A cherry-pick of three commits that stopped on a conflict of the second, continued in Gitardo or from a terminal,
//! and what one Undo takes back after it, also once a commit was made in a terminal (`selection.cherry-pick`,
//! `undo.last`).

use std::time::Duration;

use app::store::{Message, State};
use model::CommitId;
use model::contract::{FeatureId, Target};
use model::repo::Change;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::OperationsDriver;

/// `main` and `feature` apart from `Add the readme`, both changing `shared.txt`: `feature` adds `a.txt`, changes
/// `shared.txt`, then adds `c.txt`; `main` checked out. The three commits of `feature`, newest first.
fn three_with_a_conflict_in_the_second() -> (Repository, Vec<CommitId>) {
    let mut repository = Repository::new();
    let _base = repository.commit(
        "Add the readme",
        &[("README.md", "Gitardo\n"), ("shared.txt", "base\n")],
    );
    repository.branch("feature");
    let _main = repository.commit("Change shared on main", &[("shared.txt", "main\n")]);
    repository.checkout("feature");
    let picked = [
        repository.commit("Add a", &[("a.txt", "a\n")]),
        repository.commit("Change shared on feature", &[("shared.txt", "feature\n")]),
        repository.commit("Add c", &[("c.txt", "c\n")]),
    ];
    repository.checkout("main");
    let newest_first = picked
        .iter()
        .rev()
        .map(|hash| CommitId::parse(hash.trim()).unwrap())
        .collect();
    (repository, newest_first)
}

/// Runs an operation of the catalog as a click on its entry does, and waits until it ended, confirming its plan.
fn run_until_ended(driver: &OperationsDriver, state: &mut State, feature: &'static str, target: Target) {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

/// The repository read again, as the watcher asks after a change made outside Gitardo.
fn read_again(state: &mut State, at: u64) {
    let changes = vec![Change::GitFolder("HEAD".into()), Change::GitFolder("index".into())];
    let _ = super::support::run(
        state,
        Message::Changed {
            changes,
            at: Duration::from_secs(at),
        },
    );
    let _ = super::support::run(
        state,
        Message::Wake {
            at: Duration::from_secs(at + 10),
        },
    );
}

/// Cherry-picks the three commits, which stops on the second, and resolves its conflict.
fn stopped_and_resolved() -> (Repository, String, State, OperationsDriver) {
    let (repository, newest_first) = three_with_a_conflict_in_the_second();
    let before = repository.git(&["rev-parse", "HEAD"]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run_until_ended(
        &driver,
        &mut state,
        "selection.cherry-pick",
        Target::Commits(newest_first),
    );
    assert_eq!(
        repository.git(&["ls-files", "-u"]).lines().count(),
        3,
        "stopped on shared.txt"
    );
    std::fs::write(repository.path().join("shared.txt"), "both\n").unwrap();
    let _added = repository.git(&["add", "shared.txt"]);
    read_again(&mut state, 10);
    (repository, before, state, driver)
}

#[test]
fn selection_cherry_pick__a_stop_on_the_second_commit_resolved_and_continued_in_gitardo_is_undone_whole() {
    let (repository, before, mut state, driver) = stopped_and_resolved();
    run_until_ended(&driver, &mut state, "conflicts.continue-operation", Target::Head);
    assert_eq!(repository.git(&["rev-list", "--count", "HEAD"]).trim(), "5");
    run_until_ended(&driver, &mut state, "undo.last", Target::Head);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before);
}

#[test]
fn undo_last__a_cherry_pick_of_three_that_stopped_and_was_continued_in_gitardo_is_undone_by_one_undo() {
    let (repository, before, mut state, driver) = stopped_and_resolved();
    run_until_ended(&driver, &mut state, "conflicts.continue-operation", Target::Head);
    run_until_ended(&driver, &mut state, "undo.last", Target::Head);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn selection_cherry_pick__after_a_stop_that_was_continued_from_a_terminal_undo_takes_back_only_the_last_commit() {
    let (repository, _before, mut state, driver) = stopped_and_resolved();
    let _continued = repository.git(&["-c", "core.editor=true", "cherry-pick", "--continue"]);
    read_again(&mut state, 30);
    let before_the_last = repository.git(&["rev-parse", "HEAD~1"]);
    run_until_ended(&driver, &mut state, "undo.last", Target::Head);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before_the_last);
}

#[test]
fn undo_last__with_a_commit_made_in_a_terminal_after_the_cherry_pick_undo_takes_back_the_terminals_by_the_reflog() {
    let (repository, _before, mut state, driver) = stopped_and_resolved();
    run_until_ended(&driver, &mut state, "conflicts.continue-operation", Target::Head);
    let picked = repository.git(&["rev-parse", "HEAD"]);
    std::fs::write(repository.path().join("d.txt"), "d\n").unwrap();
    let _added = repository.git(&["add", "d.txt"]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Add d"]);
    read_again(&mut state, 30);
    run_until_ended(&driver, &mut state, "undo.last", Target::Head);
    assert_eq!(
        repository.git(&["rev-parse", "HEAD"]),
        picked,
        "only the terminal's commit is taken back"
    );
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]).trim(), "d.txt");
}
