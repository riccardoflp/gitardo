//! The branch filter drawn from the store, with repositories made for each test: the button of the header and its
//! list, the presets, the entries that narrow the timeline to a history, the bar over the timeline, and the choice
//! kept for each repository.

use std::path::PathBuf;

use app::persisted::rail::shown_of;
use app::settings::Settings;
use app::store::{Message, State};
use model::filter::{BranchesShown, Preset, StoredValue, preset, show_only};
use model::graph::Row;
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::{FilterIntent, Intent};

use super::pointer::apply;
use super::support::{Window, find, has, run};
use super::timeline::subjects;
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);
/// What the button of the branch filter is named before what it says.
const BUTTON: &str = "Branches shown in the timeline: ";

/// Hands an intent to the store, running the jobs it starts.
fn ask(state: &mut State, intent: Intent) -> Vec<app::store::Effect> {
    run(state, Message::Intent(intent))
}

/// Shows some branches in the timeline; none is everything.
fn show(state: &mut State, shown: Option<BranchesShown>) {
    let _effects = ask(state, Intent::Filter(FilterIntent::Show(shown)));
}

/// References by their full names, ticked one by one.
fn ticked(names: &[&str]) -> BranchesShown {
    BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
}

/// The name of the button of the branch filter when it says some words.
fn button(what: &str) -> String {
    format!("{BUTTON}{what}")
}

/// Whether the button of the branch filter, saying some words, is pressed in.
fn is_pressed(tree: &TreeUpdate, what: &str) -> bool {
    find(tree, Role::Button, &button(what)).toggled() == Some(Toggled::True)
}

/// The name of the widget with the keyboard.
fn focused(tree: &TreeUpdate) -> Option<String> {
    tree.nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

/// The subjects of the commits read, from the top, without the stashes and the draft.
fn commits(state: &State) -> Vec<String> {
    state
        .timeline
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some(commit.subject.clone()),
            _ => None,
        })
        .collect()
}

/// A key pressed, with no modifier.
fn press(key: Key) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(key, Modifiers::NONE))
}

/// Opens the list of the branch filter with a click on its button, and draws it.
fn open_picker(window: &mut Window, state: &mut State, what: &str) -> TreeUpdate {
    let intents = window.click(state, Role::Button, &button(what));
    assert!(
        matches!(intents.as_slice(), [Intent::Filter(FilterIntent::OpenPicker { .. })]),
        "{intents:?}"
    );
    apply(state, intents);
    let (_, _) = window.draw(state, &[]);
    window.draw(state, &[]).1
}

#[test]
fn filter_picker__clicking_the_button_opens_the_list_with_the_keyboard_in_its_box_and_again_closes_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = open_picker(&mut window, &mut state, "All branches");
    assert_eq!(focused(&tree).as_deref(), Some("Filter branches and tags"));
    assert!(has(&tree, Role::CheckBox, "feature"));
    let intents = window.click(&state, Role::Button, &button("All branches"));
    assert_eq!(intents, [Intent::Filter(FilterIntent::ClosePicker)]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::CheckBox, "feature"));
}

#[test]
fn filter_picker__ticking_feature_from_all_shows_only_its_history_and_the_button_says_feature_pressed_in() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _tree = open_picker(&mut window, &mut state, "All branches");
    let intents = window.click(&state, Role::CheckBox, "feature");
    apply(&mut state, intents);
    assert_eq!(state.timeline.shown, Some(ticked(&["refs/heads/feature"])));
    assert_eq!(subjects(&state), ["Read the settings at startup", "Add the readme"]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_pressed(&tree, "feature"));
}

#[test]
fn filter_picker__ticking_fix_too_makes_the_button_say_feature_and_1() {
    let (repository, _remote) = repository_with_history();
    repository.branch("fix");
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, Some(ticked(&["refs/heads/feature"])));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _tree = open_picker(&mut window, &mut state, "feature");
    let intents = window.click(&state, Role::CheckBox, "fix");
    apply(&mut state, intents);
    assert_eq!(
        state.timeline.shown,
        Some(ticked(&["refs/heads/feature", "refs/heads/fix"]))
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_pressed(&tree, "feature +1"));
}

#[test]
fn filter_picker__escape_with_text_empties_the_box_and_escape_again_closes_the_list() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _tree = open_picker(&mut window, &mut state, "All branches");
    let (intents, _) = window.draw(&state, &[InputEvent::TextTyped(String::from("ma"))]);
    assert_eq!(intents, [Intent::Filter(FilterIntent::Type(String::from("ma")))]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::CheckBox, "feature"));
    let (intents, _) = window.draw(&state, &[press(Key::Named(NamedKey::Escape))]);
    assert!(
        intents.contains(&Intent::Filter(FilterIntent::Type(String::new()))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    let (intents, _) = window.draw(&state, &[press(Key::Named(NamedKey::Escape))]);
    assert!(
        intents.contains(&Intent::Filter(FilterIntent::ClosePicker)),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert!(state.rail.picker.is_none());
}

#[test]
fn filter_list_too__the_branch_filter_of_the_timeline_has_no_filter_the_list_of_branches_too() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = open_picker(&mut window, &mut state, "All branches");
    assert!(has(&tree, Role::CheckBox, "feature"), "the list is open");
    assert!(!has(&tree, Role::CheckBox, "Filter the list of branches too"));
}

#[test]
fn filter_preset__current_on_main_then_checking_out_feature_shows_the_history_of_feature() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, preset(Preset::Current));
    assert_eq!(
        commits(&state),
        ["Show the branch rail", "Read the settings at startup", "Add the readme"]
    );
    repository.checkout("feature");
    let _effects = ask(&mut state, Intent::Refresh);
    assert_eq!(state.timeline.shown, preset(Preset::Current));
    assert_eq!(commits(&state), ["Read the settings at startup", "Add the readme"]);
}

