//! Operations on branches run from the window: a double click on a branch of the rail, the commit checked out on a
//! detached `HEAD` as the header says it, and a branch deleted then brought back by Undo.

use app::store::State;
use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;

use super::support::{OperationsDriver, Window, says};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 520);

/// Runs an operation of the catalog on a reference, as its menu does, says yes to its plan when it shows one, and
/// waits for it to end.
fn run_to_the_end(driver: &OperationsDriver, state: &mut State, (feature, target): (&'static str, Target)) {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
    loop {
        match driver.next(state) {
            app::store::Message::FeatureConfirms(_) => {
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            app::store::Message::FeatureEnded { .. } => return,
            other => panic!("{feature} asks nothing here, and said {other:?}"),
        }
    }
}

/// Whether the label of a branch in the timeline is the filled one, of what is checked out.
fn is_filled(state: &State, name: &str) -> bool {
    state
        .timeline
        .layout
        .labels
        .values()
        .flatten()
        .any(|label| label.name == name && label.is_checked_out)
}

#[test]
fn branch_checkout__double_clicking_feature_in_the_branch_rail_checks_it_out_and_its_label_is_filled_after() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.double_click(&state, Role::Button, "feature");
    assert_eq!(
        intents,
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("branch.checkout"),
            Target::Branch(String::from("feature"))
        ))]
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    assert!(
        matches!(driver.next(&mut state), app::store::Message::FeatureEnded { .. }),
        "a checkout shows no plan"
    );
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines.first().map(String::as_str), Some("git checkout feature"));
    assert_eq!(repository.git(&["branch", "--show-current"]).trim(), "feature");
    assert!(is_filled(&state, "feature"), "the label of feature is filled after");
    assert!(!is_filled(&state, "main"));
}

#[test]
fn branch_checkout_detached__the_header_says_detached_at_the_short_hash_after() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let parent = repository.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let commit = model::CommitId::parse(&parent).unwrap();
    run_to_the_end(
        &driver,
        &mut state,
        ("branch.checkout-detached", Target::Commit(commit)),
    );
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines.first(), Some(&format!("git checkout --detach {parent}")));
    let short = parent.get(..7).unwrap();
    let (_, tree) = window.draw(&state, &[]);
    assert!(
        says(&tree, &format!("detached at {short}")),
        "the header says detached at {short}"
    );
}

#[test]
fn undo_last__after_deleting_a_branch_in_gitardo_undo_creates_it_again_at_the_same_commit() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let commit = repository.git(&["rev-parse", "feature"]);
    run_to_the_end(
        &driver,
        &mut state,
        ("branch.delete", Target::Branch(String::from("feature"))),
    );
    assert_eq!(
        repository.git(&["branch", "--list", "feature"]),
        "",
        "the branch is deleted"
    );
    run_to_the_end(&driver, &mut state, ("undo.last", Target::Head));
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert!(
        lines.contains(&format!("git branch feature {}", commit.trim())),
        "{lines:?}"
    );
    assert_eq!(repository.git(&["rev-parse", "feature"]), commit, "at the same commit");
}

#[test]
fn branch_delete__after_another_commit_undo_takes_back_that_commit_instead() {
    let (mut repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run_to_the_end(
        &driver,
        &mut state,
        ("branch.delete", Target::Branch(String::from("feature"))),
    );
    let before = repository.git(&["rev-parse", "HEAD"]);
    let _ = repository.commit("Show the tags", &[("tags.txt", "v1.0\n")]);
    let _effects = super::support::run(&mut state, app::store::Message::Intent(Intent::Refresh));
    run_to_the_end(&driver, &mut state, ("undo.last", Target::Head));
    assert_eq!(
        repository.git(&["rev-parse", "HEAD"]),
        before,
        "the commit is taken back"
    );
    assert_eq!(
        repository.git(&["branch", "--list", "feature"]),
        "",
        "the branch stays deleted"
    );
}
