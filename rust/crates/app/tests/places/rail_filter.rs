//! The filter of the branch rail drawn from the store (`refs.list-filter`, `refs.listed`, `refs.scope-line`): its
//! button and its list, the line of what the rail lists, the choice kept for each repository apart from the
//! timeline's, and its picture.

use app::persisted::Persisted;
use app::settings::Settings;
use app::store::{Message, State};
use model::filter::{BranchesShown, Kinds, Preset, ReferenceKind, preset};
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::ThemeName;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::{FilterIntent, Intent, RailIntent};

use super::pointer::apply;
use super::rail::{rail_entry, select, two_repositories};
use super::support::{Window, find, has, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);
/// What the button of the filter of the rail is named before what it says.
const BUTTON: &str = "Branches listed in the branch rail: ";

/// Hands an intent to the store, running the jobs it starts.
fn ask(state: &mut State, intent: Intent) -> Vec<app::store::Effect> {
    run(state, Message::Intent(intent))
}

/// References by their full names, ticked one by one.
fn ticked(names: &[&str]) -> BranchesShown {
    BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
}

/// The button of the filter of the rail when it says some words.
fn button(what: &str) -> String {
    format!("{BUTTON}{what}")
}

/// Opens the list of the filter of the rail with a click on its button, and gives the window drawn with it open.
fn open_list(window: &mut Window, state: &mut State, what: &str) -> TreeUpdate {
    let intents = window.click(state, Role::Button, &button(what));
    assert!(
        matches!(
            intents.as_slice(),
            [Intent::Rail(RailIntent::OpenListFilter { window: Some(_) })]
        ),
        "{intents:?}"
    );
    apply(state, intents);
    let (_, _) = window.draw(state, &[]);
    window.draw(state, &[]).1
}

/// Clicks a widget, hands its intents to the store, and draws the window again.
fn click(window: &mut Window, state: &mut State, (role, name): (Role, &str)) -> TreeUpdate {
    let intents = window.click(state, role, name);
    apply(state, intents);
    window.draw(state, &[]).1
}

#[test]
fn refs_list_filter__local_in_the_filter_of_the_rail_lists_no_remote_branch_and_the_timeline_still_shows_every_branch()
{
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let rows = state.timeline.rows.len();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = open_list(&mut window, &mut state, "All branches");
    assert!(rail_entry(&tree, "origin/main").is_some());
    let tree = click(&mut window, &mut state, (Role::Button, "Local"));
    assert_eq!(state.rail.listed, preset(Preset::Local));
    assert!(rail_entry(&tree, "origin/main").is_none(), "no remote branch is listed");
    assert!(rail_entry(&tree, "feature").is_some());
    assert_eq!(state.timeline.shown, None, "the timeline still shows every branch");
    assert_eq!(state.timeline.rows.len(), rows);
}

#[test]
fn refs_list_filter__ticking_main_and_feature_lists_only_those_two_among_the_local_branches_and_the_timeline_is_not_narrowed()
 {
    let (repository, _remote) = repository_with_history();
    repository.branch("fix");
    let mut state = showing(repository.path().to_path_buf());
    super::fold_changes(&mut state);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _tree = open_list(&mut window, &mut state, "All branches");
    let _tree = click(&mut window, &mut state, (Role::CheckBox, "main"));
    let tree = click(&mut window, &mut state, (Role::CheckBox, "feature"));
    assert_eq!(
        state.rail.listed,
        Some(ticked(&["refs/heads/main", "refs/heads/feature"]))
    );
    assert!(rail_entry(&tree, "main").is_some());
    assert!(rail_entry(&tree, "feature").is_some());
    assert!(rail_entry(&tree, "fix").is_none());
    assert_eq!(state.timeline.shown, None, "the timeline is not narrowed");
}

#[test]
fn refs_list_filter__narrowing_the_timeline_to_main_from_its_branch_filter_leaves_the_filter_of_the_rail_as_it_was() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let local = BranchesShown::Kinds(Kinds::only(ReferenceKind::Local));
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(Some(local.clone()))));
    let _effects = ask(
        &mut state,
        Intent::Filter(FilterIntent::Show(Some(ticked(&["refs/heads/main"])))),
    );
    assert_eq!(state.rail.listed, Some(local), "the filter of the rail is as it was");
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(None)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    for name in ["main", "feature", "origin/main"] {
        assert!(rail_entry(&tree, name).is_some(), "{name} is listed");
    }
    assert_eq!(
        state.timeline.shown,
        Some(ticked(&["refs/heads/main"])),
        "the timeline keeps what it shows"
    );
}

