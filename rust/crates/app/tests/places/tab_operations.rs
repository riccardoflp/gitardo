//! The operation running while tabs are switched (`repo.switch-tab`, `repo.tabs`), over three clones made for each
//! test: the other tabs can be shown and searched, the tab of the operation shows a spinner, its questions are asked in
//! it, another operation is refused, and a file of another tab chosen for its history shows that tab.

use app::persisted::same_folder;
use app::store::{Message, State, ViewData};
use model::contract::{FeatureId, Target};
use model::repo::ReadCause;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;
use views::search::SearchIntent;

use super::pointer::{bounds, middle, wheel};
use super::search::search;
use super::support::{OperationsDriver, Window, find};
use super::tabs::{WIDE, three_clones};

/// Whether the repository shown is the one of a folder.
fn shows(state: &State, folder: &std::path::Path) -> bool {
    state.shown_folder().is_some_and(|shown| same_folder(shown, folder))
}

/// Starts an operation on `HEAD` in the tab shown.
fn run(driver: &OperationsDriver, state: &mut State, feature: &'static str) {
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head)),
    );
}

/// Reduces what the operation says until it ends, backing out of a question or a plan.
fn until_ended(driver: &OperationsDriver, state: &mut State) {
    loop {
        match driver.next(state) {
            Message::FeatureEnded { .. } => return,
            Message::FeatureAsks(_) => driver.intent(state, Intent::Feature(FeatureIntent::Answer(None))),
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(false))),
            _ => {}
        }
    }
}

/// Whether the island draws the tab at a place as one where an operation runs.
fn is_running(state: &State, place: usize) -> bool {
    let data = ViewData::new(state, Vec::new(), 0);
    data.window_state(state)
        .tabs
        .is_some_and(|tabs| tabs.tabs.get(place).is_some_and(|tab| tab.is_running))
}

#[test]
fn repo_switch_tab__while_a_pull_runs_in_api_web_can_be_shown_scrolled_and_searched_and_api_shows_a_spinner() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "sync.pull");
    assert!(state.is_operation_running);
    driver.intent(&mut state, Intent::SwitchTab(1));
    assert!(shows(&state, &clones[1]), "web is shown while the pull runs");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = middle(bounds(&tree, Role::ListBox, "History"));
    for intent in wheel(&mut window, &state, list, 3.0) {
        driver.intent(&mut state, intent);
    }
    search(&mut state, SearchIntent::Type(String::from("branch rail")));
    assert_eq!(state.search.text, "branch rail");
    assert!(is_running(&state, 0) && !is_running(&state, 1));
    let (_, tree) = window.draw(&state, &[]);
    let _spinner = find(&tree, Role::ProgressIndicator, views::text::repo::OPERATION_RUNNING);
    until_ended(&driver, &mut state);
    assert!(!state.is_operation_running);
    assert!(shows(&state, &clones[1]), "web stays shown when the pull ends");
    assert!(!is_running(&state, 0));
    let api = state.tabs.open[0].kept.as_ref().unwrap();
    assert_eq!(
        api.hidden.out_of_date(),
        Some(&ReadCause::OperationEnded),
        "api is read again once shown"
    );
}

#[test]
fn repo_tabs__a_tab_whose_repository_runs_an_operation_shows_a_spinner_while_another_tab_is_shown() {
    let (_kept, _clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "branch.create");
    driver.intent(&mut state, Intent::SwitchTab(2));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let spinner = find(&tree, Role::ProgressIndicator, views::text::repo::OPERATION_RUNNING)
        .bounds()
        .unwrap();
    let api = find(&tree, Role::Button, "api").bounds().unwrap();
    assert!(
        spinner.x0 >= api.x0 && spinner.x1 <= api.x1,
        "the spinner is on the tab of api"
    );
    until_ended(&driver, &mut state);
}

#[test]
fn repo_switch_tab__a_question_the_operation_of_api_asks_while_web_is_shown_is_asked_once_api_is_shown_again() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "branch.create");
    driver.intent(&mut state, Intent::SwitchTab(1));
    assert!(shows(&state, &clones[1]));
    let said = driver.next(&mut state);
    assert!(matches!(said, Message::FeatureAsks(_)), "{said:?}");
    assert!(shows(&state, &clones[0]), "api is shown again before the question");
    assert!(app::store::feature::question_state(&state).is_some());
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Answer(None)));
    until_ended(&driver, &mut state);
}

#[test]
fn repo_switch_tab__starting_a_commit_in_web_while_a_pull_runs_in_api_is_refused_with_that_sentence() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "sync.pull");
    driver.intent(&mut state, Intent::SwitchTab(1));
    run(&driver, &mut state, "commit.create");
    let title = app::store::tabs::operation_title(&state);
    let said = state.operations.said.as_ref().unwrap();
    assert!(said.is_error);
    assert_eq!(said.text, views::text::repo::running_elsewhere(&title, "api"));
    assert!(said.text.ends_with(" is running in api. Try again once it ends."));
    let running = state.operations.running.as_ref().unwrap();
    assert_eq!(running.feature, FeatureId("sync.pull"), "the pull goes on");
    assert!(shows(&state, &clones[1]));
    until_ended(&driver, &mut state);
}

#[test]
fn repo_switch_tab__show_file_history_of_a_file_of_web_chosen_while_api_is_shown_shows_the_tab_of_web() {
    let (_kept, clones, mut state) = three_clones();
    let driver = OperationsDriver::new();
    let target = Target::Path(clones[1].join("README.md"));
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(FeatureId("file-history.file"), target)),
    );
    assert!(shows(&state, &clones[1]));
    assert_eq!(state.tabs.shown, Some(1), "the tab of web");
    until_ended(&driver, &mut state);
    assert!(
        state.search.history.is_some(),
        "the timeline lists the history of the file"
    );
    assert!(shows(&state, &clones[1]));
}
