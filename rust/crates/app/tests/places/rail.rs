//! The branch rail drawn from the store, with repositories made for each test: the lists that stay open, the filter
//! box, the line of the branches shown, the favorites of each repository, the history a branch brings out under the
//! pointer, and its pictures.

use std::path::Path;
use std::time::Duration;

use app::persisted::rail::favorites_of;
use app::settings::Settings;
use app::store::{Message, State};
use model::CommitId;
use model::filter::BranchesShown;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::graph::{GraphIntent, HoverTarget, HoverWay};
use views::{FilterIntent, Intent, RailIntent, RailList};

use super::pointer::{apply, bounds, middle, moved};
use super::support::{Window, has, run};
use super::timeline::{showing_with, written};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// Hands an intent to the store, running the jobs it starts.
fn ask(state: &mut State, intent: Intent) -> Vec<app::store::Effect> {
    run(state, Message::Intent(intent))
}

/// The timeline narrowed to some references, by their full names.
fn narrow(state: &mut State, names: &[&str]) {
    let shown = BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect());
    let _effects = ask(state, Intent::Filter(FilterIntent::Show(Some(shown))));
}

/// The rectangle of an entry of the branch rail by its name, when the rail lists it.
fn rail_entry(tree: &TreeUpdate, name: &str) -> Option<Rect> {
    let rail = bounds(tree, Role::Navigation, "Branches");
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds())
        .map(|found| {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a window is far smaller than an f32 counts"
            )]
            Rect::new(
                found.x0 as f32,
                found.y0 as f32,
                (found.x1 - found.x0) as f32,
                (found.y1 - found.y0) as f32,
            )
        })
        .find(|found| found.x >= rail.x && found.right() <= rail.right() + 0.5)
}

/// The commit a branch of the repository shown is at.
fn tip_of(state: &State, name: &str) -> CommitId {
    let read = &state.shown_read().unwrap().state;
    read.branches
        .iter()
        .find(|branch| branch.name == name)
        .unwrap()
        .commit
        .clone()
}

/// A key pressed, with no modifier.
fn press(key: Key) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(key, Modifiers::NONE))
}

/// Two repositories in one workspace, the first shown.
fn two_repositories() -> (Repository, Repository, State) {
    let (first, _remote) = repository_with_history();
    let mut second = Repository::new();
    let _ = second.commit("Start the site", &[("index.html", "site\n")]);
    second.branch("feature");
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let mut state = super::window();
    let _effects = run(&mut state, Message::Started { folders });
    (first, second, state)
}

/// Shows another repository of the workspace.
fn select(state: &mut State, folder: &Path) {
    let _effects = ask(state, Intent::SelectRepository(folder.to_path_buf()));
}

#[test]
fn refs_tree__a_list_kept_aside_opened_stays_open_when_another_repository_is_chosen() {
    let (_first, second, mut state) = two_repositories();
    assert!(!state.rail.lists.is_open(&RailList::Tags));
    let _effects = ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Tags)));
    select(&mut state, second.path());
    assert!(state.rail.lists.is_open(&RailList::Tags));
    assert!(!state.rail.lists.is_open(&RailList::Stashes));
    assert!(state.rail.lists.is_open(&RailList::Local));
}

