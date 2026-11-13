//! What the Merge page runs and accepts beyond a side: **Mark as resolved** run as an operation, writing and staging the
//! file or asking first, and the proposals of the rules accepted one by one or all at once.

use app::store::feature::Ended;
use app::store::{Message, State};
use model::merge_view::Choice;
use platform::accesskit::Role;
use render::ThemeName;
use ui::Modifiers;
use views::Intent;
use views::feature::FeatureIntent;
use views::merge_view::MergePageIntent;
use views::page::Page;

use super::diff::press;
use super::merge::{
    FILE, MERGE, conflicts_at, first_conflict, lines_with, on_disk, open, opened, page_intent, shown, stopped_merge,
    text, write,
};
use super::pointer::apply;
use super::showing;
use super::support::{OperationsDriver, Window, shortcut};
/// Runs **Mark as resolved** on the page shown, and gives what the operation says first.
fn mark_resolved(driver: &OperationsDriver, state: &mut State) -> Message {
    let mut window = Window::new(ThemeName::Dark, MERGE);
    for intent in window.click(state, Role::Button, "Mark as resolved") {
        driver.intent(state, intent);
    }
    driver.next(state)
}

/// The lines of the Command log.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

#[test]
fn merge_view_mark_resolved__with_every_conflict_chosen_and_unsaved_it_writes_then_stages_and_the_page_closes() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Current));
    let driver = OperationsDriver::new();
    let Message::FeatureEnded { ended, .. } = mark_resolved(&driver, &mut state) else {
        panic!("the file is written and staged without a question");
    };
    assert!(matches!(ended, Ended::Ran { .. }), "it ran: {ended:?}");
    let lines = logged(&state);
    assert!(
        lines
            .first()
            .is_some_and(|line| line.starts_with("write ") && line.ends_with("app.rs")),
        "{lines:?}"
    );
    assert!(
        lines
            .get(1)
            .is_some_and(|line| line.starts_with("git add -- ") && line.ends_with("src/app.rs")),
        "{lines:?}"
    );
    assert_eq!(on_disk(&repository), lines_with(&[(10, "line 10 on main")]));
    assert_eq!(repository.git(&["diff", "--name-only", "--diff-filter=U"]), "");
    assert_eq!(state.merge_pages.pages, [], "the page closes");
    assert!(!matches!(state.page_shown(), Some(Page::Merge(_))));
}

#[test]
fn merge_view_mark_resolved__with_1_conflict_left_it_asks_first_and_cancelling_writes_and_stages_nothing() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let before = on_disk(&repository);
    let driver = OperationsDriver::new();
    assert!(matches!(mark_resolved(&driver, &mut state), Message::FeatureAsks(_)));
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Answer(None)));
    let Message::FeatureEnded { ended, .. } = driver.next(&mut state) else {
        panic!("the operation ends");
    };
    assert_eq!(ended, Ended::BackedOut);
    assert_eq!(on_disk(&repository), before);
    assert_eq!(repository.git(&["diff", "--name-only", "--diff-filter=U"]).trim(), FILE);
    assert_eq!(shown(&state).path, FILE, "the page stays");
}

#[test]
fn merge_view_mark_resolved__with_the_file_saved_already_it_is_only_staged() {
    let repository = conflicts_at(&[10]);
    write(&repository, &lines_with(&[(10, "line 10 on feature")]));
    let mut state = opened(&repository);
    let driver = OperationsDriver::new();
    let Message::FeatureEnded { .. } = mark_resolved(&driver, &mut state) else {
        panic!("a file saved already is staged without a question");
    };
    let lines = logged(&state);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines.first().is_some_and(|line| line.starts_with("git add -- ")),
        "{lines:?}"
    );
    assert_eq!(repository.git(&["diff", "--name-only", "--diff-filter=U"]), "");
}

#[test]
fn merge_view_accept_proposals__with_auto_resolve_on_a_conflict_of_lines_next_to_each_other_has_a_proposal_of_the_rules()
 {
    let repository = stopped_merge(&[(10, "line 10 on main")], &[(11, "line 11 on feature")]);
    let mut state = showing(repository.path().to_path_buf());
    state.settings.auto_resolve = true;
    open(&mut state);
    let proposal = text(&state).proposals.first().cloned().unwrap();
    assert_eq!(proposal.region, first_conflict(&state));
    let (_, tree) = Window::new(ThemeName::Dark, MERGE).draw(&state, &[]);
    let star = super::support::find(&tree, Role::Button, "Accept the proposal");
    assert_eq!(
        star.value(),
        Some("Accept the proposal of the rules \u{2014} Changes to lines next to each other")
    );
}

#[test]
fn merge_view_accept_proposals__the_sparkle_accepts_every_proposal_at_once_and_one_ctrl_z_takes_them_back() {
    let repository = stopped_merge(
        &[(5, "line 5 on main"), (20, "line 20 on main")],
        &[(6, "line 6 on feature"), (21, "line 21 on feature")],
    );
    let mut state = showing(repository.path().to_path_buf());
    state.settings.auto_resolve = true;
    open(&mut state);
    let before = shown(&state).result.clone();
    assert_eq!(text(&state).proposals.len(), 2);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let intents = window.click(
        &state,
        Role::Button,
        "Accept what is proposed for the 2 conflicts, by the rules",
    );
    apply(&mut state, intents);
    let merged = lines_with(&[
        (5, "line 5 on main"),
        (6, "line 6 on feature"),
        (20, "line 20 on main"),
        (21, "line 21 on feature"),
    ]);
    assert_eq!(shown(&state).result, merged);
    let _ = press(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(shown(&state).result, before);
}