#[test]
fn refs_list_filter__the_button_is_pressed_in_while_the_rail_lists_less_than_everything_and_its_tooltip_says_what() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        find(&tree, Role::Button, &button("All branches")).toggled(),
        Some(Toggled::False)
    );
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(preset(Preset::Local))));
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        find(&tree, Role::Button, &button("Local branches")).toggled(),
        Some(Toggled::True)
    );
}

#[test]
fn refs_list_filter__filter_the_branch_rail_in_quick_search_opens_its_list() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let choose = QuickChoice::Command(QuickCommand::FilterBranchRail);
    let _effects = ask(&mut state, Intent::Search(SearchIntent::ChooseQuick(choose)));
    assert!(state.rail.list_filter.is_some());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Remotes"), "the list is open");
    assert!(
        state
            .rail
            .list_filter
            .as_ref()
            .is_some_and(|open| open.window.is_some())
    );
}

#[test]
fn refs_scope_line__with_the_filter_of_the_rail_on_main_and_feature_the_local_list_has_only_those_two_under_a_line() {
    let (repository, _remote) = repository_with_history();
    repository.branch("fix");
    let mut state = showing(repository.path().to_path_buf());
    let both = ticked(&["refs/heads/main", "refs/heads/feature"]);
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(Some(both))));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(rail_entry(&tree, "fix").is_none());
    assert!(rail_entry(&tree, "origin/main").is_none());
    let says_main_and_one = tree
        .nodes
        .iter()
        .any(|(_, node)| node.label() == Some("main +1") || node.value() == Some("main +1"));
    assert!(says_main_and_one, "the line says main +1");
    assert!(has(&tree, Role::Button, "Show all"));
}

#[test]
fn refs_scope_line__show_all_on_the_line_takes_it_away_and_lists_everything_and_the_timeline_shows_what_it_showed() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _effects = ask(
        &mut state,
        Intent::Filter(FilterIntent::Show(Some(ticked(&["refs/heads/feature"])))),
    );
    let rows = state.timeline.rows.len();
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(preset(Preset::Local))));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Show all");
    assert_eq!(intents, [Intent::Rail(RailIntent::List(None))]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Show all"));
    assert!(rail_entry(&tree, "origin/main").is_some());
    assert_eq!(state.timeline.shown, Some(ticked(&["refs/heads/feature"])));
    assert_eq!(state.timeline.rows.len(), rows);
}

#[test]
fn refs_listed__the_choice_of_api_is_in_use_when_the_window_opens_again_and_web_keeps_its_own() {
    let (api, web, mut state) = two_repositories();
    let local = preset(Preset::Local);
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(local.clone())));
    select(&mut state, web.path());
    assert_eq!(state.rail.listed, None, "web lists everything");
    let main = ticked(&["refs/heads/main"]);
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(Some(main.clone()))));
    assert_eq!(state.timeline.shown, None, "the timeline's choice is apart");
    let persisted = Persisted::from_text(&state.persisted.to_text());
    let mut opened = State::new(persisted, Settings::default(), None);
    let folders = vec![api.path().to_path_buf(), web.path().to_path_buf()];
    let _effects = run(&mut opened, Message::Started { folders });
    select(&mut opened, api.path());
    assert_eq!(opened.rail.listed, local);
    assert_eq!(opened.timeline.shown, None);
    select(&mut opened, web.path());
    assert_eq!(opened.rail.listed, Some(main));
}

#[test]
fn the_rail_filtered_to_local_branches_is_drawn_with_its_line_and_its_list_open() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _effects = ask(&mut state, Intent::Rail(RailIntent::List(preset(Preset::Local))));
    let mut window = Window::new(ThemeName::Dark, (1000, 640));
    let _tree = open_list(&mut window, &mut state, "Local branches");
    window.assert_snapshot("rail-listed-dark");
}
