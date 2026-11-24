//! The files git resolved as before in the window (`rerere.resolved-as-before`, `rerere.forget`), on repositories in
//! which git recorded how `m.txt` was resolved and a merge of `topic` stopped again on `m.txt` and `n.txt`: the
//! banner, the rows of the changes and their menus, the Diff page of what git wrote, and forgetting the resolution.

use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;
use views::feature::FeatureIntent;
use views::text::rerere as words;

use super::changes_menu::menu_of;
use super::conflicts::to_the_end;
use super::diff::{shown, text};
use super::rerere::{diverged, merge_topic, recorded};
use super::showing;
use super::support::{OperationsDriver, Window, says};

/// A window wide enough for the whole Changes page.
const WIDE: (u32, u32) = (1200, 640);

/// The merge of `topic` stopped again in a repository where git recorded how `m.txt` was resolved, with
/// `rerere.autoUpdate` as asked; the Changes page of it open.
fn stopped_again(stages: bool) -> (Repository, State, OperationsDriver) {
    let repository = recorded();
    let _ = repository.git(&["config", "rerere.autoUpdate", if stages { "true" } else { "false" }]);
    let _ = merge_topic(&repository);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (repository, state, driver)
}

/// The text of a file of the working tree.
fn read(repository: &Repository, path: &str) -> String {
    std::fs::read_to_string(repository.path().join(path)).unwrap()
}

/// Chooses an item of the menu of the row of `m.txt`, says yes to its plan, and gives the plan's warning and the
/// lines it logged.
fn choose_on_m(state: &mut State, driver: &OperationsDriver, item: &str) -> (Option<String>, Vec<String>) {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let items = menu_of(&mut window, state, "m.txt");
    assert!(items.iter().any(|name| name == item), "{items:?}");
    for intent in window.click(state, Role::MenuItem, item) {
        driver.intent(state, intent);
    }
    let mut warning = None;
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(plan) => {
                if let model::contract::Danger::Destructive(said) = &plan.danger {
                    warning = Some(said.as_str().to_owned());
                }
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureEnded { log, .. } => {
                return (
                    warning,
                    log.into_iter()
                        .filter(super::support::is_ran)
                        .map(|entry| entry.line)
                        .collect(),
                );
            }
            _ => {}
        }
    }
}

#[test]
fn rerere_resolved_as_before__the_banner_says_two_files_have_conflicts_and_git_resolved_one_as_before() {
    let (_repository, state, _driver) = stopped_again(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(
        &tree,
        "Merge in progress \u{b7} 2 files have conflicts \u{b7} git resolved 1 as before"
    ));
}

#[test]
fn rerere_resolved_as_before__the_row_of_m_says_resolved_as_before_and_show_what_git_wrote_opens_the_markers() {
    let (_repository, mut state, driver) = stopped_again(false);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, words::RESOLVED_AS_BEFORE));
    let items = menu_of(&mut window, &state, "m.txt");
    assert_eq!(
        items,
        [
            words::SHOW_WHAT_GIT_WROTE,
            views::text::changes::MARK_AS_RESOLVED,
            words::FORGET
        ]
    );
    assert_eq!(
        menu_of(&mut Window::new(ThemeName::Dark, WIDE), &state, "n.txt"),
        [
            views::text::changes::MARK_AS_RESOLVED,
            views::text::changes::TAKE_INCOMING,
            views::text::changes::TAKE_CURRENT
        ]
    );
    for intent in window.click(&state, Role::MenuItem, words::SHOW_WHAT_GIT_WROTE) {
        driver.intent(&mut state, intent);
    }
    let page = shown(&state);
    assert_eq!(page.title, "m.txt (As git marked it \u{2194} Working tree)");
    let diff = text(page);
    assert_eq!(diff.left, "<<<<<<< main\nmain m\n=======\ntopic m\n>>>>>>> topic\n");
    assert_eq!(diff.right, "resolved m\n");
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, words::AS_GIT_MARKED_IT));
}

#[test]
fn rerere_resolved_as_before__mark_as_resolved_on_it_runs_git_add_m() {
    let (repository, mut state, driver) = stopped_again(false);
    let (_, log) = choose_on_m(&mut state, &driver, views::text::changes::MARK_AS_RESOLVED);
    assert_eq!(log, ["git add -- m.txt"]);
    let unmerged = repository.git(&["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(unmerged.trim(), "n.txt");
}

#[test]
fn rerere_resolved_as_before__with_rerere_off_nothing_of_this_is_drawn() {
    let repository = diverged(false);
    let _ = merge_topic(&repository);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Merge in progress \u{b7} 2 files have conflicts"));
    assert!(!says(&tree, words::RESOLVED_AS_BEFORE));
    let items = menu_of(&mut window, &state, "m.txt");
    assert!(!items.iter().any(|item| item == words::FORGET), "{items:?}");
}

#[test]
fn rerere_forget__on_m_it_asks_with_the_warning_then_forgets_and_m_has_its_markers_after() {
    let (repository, mut state, driver) = stopped_again(false);
    let (warning, log) = choose_on_m(&mut state, &driver, words::FORGET);
    assert_eq!(warning, Some(features::rerere::forget_warning("m.txt")));
    assert_eq!(log, ["git rerere forget m.txt", "git checkout --merge -- m.txt"]);
    assert!(read(&repository, "m.txt").contains("<<<<<<<"));
}

#[test]
fn rerere_forget__on_m_that_git_staged_the_same_two_commands_leave_it_in_conflict_not_staged() {
    let (repository, mut state, driver) = stopped_again(true);
    assert!(state.shown_read().unwrap().state.resolved_as_before[0].is_staged);
    let (_, log) = choose_on_m(&mut state, &driver, words::FORGET);
    assert_eq!(log, ["git rerere forget m.txt", "git checkout --merge -- m.txt"]);
    let unmerged = repository.git(&["diff", "--name-only", "--diff-filter=U"]);
    assert!(unmerged.lines().any(|path| path == "m.txt"), "{unmerged}");
    assert!(read(&repository, "m.txt").contains("<<<<<<<"));
}

#[test]
fn rerere_forget__resolving_m_otherwise_and_running_the_same_merge_again_writes_the_new_resolution() {
    let (repository, mut state, driver) = stopped_again(false);
    let _ = choose_on_m(&mut state, &driver, words::FORGET);
    std::fs::write(repository.path().join("m.txt"), "other m\n").unwrap();
    let _ = repository.git(&["rerere"]);
    let _ = repository.git(&["merge", "--abort"]);
    let _ = merge_topic(&repository);
    assert_eq!(read(&repository, "m.txt"), "other m\n");
}

#[test]
fn rerere_forget__with_command_preview_off_it_still_asks() {
    let (_repository, mut state, driver) = stopped_again(false);
    state.settings.features.command_preview = model::contract::CommandPreview::Off;
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = menu_of(&mut window, &state, "m.txt");
    for intent in window.click(&state, Role::MenuItem, words::FORGET) {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("Forget this resolution asks first");
    };
    assert_eq!(plan.title.as_str(), "Forget the resolution of m.txt");
    to_the_end(
        &driver,
        &mut state,
        vec![Intent::Feature(FeatureIntent::Confirm(false))],
    );
}
