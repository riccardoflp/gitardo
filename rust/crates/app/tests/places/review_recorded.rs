//! A file git resolved as before in the review of the conflicts (`review.recorded`), on a repository in which git
//! recorded how `m.txt` was resolved and a rebase of `topic` onto `main` then stopped on `m.txt` and `n.txt`: its card
//! under **Resolved as before**, accepted and staged by **Write**, or forgotten.

use app::store::State;
use model::review::{KeptChanges, KeptRow};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::review::{Filter, ReviewIntent};
use views::text::rerere as words;

use super::conflicts::to_the_end;
use super::rerere::recorded;
use super::review::{cards, drawn, review, reviewed};
use super::support::{OperationsDriver, Window, says, says_in};

/// A repository of [`recorded`] in which `topic` is rebased onto `main`, which stops on `m.txt` and `n.txt`; git wrote
/// in `m.txt` the resolution it recorded, and did not stage it.
fn rebased() -> Repository {
    let repository = recorded();
    repository.checkout("topic");
    let output = repository.command("git").args(["rebase", "main"]).output().unwrap();
    assert!(!output.status.success(), "the rebase stops on conflicts");
    let m = std::fs::read_to_string(repository.path().join("m.txt")).unwrap();
    assert_eq!(m, "resolved m\n", "git wrote the resolution it recorded");
    repository
}

/// The lines git wrote in place of the markers, and the marked lines it took the place of, as the card shows them.
fn rows_of(state: &State) -> Vec<KeptRow> {
    let page = app::store::review::page_state(state).unwrap();
    let file = page.recorded.first().unwrap();
    assert_eq!(file.path, "m.txt");
    match &file.changes {
        KeptChanges::Rows(rows) => rows.clone(),
        other => panic!("the changes are rows, not {other:?}"),
    }
}

#[test]
fn review_recorded__after_a_rebase_the_review_lists_m_under_resolved_as_before_with_the_lines_git_wrote() {
    let repository = rebased();
    let mut state = reviewed(&repository);
    let rows = rows_of(&state);
    assert!(
        rows.contains(&KeptRow::Added(1, String::from("resolved m"))),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| matches!(row, KeptRow::Removed(line) if line.starts_with("<<<<<<< "))),
        "{rows:?}"
    );
    let tree = drawn(&state);
    assert!(says(&tree, words::RESOLVED_BY_GIT));
    assert!(says(&tree, "Resolved as before 1"));
    review(&mut state, ReviewIntent::Filter(Filter::ResolvedAsBefore));
    let page = app::store::review::page_state(&state).unwrap();
    assert!(page.is_recorded_shown(page.recorded.first().unwrap()));
    assert!(page.conflicts().all(|card| !page.is_shown(card)), "only m.txt is kept");
}

#[test]
fn review_recorded__accept_and_write_stage_m_and_the_head_said_1_file_resolved_by_git_as_before() {
    let repository = rebased();
    let mut state = reviewed(&repository);
    assert!(says_in(&drawn(&state), "; 1 file resolved by git as before."));
    review(&mut state, ReviewIntent::AcceptRecorded(String::from("m.txt"), true));
    let page = app::store::review::page_state(&state).unwrap();
    assert_eq!(page.to_write(), 1);
    let driver = OperationsDriver::new();
    to_the_end(&driver, &mut state, vec![Intent::Review(ReviewIntent::Write)]);
    let staged = repository.git(&["diff", "--cached", "--name-only"]);
    assert!(staged.lines().any(|path| path == "m.txt"), "{staged}");
    let unmerged = repository.git(&["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(unmerged.trim(), "n.txt");
}

#[test]
fn review_recorded__forget_it_asks_then_m_has_its_markers_again_and_its_conflict_is_a_card() {
    let repository = rebased();
    let mut state = reviewed(&repository);
    let driver = OperationsDriver::new();
    let intents = Window::new(ThemeName::Dark, (1400, 1000)).click(&state, Role::Button, words::FORGET_IT);
    assert_eq!(
        intents,
        [Intent::Review(ReviewIntent::ForgetRecorded(String::from("m.txt")))]
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    let app::store::Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("Forget it asks first");
    };
    assert_eq!(plan.title.as_str(), "Forget the resolution of m.txt");
    to_the_end(
        &driver,
        &mut state,
        vec![Intent::Feature(views::feature::FeatureIntent::Confirm(true))],
    );
    let m = std::fs::read_to_string(repository.path().join("m.txt")).unwrap();
    assert!(m.contains("<<<<<<<"), "{m}");
    super::review::read_again(&mut state);
    assert!(cards(&state).iter().any(|card| card.key.path == "m.txt"));
    let recorded = app::store::review::page_state(&state).unwrap().recorded;
    assert_eq!(recorded, &[] as &[model::review::RecordedFile]);
}
