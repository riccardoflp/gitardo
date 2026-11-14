//! Sync and the stashes from the window: the counts and the menu of **Sync** in the header, what cannot be clicked
//! while an operation runs, the stash dropped and brought back by **Undo**, and the menus of a stash.

use app::store::{Message, State};
use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::{Remote, Repository};
use views::Intent;
use views::feature::FeatureIntent;
use views::text::header as words;

use super::support::{OperationsDriver, Window, has, says};
use super::{repository_with_history, showing};

/// The size of the window of the tests.
const WIDE: (u32, u32) = (1000, 520);

/// `main` two commits ahead of `origin/main` and three behind it, as of the last fetch.
fn two_ahead_three_behind() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let base = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    for name in ["b", "c", "d"] {
        let _theirs = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "theirs\n")]);
    }
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    for name in ["e", "f"] {
        let _mine = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "mine\n")]);
    }
    (repository, remote)
}

/// Gives the operation thread the intents of a frame.
fn give(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// Runs an operation of the catalog as a click on its entry does.
fn run(driver: &OperationsDriver, state: &mut State, feature: &'static str, target: Target) {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
}

/// Waits until the operation ends, confirming the plan it shows first.
fn confirm_until_ended(driver: &OperationsDriver, state: &mut State) -> Vec<String> {
    let mut shown = Vec::new();
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(plan) => {
                shown = views::feature::plan_lines(plan.as_ref());
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureEnded { .. } => return shown,
            _ => {}
        }
    }
}

#[test]
fn graph_toolbar__with_main_two_ahead_and_three_behind_sync_shows_the_counts() {
    let (repository, _remote) = two_ahead_three_behind();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, &words::sync_counts(3, 2)));
}

#[test]
fn graph_toolbar__detached_at_a_commit_the_header_names_its_short_hash() {
    let (repository, _remote) = repository_with_history();
    let short = repository.git(&["rev-parse", "--short", "HEAD~1"]);
    let _detached = repository.git(&["checkout", "--quiet", "--detach", "HEAD~1"]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &words::detached_at(short.trim())));
}

#[test]
fn graph_toolbar__while_a_fetch_runs_sync_pull_and_push_cannot_be_clicked() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let sync = words::sync_counts(0, 1);
    run(&driver, &mut state, "sync.fetch", Target::Head);
    assert!(state.is_operation_running, "the fetch runs");
    assert_eq!(window.click(&state, Role::Button, &sync), []);
    let _menu = window.click(&state, Role::Button, words::SYNC_MENU);
    assert_eq!(window.click(&state, Role::MenuItem, words::PULL), []);
    assert_eq!(
        window.click(&state, Role::MenuItem, &words::with_count(words::PUSH, '\u{2191}', 1)),
        []
    );
    let _ended = confirm_until_ended(&driver, &mut state);
    assert!(!state.is_operation_running);
    let (_, tree) = window.draw(&state, &[]);
    if !has(&tree, Role::MenuItem, words::PULL) {
        let _menu = window.click(&state, Role::Button, words::SYNC_MENU);
    }
    let pull = window.click(&state, Role::MenuItem, words::PULL);
    assert_eq!(
        pull,
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("sync.pull"),
            Target::Head
        ))],
        "once the fetch ended, **Pull** runs"
    );
}

#[test]
fn sync_pull_with__a_way_chosen_in_the_menu_of_sync_pulls_that_way_without_asking() {
    let (repository, _remote) = two_ahead_three_behind();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _menu = window.click(&state, Role::Button, words::SYNC_MENU);
    let chosen = window.click(&state, Role::MenuItem, words::PULL_REBASE);
    give(&driver, &mut state, chosen);
    let _ended = confirm_until_ended(&driver, &mut state);
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines, ["git pull --rebase"]);
    let subjects = repository.git(&["log", "--format=%s", "-6"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        ["Add f", "Add e", "Add d", "Add c", "Add b", "Add a"]
    );
}

#[test]
fn stash_drop__undo_right_after_brings_it_back() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    run(&driver, &mut state, "stash.drop", Target::Stash(0));
    let _dropped = confirm_until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["stash", "list"]), "", "the stash is dropped");
    let _menu = window.click(&state, Role::Button, "More");
    let chosen = window.click(&state, Role::MenuItem, words::UNDO);
    give(&driver, &mut state, chosen);
    let _restored = confirm_until_ended(&driver, &mut state);
    assert!(repository.git(&["stash", "list"]).contains("half the tags"));
}

#[test]
fn stash_undo_drop__undo_shows_git_stash_store_then_lists_the_stash_again_with_its_message_and_files() {
    let (repository, _remote) = repository_with_history();
    let commit = repository.git(&["rev-parse", "stash@{0}"]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "stash.drop", Target::Stash(0));
    let _dropped = confirm_until_ended(&driver, &mut state);
    run(&driver, &mut state, "undo.last", Target::Head);
    let shown = confirm_until_ended(&driver, &mut state);
    let store = shown.iter().find(|line| line.starts_with("git stash store")).cloned();
    assert_eq!(
        store,
        Some(format!(
            "git stash store -m \"On main: half the tags\" {}",
            commit.trim()
        ))
    );
    let read = state.shown_read().unwrap();
    let messages: Vec<&str> = read.state.stashes.iter().map(|stash| stash.message.as_str()).collect();
    assert_eq!(messages, ["On main: half the tags"]);
    assert_eq!(
        repository.git(&["stash", "show", "--name-only", "stash@{0}"]).trim(),
        "rail.txt"
    );
}

#[test]
fn undo_last__after_dropping_a_stash_undo_puts_it_back_with_its_message() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    run(&driver, &mut state, "stash.drop", Target::Stash(0));
    let _dropped = confirm_until_ended(&driver, &mut state);
    run(&driver, &mut state, "undo.last", Target::Head);
    let _restored = confirm_until_ended(&driver, &mut state);
    assert_eq!(
        repository.git(&["stash", "list", "--format=%gs"]).trim(),
        "On main: half the tags"
    );
}
