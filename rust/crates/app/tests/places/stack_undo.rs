//! One Undo of a rebase that moved the other branches of a stack puts them all back, as the journal of the engine kept
//! them (`undo.moved-branches`).

use std::time::Duration;

use app::store::{Message, State};
use model::contract::{Danger, FeatureId, Plan, Target};
use model::repo::Change;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::OperationsDriver;

/// `p1`, `p2` and `p3` stacked one commit each on `main`, `p3` checked out, and `main` one commit further.
fn stack() -> Repository {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    for name in ["p1", "p2", "p3"] {
        repository.branch(name);
        repository.checkout(name);
        let file = format!("{name}.txt");
        let _part = repository.commit(&format!("Add {name}"), &[(file.as_str(), name)]);
    }
    repository.checkout("main");
    let _b = repository.commit("Add b", &[("b.txt", "b")]);
    repository.checkout("p3");
    repository
}

/// The commit a branch is on.
fn tip(repository: &Repository, name: &str) -> String {
    repository.git(&["rev-parse", name]).trim_end().to_owned()
}

/// Runs an operation of the catalog, says yes to its plan, and gives the plans it showed once it ended.
fn run_to_the_end(
    driver: &OperationsDriver,
    state: &mut State,
    (feature, target): (&'static str, Target),
) -> Vec<Plan> {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
    let mut shown = Vec::new();
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(plan) => {
                shown.push((*plan).clone());
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureEnded { .. } => return shown,
            other => panic!("{feature} asks nothing here, and said {other:?}"),
        }
    }
}

/// The repository read again, as the watcher asks after a change made outside Gitardo.
fn read_again(state: &mut State) {
    let changes = vec![Change::GitFolder("refs/heads/p1".into())];
    let _ = super::support::run(
        state,
        Message::Changed {
            changes,
            at: Duration::from_secs(1),
        },
    );
    let _ = super::support::run(
        state,
        Message::Wake {
            at: Duration::from_secs(11),
        },
    );
}

/// The lines of the command log.
fn log_lines(state: &State) -> Vec<String> {
    state
        .log
        .entries()
        .filter(|entry| super::support::is_ran(entry))
        .map(|entry| entry.line.clone())
        .collect()
}

#[test]
fn undo_moved_branches__after_a_rebase_of_p3_that_moved_p1_and_p2_one_undo_puts_the_three_back() {
    let repository = stack();
    let before = ["p1", "p2", "p3"].map(|name| tip(&repository, name));
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _rebased = run_to_the_end(
        &driver,
        &mut state,
        ("rebase.onto", Target::Branch(String::from("main"))),
    );
    assert_ne!(tip(&repository, "p1"), before[0], "p1 moved with the rebase");
    let shown = run_to_the_end(&driver, &mut state, ("undo.last", Target::Head));
    assert_eq!(["p1", "p2", "p3"].map(|name| tip(&repository, name)), before);
    let lines = log_lines(&state);
    assert!(
        lines.iter().any(|line| line.starts_with("git reset --keep ")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("git update-ref -m \"undo: back to before the rebase\" --stdin")),
        "{lines:?}"
    );
    let Some(Danger::Destructive(said)) = shown.first().map(|plan| &plan.danger) else {
        panic!("Undo asks first: {shown:?}")
    };
    assert_eq!(
        said.as_str(),
        "Undo the rebase of `p3`, and put `p1` and `p2` back where they were."
    );
}

#[test]
fn undo_moved_branches__with_p1_moved_since_the_rebase_undo_puts_p3_and_p2_back_and_says_p1_stays() {
    let repository = stack();
    let before = ["p1", "p2", "p3"].map(|name| tip(&repository, name));
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _rebased = run_to_the_end(
        &driver,
        &mut state,
        ("rebase.onto", Target::Branch(String::from("main"))),
    );
    let tree = repository.git(&["rev-parse", "p1^{tree}"]);
    let parent = tip(&repository, "p1");
    let committed = repository.git(&["commit-tree", tree.trim_end(), "-p", &parent, "-m", "More on p1"]);
    let _moved = repository.git(&["update-ref", "refs/heads/p1", committed.trim_end()]);
    read_again(&mut state);
    let shown = run_to_the_end(&driver, &mut state, ("undo.last", Target::Head));
    assert_eq!(tip(&repository, "p1"), committed.trim_end(), "p1 stays where it is");
    assert_eq!(tip(&repository, "p2"), before[1]);
    assert_eq!(tip(&repository, "p3"), before[2]);
    let Some(Danger::Destructive(said)) = shown.first().map(|plan| &plan.danger) else {
        panic!("Undo asks first: {shown:?}")
    };
    assert!(
        said.as_str()
            .ends_with("`p1` moved since the rebase and stays where it is."),
        "{said}"
    );
}
