//! An operation stopped on conflicts, in the window: the banner over the timeline, the message when an operation stops,
//! the buttons of a file in conflict on the Changes page, and the resolutions Gitardo writes and takes back.

use app::settings::Settings;
use app::store::{Message, State};
use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::{CONFLICT_FILE, Repository};
use views::Intent;
use views::changes::ChangesIntent;
use views::conflicts::ConflictsIntent;
use views::feature::FeatureIntent;
use views::page::Page;

use super::showing;
use super::support::{OperationsDriver, Window, find, has, says};
use super::timeline::showing_with;

/// The size of the window.
const WIDE: (u32, u32) = (1200, 560);

/// Runs a git command that may fail, as one that stops on conflicts does.
fn git_may_fail(repository: &Repository, arguments: &[&str]) {
    let _output = repository.command("git").args(arguments).output().unwrap();
}

/// A repository where `main` and `feature` both changed `a.txt` and `b.txt` from the same base, `main` checked out.
fn two_files_changed_on_both_sides() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("a.txt", "base\n"), ("b.txt", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change them on main", &[("a.txt", "main\n"), ("b.txt", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit(
        "Change them on feature",
        &[("a.txt", "feature\n"), ("b.txt", "feature\n")],
    );
    repository.checkout("main");
    repository
}

/// The git commands the operations ran, as the Command log has them.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

/// Gives the intents to the window, saying yes to every plan shown and the default to every question, until the
/// operation they start ends.
pub(super) fn to_the_end(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureAsks(question) => {
                let answer = question.default.clone();
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(answer)));
            }
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

/// The settings with `conflicts.autoResolve` on.
fn auto_resolve() -> Settings {
    let mut settings = Settings::default();
    settings.features.conflicts_auto_resolve = true;
    settings
}

/// The Changes page of a repository, with the thread of the operations.
fn changes_page(repository: &Repository, settings: Settings) -> (State, OperationsDriver) {
    let mut state = showing_with(repository.path().to_path_buf(), settings);
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (state, driver)
}

/// The text of a file of the working tree.
fn read(repository: &Repository, name: &str) -> String {
    std::fs::read_to_string(repository.path().join(name)).unwrap()
}

#[test]
fn conflicts_banner__a_merge_stopped_on_2_files_has_continue_disabled_abort_and_no_skip() {
    let repository = two_files_changed_on_both_sides();
    git_may_fail(&repository, &["merge", "feature"]);
    let state = showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Merge in progress \u{b7} 2 files have conflicts"));
    assert!(find(&tree, Role::Button, "Continue").is_disabled());
    assert!(!find(&tree, Role::Button, "Abort").is_disabled());
    assert!(!has(&tree, Role::Button, "Skip"));
}

#[test]
fn conflicts_banner__a_stash_applied_with_a_conflict_says_1_file_has_conflicts_with_no_button() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the file", &[("a.txt", "base\n")]);
    std::fs::write(repository.path().join("a.txt"), "stashed\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet"]);
    let _ = repository.commit("Change the file", &[("a.txt", "committed\n")]);
    git_may_fail(&repository, &["stash", "apply"]);
    let state = showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "1 file has conflicts"));
    for button in ["Continue", "Skip", "Abort"] {
        assert!(!has(&tree, Role::Button, button), "{button} is drawn");
    }
}

#[test]
fn conflicts_banner__git_am_stopped_on_a_patch_is_named_patch_and_offers_skip() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the file", &[("a.txt", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change the file on main", &[("a.txt", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Change the file on feature", &[("a.txt", "feature\n")]);
    let patch = repository.git(&["format-patch", "-1", "--stdout"]);
    repository.checkout("main");
    let file = repository.path().join("..").join("change.patch");
    std::fs::write(&file, patch).unwrap();
    git_may_fail(&repository, &["am", file.to_str().unwrap()]);
    let state = showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Patch (git am) in progress"));
    assert!(has(&tree, Role::Button, "Skip"));
    assert!(has(&tree, Role::Button, "Abort"));
}

/// Opens the menu of the row of a file in conflict and chooses an item of it; gives what it asks.
fn in_menu(window: &mut Window, state: &State, (path, item): (&str, &str)) -> Vec<Intent> {
    let items = super::changes_menu::menu_of(window, state, path);
    assert!(
        items.iter().any(|name| name == item),
        "{item} is in the menu: {items:?}"
    );
    window.click(state, Role::MenuItem, item)
}

#[test]
fn conflicts_banner__staging_the_last_file_in_conflict_enables_continue() {
    let mut repository = Repository::new();
    repository.conflict();
    let (mut state, driver) = changes_page(&repository, Settings::default());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(find(&tree, Role::Button, "Continue").is_disabled());
    std::fs::write(repository.path().join(CONFLICT_FILE), "both\n").unwrap();
    let intents = in_menu(&mut window, &state, (CONFLICT_FILE, "Mark as resolved"));
    to_the_end(&driver, &mut state, intents);
    assert_eq!(logged(&state), [format!("git add -- {CONFLICT_FILE}")]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!find(&tree, Role::Button, "Continue").is_disabled());
}

#[test]
fn conflicts_stopped__a_merge_that_stops_on_2_files_says_so_with_its_title() {
    let repository = two_files_changed_on_both_sides();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let merge = Intent::Feature(FeatureIntent::Run(
        FeatureId("merge.into-current"),
        Target::Branch(String::from("feature")),
    ));
    to_the_end(&driver, &mut state, vec![merge]);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(
        &tree,
        "Merge feature into main stopped: 2 files have conflicts. Resolve them, then continue or abort the merge."
    ));
    assert!(has(&tree, Role::Button, "Show log"));
}