#[test]
fn filter_preset__local_while_detached_at_a_commit_no_local_branch_has_still_lists_that_commit() {
    let (mut repository, _remote) = repository_with_history();
    let _ = repository.git(&["checkout", "--quiet", "--detach"]);
    let _ = repository.commit("Try an idea", &[("idea.txt", "idea\n")]);
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, preset(Preset::Local));
    assert_eq!(commits(&state).first().map(String::as_str), Some("Try an idea"));
}

#[test]
fn filter_show_only__on_v1_0_lists_the_commits_v1_0_reaches_and_no_others() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, Some(show_only("refs/tags/v1.0")));
    assert_eq!(subjects(&state), ["Read the settings at startup", "Add the readme"]);
}

#[test]
fn filter_show_only__on_feature_while_main_is_checked_out_the_commit_of_main_is_not_listed() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let main = state.shown_read().unwrap().state.head.commit.clone().unwrap();
    show(&mut state, Some(show_only("refs/heads/feature")));
    assert!(!state.timeline.rows.iter().any(|row| row.id() == Some(&main)));
    assert_eq!(subjects(&state), ["Read the settings at startup", "Add the readme"]);
}

#[test]
fn filter_show_all__show_all_branches_on_the_bar_shows_everything_and_the_bar_goes() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let everything = subjects(&state);
    show(&mut state, Some(show_only("refs/heads/feature")));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Show all branches");
    assert_eq!(intents, [Intent::Filter(FilterIntent::Show(None))]);
    apply(&mut state, intents);
    assert_eq!(subjects(&state), everything);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Show all branches"));
}

#[test]
fn filter_show_all__the_button_of_the_branch_filter_is_no_longer_pressed_in() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, Some(show_only("refs/heads/feature")));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_pressed(&tree, "feature"));
    show(&mut state, None);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!is_pressed(&tree, "All branches"));
}

#[test]
fn filter_remember__narrowing_to_feature_choosing_another_repository_and_coming_back_shows_feature_alone() {
    let (first, _remote) = repository_with_history();
    let (second, _other) = repository_with_history();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let mut state = super::window();
    let _effects = run(&mut state, Message::Started { folders });
    show(&mut state, Some(show_only("refs/heads/feature")));
    let _effects = ask(&mut state, Intent::SelectRepository(second.path().to_path_buf()));
    assert_eq!(state.timeline.shown, None);
    let _effects = ask(&mut state, Intent::SelectRepository(first.path().to_path_buf()));
    assert_eq!(state.timeline.shown, Some(show_only("refs/heads/feature")));
    assert_eq!(subjects(&state), ["Read the settings at startup", "Add the readme"]);
}

#[test]
fn filter_remember__after_branch_d_feature_a_timeline_narrowed_to_feature_and_main_shows_main_alone_and_keeps_it() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    show(&mut state, Some(ticked(&["refs/heads/feature", "refs/heads/main"])));
    let _ = repository.git(&["branch", "-D", "feature"]);
    let _effects = ask(&mut state, Intent::Refresh);
    assert_eq!(state.timeline.shown, Some(ticked(&["refs/heads/main"])));
    let kept = shown_of(&state.persisted.branches_shown, &folder);
    assert_eq!(
        kept,
        Some(&StoredValue::References(vec![String::from("refs/heads/main")]))
    );
}

#[test]
fn filter_remember__narrowed_to_feature_alone_deleting_feature_shows_everything() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let everything = subjects(&state);
    show(&mut state, Some(show_only("refs/heads/feature")));
    let _ = repository.git(&["branch", "-D", "feature"]);
    let _effects = ask(&mut state, Intent::Refresh);
    assert_eq!(state.timeline.shown, None);
    assert_eq!(subjects(&state), everything);
    assert_eq!(shown_of(&state.persisted.branches_shown, &folder), None);
}

#[test]
fn filter_remember__the_choice_is_there_after_the_window_is_closed_and_opened_again_on_the_workspace() {
    let (repository, _remote) = repository_with_history();
    let folder: PathBuf = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    show(&mut state, Some(show_only("refs/heads/feature")));
    let mut opened = State::new(state.persisted.clone(), Settings::default(), None);
    let _effects = run(&mut opened, Message::Started { folders: vec![folder] });
    assert_eq!(opened.timeline.shown, Some(show_only("refs/heads/feature")));
    assert_eq!(subjects(&opened), ["Read the settings at startup", "Add the readme"]);
}

#[test]
fn the_timeline_narrowed_to_two_branches_is_drawn_with_its_bar_and_the_button_pressed_in() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    show(&mut state, Some(ticked(&["refs/heads/main", "refs/heads/feature"])));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_pressed(&tree, "main +1"));
    assert!(has(&tree, Role::Button, "Show all branches"));
    window.assert_snapshot("timeline-narrowed-dark");
}
