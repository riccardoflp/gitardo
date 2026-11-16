//! The tabs of the repositories open (`repo.tabs`, `repo.switch-tab`, `repo.keep-open`), over three clones made for each
//! test: what the island draws, what a click and the keys ask, and what each repository keeps while another is shown.

use std::path::PathBuf;

use app::persisted::same_folder;
use app::store::{Message, State, ViewData, reduce};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, Point};
use views::graph::{GraphIntent, Selected};
use views::page::Page;
use views::search::SearchIntent;
use views::{Content, Intent, TimelineState};

use super::commit_page::open;
use super::inspector::{ask, row_id};
use super::pointer::{bounds, middle, moved, wheel};
use super::search::search;
use super::support::{self, Window, find};

/// The size of the window of these tests.
pub(super) const WIDE: (u32, u32) = (1000, 580);

/// A repository with a little history, cloned as `api`, `web` and `docs` in a folder of their own, and a window
/// showing the three, `api` first; the folder is kept alive by the first value.
pub(super) fn three_clones() -> ((Repository, Repository), [PathBuf; 3], State) {
    let (kept, clones) = clone_three();
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    (kept, clones, state)
}

/// A repository with a little history, cloned as `api`, `web` and `docs` in a folder of their own, which the first
/// value keeps alive.
pub(super) fn clone_three() -> ((Repository, Repository), [PathBuf; 3]) {
    let mut origin = Repository::new();
    let _ = origin.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = origin.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    let _ = origin.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    let holder = Repository::new();
    let clones = ["api", "web", "docs"].map(|name| holder.path().join("clones").join(name));
    for clone in &clones {
        let _ = origin.git(&["clone", "--quiet", ".", clone.to_str().unwrap()]);
        test_identity(&origin, clone);
    }
    ((origin, holder), clones)
}

/// Gives a clone the identity of the tests, so that the status bar names no real person.
fn test_identity(origin: &Repository, clone: &std::path::Path) {
    let clone = clone.to_str().unwrap();
    let _ = origin.git(&["-C", clone, "config", "user.name", "Test Author"]);
    let _ = origin.git(&["-C", clone, "config", "user.email", "author@example.com"]);
}

/// The tabs as the island is given them: the name of each, and whether it is the one shown.
fn tabs(state: &State) -> Vec<(String, bool)> {
    let data = ViewData::new(state, Vec::new(), 0);
    data.window_state(state)
        .tabs
        .map_or_default(|tabs| tabs.tabs.iter().map(|tab| (tab.name.clone(), tab.is_shown)).collect())
}

/// Whether the timeline shows a repository as read, and not **Loading repository…**.
fn shows_a_repository(state: &State) -> bool {
    let data = ViewData::new(state, Vec::new(), 0);
    matches!(
        data.window_state(state).content,
        Content::Timeline(TimelineState::Repository { .. })
    )
}

/// The key that shows the first tab, as its card says: `Ctrl+1`, and `Cmd+1` on macOS.
fn the_first_tab() -> InputEvent {
    let with = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    InputEvent::KeyPressed(KeyEvent::press(Key::Character('1'), with))
}

/// The key that shows the next tab, as its card says: `Ctrl+PageDown`, and `Cmd+Shift+]` on macOS.
fn the_next_tab() -> InputEvent {
    let event = if cfg!(target_os = "macos") {
        KeyEvent::press(Key::Character(']'), Modifiers::META | Modifiers::SHIFT)
    } else {
        KeyEvent::press(Key::Named(NamedKey::PageDown), Modifiers::CONTROL)
    };
    InputEvent::KeyPressed(event)
}

#[test]
fn repo_tabs__a_window_of_three_repositories_draws_three_tabs_with_the_one_shown_marked_and_plus() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(1));
    assert_eq!(
        tabs(&state),
        [
            (String::from("api"), false),
            (String::from("web"), true),
            (String::from("docs"), false)
        ]
    );
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    for name in ["api", "web", "docs"] {
        let _ = find(&tree, Role::Button, name);
    }
    let _ = find(&tree, Role::Button, views::text::repo::OPEN_ANOTHER);
}

#[test]
fn repo_tabs__a_click_ctrl_1_and_ctrl_page_down_ask_for_the_tabs_they_name() {
    let (_kept, _clones, mut state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(window.click(&state, Role::Button, "docs"), [Intent::SwitchTab(2)]);
    ask(&mut state, Intent::SwitchTab(2));
    let (intents, _) = window.draw(&state, &[the_first_tab()]);
    assert_eq!(intents, [Intent::SwitchTab(0)]);
    let (intents, _) = window.draw(&state, &[the_next_tab()]);
    assert_eq!(
        intents,
        [Intent::SwitchTab(0)],
        "from the last tab, the next is the first"
    );
}

#[test]
fn repo_tabs__a_name_longer_than_its_tab_is_cut_at_its_start() {
    let mut origin = Repository::new();
    let _ = origin.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let holder = Repository::new();
    let long = holder
        .path()
        .join("a-repository-whose-name-is-far-longer-than-any-tab-can-show");
    let _ = origin.git(&["clone", "--quiet", ".", long.to_str().unwrap()]);
    test_identity(&origin, &long);
    let mut state = super::window();
    let _ = support::run(&mut state, Message::Started { folders: vec![long] });
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let cut = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| node.value())
        .find(|text| text.starts_with('\u{2026}'))
        .unwrap();
    assert!(cut.ends_with("tab-can-show"), "{cut} keeps the end of the name");
}

#[test]
fn repo_switch_tab__showing_the_tab_shown_already_reads_nothing() {
    let (_kept, _clones, mut state) = three_clones();
    assert_eq!(reduce(&mut state, Message::Intent(Intent::SwitchTab(0))), []);
}

