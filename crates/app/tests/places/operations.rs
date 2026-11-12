//! Operations run from the window: asked from the header, their questions answered in the dialogs of the window, run
//! on the thread of the operations, and what they did in the Command log and said over the timeline.

use app::store::Message;
use app::store::feature::Ended;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;

use super::support::{OperationsDriver, Window, find, says, typed};
use super::{repository_with_history, showing};

/// The size of the window of the pictures.
const WIDE: (u32, u32) = (1000, 520);

/// Waits for the operation to ask, show its plan or end, and says which.
fn next_kind(driver: &OperationsDriver, state: &mut app::store::State) -> &'static str {
    match driver.next(state) {
        Message::FeatureAsks(_) => "asks",
        Message::FeatureConfirms(_) => "confirms",
        Message::FeatureEnded { .. } => "ended",
        _ => "other",
    }
}

#[test]
fn branch_create__branch_in_the_more_menu_asks_the_name_and_whether_to_check_it_out_then_creates_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for intent in window.click(&state, Role::Button, "More") {
        driver.intent(&mut state, intent);
    }
    for intent in window.click(&state, Role::MenuItem, "Branch") {
        driver.intent(&mut state, intent);
    }
    assert_eq!(next_kind(&driver, &mut state), "asks");
    let (_, tree) = window.draw(&state, &[]);
    let _dialog = find(&tree, Role::Dialog, "Create branch");
    let (intents, _) = window.draw(&state, &[typed("topic")]);
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    for intent in window.click(&state, Role::Button, "Create branch") {
        driver.intent(&mut state, intent);
    }
    assert_eq!(next_kind(&driver, &mut state), "asks");
    for intent in window.click(&state, Role::RadioButton, "Create only") {
        driver.intent(&mut state, intent);
    }
    window.assert_snapshot("operation-question-dark");
    for intent in window.click(&state, Role::Button, "Create branch `topic`") {
        driver.intent(&mut state, intent);
    }
    assert_eq!(next_kind(&driver, &mut state), "ended");
    let branches = repository.git(&["branch", "--list", "topic"]);
    assert!(branches.contains("topic"), "the branch is made: {branches}");
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines, ["git branch topic HEAD"]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Create branch topic at HEAD: done."));
    assert!(!state.is_operation_running);
}

#[test]
fn preview_confirm__cancel_runs_nothing_and_logs_nothing() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let head_before = repository.git(&["rev-parse", "HEAD"]);
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(
            model::contract::FeatureId("undo.last"),
            model::contract::Target::Head,
        )),
    );
    assert_eq!(next_kind(&driver, &mut state), "confirms");
    let (_, tree) = window.draw(&state, &[]);
    let _dialog = find(&tree, Role::Dialog, "Undo the commit \"Show the branch rail\"");
    window.assert_snapshot("operation-plan-dark");
    for intent in window.click(&state, Role::Button, "Cancel") {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureEnded { ended, log } = driver.next(&mut state) else {
        panic!("the operation ends");
    };
    assert_eq!(ended, Ended::BackedOut);
    assert!(
        log.is_empty() && state.log.entries().next().is_none(),
        "nothing is logged"
    );
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), head_before, "nothing ran");
}

#[test]
fn preview_confirm__the_steps_shown_are_byte_for_byte_the_lines_the_log_then_has() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(
            model::contract::FeatureId("undo.last"),
            model::contract::Target::Head,
        )),
    );
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("Undo shows its plan first");
    };
    let shown = views::feature::plan_lines(plan.as_ref());
    for intent in window.click(&state, Role::Button, "Run") {
        driver.intent(&mut state, intent);
    }
    assert_eq!(next_kind(&driver, &mut state), "ended");
    let logged: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(logged, shown);
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s"]).trim(),
        "Read the settings at startup",
        "the commit is taken back"
    );
}