#[test]
fn refs_filter__escape_empties_the_box() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _effects = ask(&mut state, Intent::Rail(RailIntent::Type(String::from("ma"))));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _click = window.click(&state, Role::SearchInput, "Filter branches and tags");
    let (intents, _) = window.draw(&state, &[press(Key::Named(NamedKey::Escape))]);
    assert!(
        intents.contains(&Intent::Rail(RailIntent::Type(String::new()))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert_eq!(state.rail.typed, "");
}

#[test]
fn refs_scope_line__the_button_lists_every_branch_and_sets_follow_graph_filter_to_false() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    narrow(&mut state, &["refs/heads/main"]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(rail_entry(&tree, "feature").is_none());
    let intents = window.click(&state, Role::Button, "List every branch, the others dimmed");
    assert_eq!(intents, [Intent::Filter(FilterIntent::FollowGraphFilter(false))]);
    let effects = ask(&mut state, Intent::Filter(FilterIntent::FollowGraphFilter(false)));
    assert_eq!(
        written(&effects),
        Some(("branches.followGraphFilter", json::Value::Bool(false)))
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(rail_entry(&tree, "feature").is_some());
    assert!(rail_entry(&tree, "main").is_some());
}

#[test]
fn refs_scope_line__the_eye_on_a_dimmed_fix_adds_fix_to_the_branches_shown() {
    let (repository, _remote) = repository_with_history();
    repository.branch("fix");
    let settings = Settings {
        follows_graph_filter: false,
        ..Settings::default()
    };
    let mut state = showing_with(repository.path().to_path_buf(), settings);
    narrow(&mut state, &["refs/heads/main"]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let fix = rail_entry(&tree, "fix").unwrap();
    let _ = window.draw(&state, &[moved(middle(fix))]);
    let intents = window.click(&state, Role::Button, "Show this history too");
    apply(&mut state, intents);
    let both = BranchesShown::References(vec![String::from("refs/heads/main"), String::from("refs/heads/fix")]);
    assert_eq!(state.timeline.shown, Some(both));
}

#[test]
fn filter_list_too__off_the_eye_on_feature_adds_feature_to_the_timeline() {
    let (repository, _remote) = repository_with_history();
    let settings = Settings {
        follows_graph_filter: false,
        ..Settings::default()
    };
    let mut state = showing_with(repository.path().to_path_buf(), settings);
    narrow(&mut state, &["refs/heads/main"]);
    let main_only = state.timeline.rows.len();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let feature = rail_entry(&tree, "feature").unwrap();
    let _ = window.draw(&state, &[moved(middle(feature))]);
    let intents = window.click(&state, Role::Button, "Show this history too");
    apply(&mut state, intents);
    let both = BranchesShown::References(vec![
        String::from("refs/heads/main"),
        String::from("refs/heads/feature"),
    ]);
    assert_eq!(state.timeline.shown, Some(both));
    assert!(state.timeline.rows.len() >= main_only);
}

#[test]
fn refs_scope_line__show_all_takes_the_line_away_and_lists_everything() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let everything = state.timeline.rows.len();
    narrow(&mut state, &["refs/heads/main", "refs/heads/feature"]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Show all");
    assert_eq!(intents, [Intent::Filter(FilterIntent::Show(None))]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Show all"));
    assert!(rail_entry(&tree, "origin/main").is_some());
    assert_eq!(state.timeline.rows.len(), everything);
}

#[test]
fn refs_favorite__the_favorites_of_one_repository_are_not_those_of_another_of_the_same_workspace() {
    let (first, second, mut state) = two_repositories();
    let favorite = Intent::Rail(RailIntent::Favorite {
        reference: String::from("refs/heads/feature"),
        is_favorite: true,
    });
    let _effects = ask(&mut state, favorite);
    assert_eq!(
        favorites_of(&state.persisted.favorites, first.path()),
        ["refs/heads/feature"]
    );
    select(&mut state, second.path());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "FAVORITES"));
    assert_eq!(
        favorites_of(&state.persisted.favorites, second.path()),
        [] as [String; 0]
    );
    select(&mut state, first.path());
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "FAVORITES"));
}

#[test]
fn graph_hover_history__resting_on_a_branch_in_the_branch_rail_brings_out_its_history() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let feature = tip_of(&state, "feature");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let row = rail_entry(&tree, "feature").unwrap();
    let mut intents = window.draw(&state, &[moved(middle(row))]).0;
    window.now += Duration::from_millis(400);
    intents.extend(window.draw(&state, &[]).0);
    let target = HoverTarget {
        commit: feature.clone(),
        way: HoverWay::Reaches,
    };
    assert!(
        intents.contains(&Intent::Graph(GraphIntent::Hover(Some(target)))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    let (cells, _) = state.timeline.trace.as_ref().unwrap();
    let is_in = |commit: &CommitId| cells[state.timeline.layout.shown_of[commit]].is_in;
    assert!(is_in(&feature));
    assert!(!is_in(&tip_of(&state, "main")));
}

/// A repository for the picture of the rail: `main`, `feature` and `fix` checked out one after the other, `feature`
/// pushed to `origin`, a second remote `upstream`, and two tags.
fn repository_for_the_rail() -> (Repository, testkit::Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.tag("first");
    let _ = repository.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    repository.tag("v1.0");
    let remote = testkit::Remote::new(&repository);
    repository.branch("feature");
    repository.branch("fix");
    repository.checkout("feature");
    repository.checkout("fix");
    repository.checkout("main");
    let _ = repository.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "feature"]);
    let path = remote.path().to_str().unwrap().to_owned();
    let _ = repository.git(&["remote", "add", "upstream", &path]);
    let _ = repository.git(&["fetch", "--quiet", "upstream"]);
    (repository, remote)
}

#[test]
fn the_rail_with_favorites_recent_two_remotes_a_list_kept_aside_open_and_a_filter_typed_is_drawn() {
    let (repository, _remote) = repository_for_the_rail();
    let mut state = showing(repository.path().to_path_buf());
    let favorite = Intent::Rail(RailIntent::Favorite {
        reference: String::from("refs/heads/feature"),
        is_favorite: true,
    });
    let _effects = ask(&mut state, favorite);
    let _open = ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Tags)));
    let mut window = Window::new(ThemeName::Dark, (1000, 800));
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("rail-dark");
    let _typed = ask(&mut state, Intent::Rail(RailIntent::Type(String::from("f"))));
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("rail-typed-dark");
}