#[test]
fn repo_switch_tab__showing_web_then_api_again_shows_api_as_it_was_left() {
    let (_kept, clones, mut state) = three_clones();
    let commit = row_id(&state, "Read the settings");
    ask(
        &mut state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))),
    );
    ask(&mut state, Intent::SwitchTab(1));
    assert!(same_folder(state.shown_folder().unwrap(), &clones[1]));
    ask(&mut state, Intent::SwitchTab(0));
    assert!(same_folder(state.shown_folder().unwrap(), &clones[0]));
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

#[test]
fn repo_switch_tab__the_tab_shown_is_the_one_shown_when_the_window_opens_again() {
    let (_kept, clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(2));
    let mut next_time = super::window();
    next_time.persisted = state.persisted.clone();
    let _ = support::run(
        &mut next_time,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    assert!(same_folder(next_time.shown_folder().unwrap(), &clones[2]));
    assert_eq!(tabs(&next_time)[2], (String::from("docs"), true));
}

#[test]
fn repo_keep_open__a_commit_selected_in_api_and_a_search_in_web_are_both_there_when_each_is_shown_again() {
    let (_kept, _clones, mut state) = three_clones();
    let commit = row_id(&state, "Read the settings");
    ask(
        &mut state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))),
    );
    ask(&mut state, Intent::SwitchTab(1));
    assert_eq!(state.timeline.selected, None, "web has nothing selected");
    search(&mut state, SearchIntent::Type(String::from("branch rail")));
    ask(&mut state, Intent::SwitchTab(0));
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
    assert_eq!(state.search.text, "");
    ask(&mut state, Intent::SwitchTab(1));
    assert_eq!(state.search.text, "branch rail");
}

#[test]
fn repo_keep_open__the_timeline_of_api_scrolled_down_is_at_the_same_place_when_api_is_shown_again() {
    let mut origin = Repository::new();
    for number in 1..=60 {
        let _ = origin.commit(&format!("Change {number}"), &[("change.txt", &format!("{number}\n"))]);
    }
    let holder = Repository::new();
    let clones = ["api", "web"].map(|name| holder.path().join(name));
    for clone in &clones {
        let _ = origin.git(&["clone", "--quiet", ".", clone.to_str().unwrap()]);
        test_identity(&origin, clone);
    }
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let top = window.frame.to_rgba();
    let list = middle(bounds(&tree, Role::ListBox, "History"));
    let _ = wheel(&mut window, &state, list, 20.0);
    // The pointer leaves the list, so that no row is drawn as hovered.
    let _ = window.draw(&state, &[moved(Point::new(500.0, 570.0))]);
    let scrolled = window.frame.to_rgba();
    assert!(scrolled != top, "the wheel scrolled the timeline");
    ask(&mut state, Intent::SwitchTab(1));
    let _ = window.draw(&state, &[]);
    ask(&mut state, Intent::SwitchTab(0));
    let _ = window.draw(&state, &[]);
    assert!(
        window.frame.to_rgba() == scrolled,
        "api is drawn as it was left, scrolled down"
    );
}

#[test]
fn repo_keep_open__coming_back_to_a_tab_shows_what_it_kept_at_once() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(1));
    let _effects = reduce(&mut state, Message::Intent(Intent::SwitchTab(0)));
    assert!(
        shows_a_repository(&state),
        "no Loading repository… while it is read again"
    );
    assert_eq!(state.timeline.loaded(), 3);
}

#[test]
fn repo_keep_open__the_page_of_a_commit_open_in_api_is_open_again_in_api_and_not_in_web() {
    let (_kept, _clones, mut state) = three_clones();
    let id = open(&mut state, "Read the settings");
    ask(&mut state, Intent::SwitchTab(1));
    assert_eq!(state.pages.open, [], "web has no page open");
    assert_eq!(state.page_shown(), None);
    ask(&mut state, Intent::SwitchTab(0));
    assert_eq!(state.pages.open, [Page::Commit(id)]);
    assert_eq!(
        state.page_shown(),
        Some(Page::Commit(id)),
        "with its trail, as it was shown"
    );
}

#[test]
fn repo_keep_open__with_the_setting_off_coming_back_to_api_shows_it_as_a_first_read() {
    let (_kept, _clones, mut state) = three_clones();
    let mut settings = state.settings.clone();
    settings.keep_open = false;
    let _ = support::run(&mut state, Message::SettingsRead(Box::new(settings)));
    let commit = row_id(&state, "Read the settings");
    ask(&mut state, Intent::Graph(GraphIntent::Select(Selected::Commit(commit))));
    search(&mut state, SearchIntent::Type(String::from("settings")));
    ask(&mut state, Intent::SwitchTab(1));
    ask(&mut state, Intent::SwitchTab(0));
    assert_eq!(state.timeline.selected, None);
    assert_eq!(state.search.text, "");
    assert!(state.tabs.open.iter().all(|tab| tab.kept.is_none()));
}

#[test]
fn repo_keep_open__turning_the_setting_off_forgets_at_once_what_the_tabs_not_shown_kept() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(1));
    assert!(state.tabs.open[0].kept.is_some());
    let mut settings = state.settings.clone();
    settings.keep_open = false;
    let _ = support::run(&mut state, Message::SettingsRead(Box::new(settings)));
    assert!(state.tabs.open.iter().all(|tab| tab.kept.is_none()));
}

#[test]
fn the_island_of_the_tabs_is_drawn_in_each_theme() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::SwitchTab(1));
    for (theme, name) in [(ThemeName::Dark, "tabs-dark"), (ThemeName::Light, "tabs-light")] {
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
