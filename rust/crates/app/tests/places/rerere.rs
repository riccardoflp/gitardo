//! Git's rerere from the window (`rerere.turn-on`, `rerere.turn-off`), on repositories made for each test: the entry
//! of the menu of the row of the repository, ticked while git remembers resolutions, and a conflict resolved and
//! recorded, met again once it is off.

use std::path::Path;

use app::store::{Message, State};
use model::contract::Answer;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::rerere as words;

use super::large_files::menu_of;
use super::showing;
use super::support::{OperationsDriver, Window};

/// The size of the window.
const SIZE: (u32, u32) = (1000, 600);

/// A repository whose `m.txt` and `n.txt` were changed on `main` and on `topic` since `base`, both checked in, with
/// `rerere.enabled` set as asked in its own configuration, whatever the user's says.
pub(super) fn diverged(is_on: bool) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["config", "rerere.enabled", if is_on { "true" } else { "false" }]);
    let _ = repository.commit("Add the files", &[("m.txt", "base m\n"), ("n.txt", "base n\n")]);
    repository.branch("topic");
    let _ = repository.commit("Change them on main", &[("m.txt", "main m\n"), ("n.txt", "main n\n")]);
    repository.checkout("topic");
    let _ = repository.commit(
        "Change them on topic",
        &[("m.txt", "topic m\n"), ("n.txt", "topic n\n")],
    );
    repository.checkout("main");
    repository
}

/// Merges `topic` into `main`, which stops on conflicts; whether git printed that it did.
pub(super) fn merge_topic(repository: &Repository) -> String {
    let output = repository
        .command("git")
        .args(["merge", "--no-edit", "topic"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "the merge stops on conflicts");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A repository of [`diverged`] with rerere on, in which the merge of `topic` stopped once, `m.txt` was resolved as
/// `resolved m` and recorded by `git rerere`, and the merge was aborted: git has a resolution for `m.txt` alone.
pub(super) fn recorded() -> Repository {
    let repository = diverged(true);
    let _ = merge_topic(&repository);
    std::fs::write(repository.path().join("m.txt"), "resolved m\n").unwrap();
    let _ = repository.git(&["rerere"]);
    let _ = repository.git(&["merge", "--abort"]);
    repository
}

/// The window of a repository, drawn once.
fn window_of(folder: &Path) -> (State, Window) {
    let state = showing(folder.to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = window.draw(&state, &[]);
    (state, window)
}

/// Clicks an item of the menu of the row of the repository, answers the question of the operation with these answers,
/// confirms its plan, and gives the lines it logged; none when it ended without running.
fn run_from_menu(window: &mut Window, state: &mut State, item: &str, answer: Option<&Answer>) -> Vec<String> {
    let items = menu_of(window, state, "repository");
    assert!(items.contains(&item.to_owned()), "{items:?}");
    let driver = OperationsDriver::new();
    for intent in window.click(state, Role::MenuItem, item) {
        driver.intent(state, intent);
    }
    loop {
        match driver.next(state) {
            Message::FeatureAsks(_) => driver.intent(state, Intent::Feature(FeatureIntent::Answer(answer.cloned()))),
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { log, .. } => {
                return log
                    .into_iter()
                    .filter(super::support::is_ran)
                    .map(|entry| entry.line)
                    .collect();
            }
            _ => {}
        }
    }
}

#[test]
fn rerere_turn_on__with_the_first_answer_from_the_menu_it_runs_and_the_entry_is_ticked_after() {
    let repository = diverged(false);
    let (mut state, mut window) = window_of(repository.path());
    let log = run_from_menu(
        &mut window,
        &mut state,
        words::REMEMBER,
        Some(&Answer::Choice(String::from(features::rerere::LEAVE))),
    );
    assert_eq!(log, ["git config --local rerere.enabled true"]);
    assert!(state.shown_read().unwrap().state.rerere.is_on);
    let items = menu_of(&mut window, &state, "repository");
    assert!(items.contains(&String::from(words::REMEMBERING)), "{items:?}");
    assert!(!items.contains(&String::from(words::REMEMBER)), "{items:?}");
}

#[test]
fn rerere_turn_on__backing_out_of_the_question_runs_nothing() {
    let repository = diverged(false);
    let (mut state, mut window) = window_of(repository.path());
    let log = run_from_menu(&mut window, &mut state, words::REMEMBER, None);
    assert!(log.is_empty(), "{log:?}");
    let enabled = repository.git(&["config", "rerere.enabled"]);
    assert_eq!(enabled.trim(), "false");
}

#[test]
fn rerere_turn_off__it_runs_rerere_enabled_false_and_the_next_conflict_git_resolved_before_stays_marked() {
    let repository = recorded();
    let (mut state, mut window) = window_of(repository.path());
    let log = run_from_menu(&mut window, &mut state, words::REMEMBERING, None);
    assert_eq!(log, ["git config --local rerere.enabled false"]);
    let _ = merge_topic(&repository);
    let m = std::fs::read_to_string(repository.path().join("m.txt")).unwrap();
    assert!(m.contains("<<<<<<<"), "{m}");
}

#[test]
fn rerere_turn_off__the_rr_cache_folder_is_not_deleted() {
    let repository = recorded();
    let rr_cache = repository.path().join(".git").join("rr-cache");
    assert!(std::fs::read_dir(&rr_cache).unwrap().next().is_some());
    let (mut state, mut window) = window_of(repository.path());
    let _ = run_from_menu(&mut window, &mut state, words::REMEMBERING, None);
    assert!(std::fs::read_dir(&rr_cache).unwrap().next().is_some());
}
