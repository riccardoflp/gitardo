//! Operations run from the window: asked from the header, their questions answered in the dialogs of the window, run
//! on the thread of the operations, and what they did in the Command log and said over the timeline.

use app::store::Message;
use app::store::feature::Ended;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;

use super::pointer::{middle, rect};
use super::support::{OperationsDriver, Window, find, says, says_in, typed};
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
    // The top of the branch rail has a button of the same name: this one is the dialog's.
    let (_, tree) = window.draw(&state, &[]);
    let dialog = rect(find(&tree, Role::Dialog, "Create branch").bounds().unwrap());
    let accept = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Create branch"))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|button| button.x >= dialog.x && button.y >= dialog.y && button.bottom() <= dialog.bottom())
        .unwrap();
    for intent in window.click_at(&state, middle(accept), 1) {
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
    let lines: Vec<String> = state
        .log
        .entries()
        .filter(|entry| super::support::is_ran(entry))
        .map(|entry| entry.line.clone())
        .collect();
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
fn preview_confirm__with_command_preview_off_a_branch_is_made_at_once_and_a_hard_reset_shows_its_command_and_warning() {
    use model::contract::{Answer, AnswerKey, Answers, CommandPreview, Danger, FeatureId, Target};
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    state.settings.features.command_preview = CommandPreview::Off;
    let driver = OperationsDriver::new();
    let branch = Answers::new()
        .with(AnswerKey("name"), Answer::Text(String::from("topic")))
        .with(AnswerKey("mode"), Answer::Choice(String::from("only")));
    let created = FeatureIntent::RunWith(FeatureId("branch.create"), Target::Head, branch);
    driver.intent(&mut state, Intent::Feature(created));
    assert_eq!(next_kind(&driver, &mut state), "ended", "no dialog before it runs");
    assert!(repository.git(&["branch", "--list", "topic"]).contains("topic"));
    let head = repository.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let hard = Answers::new().with(features::reset::MODE, Answer::Choice(String::from("hard")));
    let commit = Target::Commit(model::CommitId::parse(&head).unwrap());
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId("reset.to-commit"), commit, hard)),
    );
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("a hard reset is shown first");
    };
    assert_eq!(views::feature::plan_lines(&plan), [format!("git reset --hard {head}")]);
    assert!(matches!(plan.danger, Danger::Destructive(_)), "with its warning");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let said: Vec<String> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect();
    assert!(
        says_in(
            &tree,
            "A hard reset throws away all uncommitted changes to tracked files."
        ),
        "{said:?}"
    );
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
    let logged: Vec<String> = state
        .log
        .entries()
        .filter(|entry| super::support::is_ran(entry))
        .map(|entry| entry.line.clone())
        .collect();
    assert_eq!(logged, shown);
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s"]).trim(),
        "Read the settings at startup",
        "the commit is taken back"
    );
}