#[test]
fn conflicts_stopped__a_stash_pop_that_leaves_conflicts_ends_with_then_stage_the_files() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the file", &[("a.txt", "base\n")]);
    std::fs::write(repository.path().join("a.txt"), "stashed\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet"]);
    let _ = repository.commit("Change the file", &[("a.txt", "committed\n")]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let pop = Intent::Feature(FeatureIntent::Run(FeatureId("stash.pop"), Target::Stash(0)));
    to_the_end(&driver, &mut state, vec![pop]);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(
        &tree,
        "Pop stash@{0} stopped: 1 file has conflicts. Resolve them, then stage the files."
    ));
}

#[test]
fn conflicts_abort_operation__with_command_preview_off_abort_still_asks_and_takes_the_merge_back() {
    let mut repository = Repository::new();
    repository.conflict();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let intents = Window::new(ThemeName::Dark, WIDE).click(&state, Role::Button, "Abort");
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("Abort did not ask first");
    };
    assert_eq!(plan.title.as_str(), "Abort merge");
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true)));
    while !matches!(driver.next(&mut state), Message::FeatureEnded { .. }) {}
    assert_eq!(logged(&state), ["git merge --abort"]);
    assert_eq!(read(&repository, CONFLICT_FILE), "ours\n");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(!says(&tree, "Merge in progress \u{b7} 1 file has conflicts"));
}

#[test]
fn conflicts_mark_resolved__a_file_deleted_on_one_side_and_kept_is_staged_with_its_content() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the file", &[("a.txt", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change the file", &[("a.txt", "changed\n")]);
    repository.checkout("feature");
    let _ = repository.git(&["rm", "--quiet", "a.txt"]);
    let _ = repository.git(&["commit", "--quiet", "-m", "Delete the file"]);
    repository.checkout("main");
    git_may_fail(&repository, &["merge", "feature"]);
    let (mut state, driver) = changes_page(&repository, Settings::default());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = in_menu(&mut window, &state, ("a.txt", "Mark as resolved"));
    to_the_end(&driver, &mut state, intents);
    assert_eq!(repository.git(&["show", ":a.txt"]), "changed\n");
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, "Conflicts 1"), "the file is no longer in conflict");
}

#[test]
fn conflicts_take_side__undo_after_taking_the_current_side_brings_the_markers_back() {
    let mut repository = Repository::new();
    repository.conflict();
    let (mut state, driver) = changes_page(&repository, Settings::default());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = in_menu(
        &mut window,
        &state,
        (CONFLICT_FILE, "Take the current side for the whole file"),
    );
    to_the_end(&driver, &mut state, intents);
    assert_eq!(read(&repository, CONFLICT_FILE), "ours\n");
    let intents = window.click(&state, Role::Button, "Undo");
    to_the_end(&driver, &mut state, intents);
    assert_eq!(
        logged(&state).last().unwrap(),
        &format!("git checkout --merge -- {CONFLICT_FILE}")
    );
    assert!(read(&repository, CONFLICT_FILE).contains("<<<<<<<"));
}

/// Twelve numbered lines, some of them changed.
fn numbered(changed: &[(usize, &str)]) -> String {
    (1..=12)
        .map(|number| {
            let line = changed
                .iter()
                .find(|(at, _)| *at == number)
                .map_or_else(|| number.to_string(), |(_, line)| (*line).to_owned());
            line + "\n"
        })
        .collect()
}

