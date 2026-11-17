//! The two menus at the end of the header (D68): the menu of the view, with the switches of the timeline
//! (`graph.view-menu`), and **…**, with the actions (`graph.more-menu`): where their buttons are, what a switch writes,
//! and their pictures open in each theme.

use app::settings::Settings;
use app::store::Effect;
use platform::accesskit::Role;
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::graph::{GraphIntent, TimelineSwitch};

use super::pointer::{bounds, wheel};
use super::repository_with_history;
use super::side_column::header_button;
use super::support::{Window, has, run};
use super::timeline::{showing_with, written};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);
/// A window narrow enough for the header to keep its icons and give up its words.
const NARROW: (u32, u32) = (820, 480);
/// A window tall enough for either menu to open whole.
const TALL: (u32, u32) = (1000, 720);

/// The themes, each with the end of the names of its snapshots.
const THEMES: [(ThemeName, &str); 4] = [
    (ThemeName::Dark, "dark"),
    (ThemeName::Light, "light"),
    (ThemeName::HighContrastDark, "high-contrast-dark"),
    (ThemeName::HighContrastLight, "high-contrast-light"),
];

/// Presses a named key, then draws what it did.
fn press(window: &mut Window, state: &app::store::State, key: NamedKey) -> Vec<Intent> {
    let event = InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::NONE));
    let (intents, _) = window.draw(state, &[event]);
    intents
}

#[test]
fn graph_view_menu__its_button_is_right_before_more_with_the_tooltip_view_in_a_wide_and_a_narrow_window() {
    let (repository, _remote) = repository_with_history();
    let state = showing_with(repository.path().to_path_buf(), Settings::default());
    for size in [WIDE, NARROW] {
        let mut window = Window::new(ThemeName::Dark, size);
        let (_, tree) = window.draw(&state, &[]);
        let view = header_button(&tree, "View");
        let more = header_button(&tree, "More");
        assert!(
            view.right() <= more.x && more.x - view.right() < 16.0,
            "right before More: {view:?} {more:?}"
        );
        assert!((view.y - more.y).abs() < 0.5 && (view.height - more.height).abs() < 0.5);
        let _ = window.click(&state, Role::Button, "View");
        let (_, tree) = window.draw(&state, &[]);
        let first = bounds(&tree, Role::MenuItem, "Show local branches");
        assert!(
            first.y >= view.bottom(),
            "the menu opens under its button: {first:?} {view:?}"
        );
        assert!(!has(&tree, Role::MenuItem, "Refresh"), "the actions are in More");
    }
}

#[test]
fn graph_view_menu__unchecking_show_the_hash_takes_the_hashes_out_and_writes_author_and_date() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let with_hashes = window.frame.to_rgba();
    let _ = window.click(&state, Role::Button, "View");
    let menu = bounds(&window.draw(&state, &[]).1, Role::MenuItem, "Show local branches");
    let _ = wheel(&mut window, &state, menu.center(), 5.0);
    let intents = window.click(&state, Role::MenuItem, "Show the hash");
    assert_eq!(
        intents,
        [Intent::Graph(GraphIntent::Switch(TimelineSwitch::Hash, false))]
    );
    let effects: Vec<Effect> = intents
        .into_iter()
        .flat_map(|intent| run(&mut state, app::store::Message::Intent(intent)))
        .collect();
    assert!(!state.settings.switches.is_on(TimelineSwitch::Hash));
    let columns = json::Value::Array(vec![
        json::Value::String("author".into()),
        json::Value::String("date".into()),
    ]);
    assert_eq!(written(&effects), Some(("graph.columns", columns)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    assert_ne!(window.frame.to_rgba(), with_hashes, "the hashes are taken out");
}

#[test]
fn graph_view_menu__in_a_window_400_high_the_menu_opens_under_its_button_and_its_last_entry_is_chosen() {
    let (repository, _remote) = repository_with_history();
    let state = showing_with(repository.path().to_path_buf(), Settings::default());
    let mut window = Window::new(ThemeName::Dark, (1000, 400));
    let button = bounds(&window.draw(&state, &[]).1, Role::Button, "View");
    let _ = window.click(&state, Role::Button, "View");
    let (_, tree) = window.draw(&state, &[]);
    let first = bounds(&tree, Role::MenuItem, "Show local branches");
    assert!(first.y >= button.bottom(), "under its button: {first:?} {button:?}");
    assert!(
        !has(&tree, Role::MenuItem, "Show the hash"),
        "the last entry is not shown yet"
    );
    let _ = press(&mut window, &state, NamedKey::End);
    let (_, tree) = window.draw(&state, &[]);
    let hash = bounds(&tree, Role::MenuItem, "Show the hash");
    assert!(hash.bottom() <= 400.0 - 8.0, "shown whole: {hash:?}");
    let intents = press(&mut window, &state, NamedKey::Enter);
    assert_eq!(
        intents,
        [Intent::Graph(GraphIntent::Switch(TimelineSwitch::Hash, false))]
    );
}

#[test]
fn graph_view_menu__the_menu_open_matches_its_snapshots_in_the_four_themes() {
    let (repository, _remote) = repository_with_history();
    let state = showing_with(repository.path().to_path_buf(), Settings::default());
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, TALL);
        let _ = window.draw(&state, &[]);
        let _ = window.click(&state, Role::Button, "View");
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("view-menu-{name}"));
    }
}

#[test]
fn graph_more_menu__the_menu_open_matches_its_snapshots_in_the_four_themes() {
    let (repository, _remote) = repository_with_history();
    let state = showing_with(repository.path().to_path_buf(), Settings::default());
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, TALL);
        let _ = window.draw(&state, &[]);
        let _ = window.click(&state, Role::Button, "More");
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("more-menu-{name}"));
    }
}