/// A merge stopped on two files: in `a.txt` both sides changed lines next to each other, which the rules resolve; in
/// `b.txt` that conflict and another on the same line, which they do not.
fn merge_stopped_on_rules() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("a.txt", &numbered(&[])), ("b.txt", &numbered(&[]))]);
    repository.branch("feature");
    let a = numbered(&[(5, "5 on main")]);
    let b = numbered(&[(5, "5 on main"), (12, "12 on main")]);
    let _ = repository.commit("Change them on main", &[("a.txt", &a), ("b.txt", &b)]);
    repository.checkout("feature");
    let a = numbered(&[(6, "6 on feature")]);
    let b = numbered(&[(6, "6 on feature"), (12, "12 on feature")]);
    let _ = repository.commit("Change them on feature", &[("a.txt", &a), ("b.txt", &b)]);
    repository.checkout("main");
    git_may_fail(&repository, &["merge", "feature"]);
    repository
}

#[test]
fn conflicts_resolve_automatically__writes_the_two_files_then_stages_the_one_resolved_in_full() {
    let repository = merge_stopped_on_rules();
    let (mut state, driver) = changes_page(&repository, auto_resolve());
    let resolve = Intent::Feature(FeatureIntent::Run(
        FeatureId("conflicts.resolve-automatically"),
        Target::Head,
    ));
    to_the_end(&driver, &mut state, vec![resolve]);
    let lines = logged(&state);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines.first().unwrap().starts_with("write ") && lines.first().unwrap().ends_with("a.txt"));
    assert!(lines.get(1).unwrap().starts_with("write ") && lines.get(1).unwrap().ends_with("b.txt"));
    assert_eq!(lines.get(2).unwrap(), "git add -- a.txt");
    assert!(!read(&repository, "a.txt").contains("<<<<<<<"));
    assert!(read(&repository, "b.txt").contains("<<<<<<<"));
}

#[test]
fn conflicts_open_file__a_file_resolved_meanwhile_opens_nothing() {
    let mut repository = Repository::new();
    repository.conflict();
    let (mut state, driver) = changes_page(&repository, Settings::default());
    driver.intent(
        &mut state,
        Intent::Conflicts(ConflictsIntent::Open(String::from("resolved.txt"))),
    );
    assert_eq!(state.page_shown(), Some(Page::Changes));
    driver.intent(
        &mut state,
        Intent::Conflicts(ConflictsIntent::Open(CONFLICT_FILE.to_owned())),
    );
    assert!(matches!(state.page_shown(), Some(Page::Merge(_))));
}

#[test]
fn conflicts_open_file__a_file_changed_by_both_sides_opens_in_the_merge_page_labelled_current_and_incoming() {
    let mut repository = Repository::new();
    repository.conflict();
    let (mut state, driver) = changes_page(&repository, Settings::default());
    driver.intent(
        &mut state,
        Intent::Conflicts(ConflictsIntent::Open(CONFLICT_FILE.to_owned())),
    );
    assert!(matches!(state.page_shown(), Some(Page::Merge(_))));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Current \u{2014} The branch checked out"));
    assert!(says(&tree, "Incoming \u{2014} The branch being merged"));
}

#[test]
fn conflicts_banner__the_banner_is_over_the_changes_page_too() {
    let mut repository = Repository::new();
    repository.conflict();
    let (state, _driver) = changes_page(&repository, Settings::default());
    assert_eq!(state.page_shown(), Some(Page::Changes));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, "Merge in progress \u{b7} 1 file has conflicts"));
}

#[test]
fn changes_pane__a_file_in_conflict_is_under_conflicts_only_with_no_box_and_a_click_on_it_opens_the_merge_page() {
    let mut repository = Repository::new();
    repository.conflict();
    std::fs::write(repository.path().join("new.txt"), "new\n").unwrap();
    let (mut state, driver) = changes_page(&repository, Settings::default());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Conflicts 1"));
    assert!(
        !has(&tree, Role::CheckBox, CONFLICT_FILE),
        "a file in conflict has no box"
    );
    let named: Vec<f64> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(CONFLICT_FILE))
        .filter_map(|(_, node)| node.bounds().map(|bounds| bounds.y0))
        .collect();
    let changes = find(&tree, Role::CheckBox, "Changes 1").bounds().unwrap().y0;
    assert_eq!(named.len(), 1, "it is listed once, under Conflicts: {named:?}");
    assert!(named[0] < changes, "above the changes");
    let whole = render::Rect::new(0.0, 0.0, 1200.0, 560.0);
    let name = super::pointer::said_at(&tree, CONFLICT_FILE, whole);
    for intent in window.click_at(&state, super::pointer::middle(name), 1) {
        driver.intent(&mut state, intent);
    }
    assert!(matches!(state.page_shown(), Some(Page::Merge(_))));
}
