//! The Activity tab of the bottom panel (`activity.what-is-running`, D64): what it lists, the ways to it from the
//! activity indicator of the status bar (`activity.status-bar`), and its pictures in each theme.

use std::time::Duration;

use app::store::{Message, State, ViewData, panel};
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, PointerMove};
use views::Intent;
use views::activity::{ActivityState, RanTask, RunningTask, what_is_running_view};
use views::panel::{PanelIntent, PanelTab};

use super::panel::without_sizes;
use super::support::{NOW, Window, find, has, run, says};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);
/// The room the bottom panel gives the tab in a window of that size.
pub(super) const TAB: Rect = Rect::new(280.0, 420.0, 700.0, 190.0);
/// The themes, each with the end of the names of its pictures.
pub(super) const THEMES: [(ThemeName, &str); 4] = [
    (ThemeName::Dark, "dark"),
    (ThemeName::Light, "light"),
    (ThemeName::HighContrastDark, "high-contrast-dark"),
    (ThemeName::HighContrastLight, "high-contrast-light"),
];

/// Something running for some milliseconds.
fn task(title: &str, millis: u64, detail: Option<&str>) -> RunningTask {
    RunningTask {
        title: title.to_owned(),
        detail: detail.map(str::to_owned),
        elapsed: Duration::from_millis(millis),
    }
}

/// The rectangle of a node, which the test fails without.
fn bounds(node: &Node) -> Rect {
    let bounds = node.bounds().unwrap();
    #[allow(clippy::cast_possible_truncation, reason = "the bounds of a window fit in an f32")]
    Rect::new(
        bounds.x0 as f32,
        bounds.y0 as f32,
        (bounds.x1 - bounds.x0) as f32,
        (bounds.y1 - bounds.y0) as f32,
    )
}

/// Something that ran for some milliseconds, and ended some seconds after the moment the tests say dates from.
fn ran(title: &str, millis: u64, detail: Option<&str>, ended: i64) -> RanTask {
    RanTask {
        task: task(title, millis, detail),
        ended: NOW + ended,
    }
}

/// The tree of a frame of the tab alone, over the room the bottom panel gives it in a window.
pub(super) fn tab(window: &mut Window, state: &ActivityState<'_>) -> TreeUpdate {
    window
        .frame(&[], |ui| ui.clip(TAB, |ui| what_is_running_view(state, ui)))
        .1
}

/// A frame of the whole window with what runs and what ran, and some input.
pub(super) fn draw_running(
    window: &mut Window,
    state: &State,
    (running, ran): (&[RunningTask], &[RanTask]),
    events: &[InputEvent],
) -> (Vec<Intent>, TreeUpdate) {
    let data = ViewData::new(state, running.to_vec(), NOW).with_ran(ran.to_vec());
    window.frame(events, |ui| views::view(&data.window_state(state), ui))
}

/// The events of a click at the middle of a rectangle: the pointer moved there, then pressed and released.
fn click_events(rect: Rect) -> [Vec<InputEvent>; 2] {
    let position = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    let button = PointerButtonEvent {
        button: PointerButton::Primary,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let moved = InputEvent::PointerMoved(PointerMove {
        position,
        modifiers: Modifiers::NONE,
    });
    [
        vec![moved],
        vec![InputEvent::PointerPressed(button), InputEvent::PointerReleased(button)],
    ]
}

/// Reduces intents in turn.
fn reduce_all(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = run(state, Message::Intent(intent));
    }
}

/// The buttons of a name in a frame, from the left: the one of the bar of views, then the tab of the bottom panel.
fn buttons(tree: &TreeUpdate, name: &str) -> Vec<Rect> {
    let mut found: Vec<Rect> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .map(|(_, node)| bounds(node))
        .collect();
    found.sort_by(|one, other| one.x.total_cmp(&other.x));
    found
}

#[test]
fn activity_what_is_running__shown_during_a_long_fetch_the_tab_lists_it_and_its_time_grows() {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for (millis, said) in [(3_200, "3 s"), (4_100, "4 s")] {
        let running = [task("Fetching origin", millis, Some("Because Sync was clicked."))];
        let tree = tab(
            &mut window,
            &ActivityState {
                running: &running,
                ..ActivityState::default()
            },
        );
        for words in ["Fetching origin", said, "Because Sync was clicked."] {
            assert!(says(&tree, words), "{words}");
        }
    }
}

#[test]
fn activity_what_is_running__when_the_fetch_ends_the_tab_says_nothing_is_running_now() {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let ran = [ran("Fetching origin", 4_000, None, 129)];
    let tree = tab(
        &mut window,
        &ActivityState {
            running: &[],
            ran: &ran,
        },
    );
    assert!(says(&tree, "Nothing is running now"));
    assert!(says(&tree, "RAN") && says(&tree, "03:02:09 \u{b7} took 4 s"));
}

#[test]
fn activity_what_is_running__a_click_on_the_indicator_shows_the_panel_at_activity_and_nothing_is_over_the_timeline() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let running = [task("Fetching origin", 2_000, Some("Because Sync was clicked."))];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = draw_running(&mut window, &state, (&running, &[]), &[]);
    assert!(!has(&tree, Role::Region, "Bottom panel"));
    let indicator = bounds(find(&tree, Role::Button, "Fetching origin (2 s)"));
    let [moved, clicked] = click_events(indicator);
    let _ = draw_running(&mut window, &state, (&running, &[]), &moved);
    let (intents, _) = draw_running(&mut window, &state, (&running, &[]), &clicked);
    assert_eq!(intents, [Intent::Panel(PanelIntent::ToggleActivity)]);
    reduce_all(&mut state, intents);
    let (_, tree) = draw_running(&mut window, &state, (&running, &[]), &[]);
    let panel_rect = bounds(find(&tree, Role::Region, "Bottom panel"));
    let list = bounds(find(&tree, Role::Region, "What is running"));
    assert!(panel_rect.contains(Point::new(list.x + 1.0, list.y + 1.0)), "{list:?}");
    assert!(says(&tree, "Because Sync was clicked."));
    let tabs: Vec<f32> = ["Terminal", "Activity", "Command log"]
        .iter()
        .map(|name| buttons(&tree, name).last().unwrap().x)
        .collect();
    assert!(tabs.is_sorted() && tabs.iter().all(|x| *x > panel_rect.x), "{tabs:?}");
    let timeline = bounds(find(&tree, Role::Main, "Timeline"));
    assert!(
        timeline.bottom() < panel_rect.y && timeline.height > 100.0,
        "the timeline is whole above the panel: {timeline:?} {panel_rect:?}"
    );
    assert!(
        window
            .pointer_targets(timeline)
            .iter()
            .all(|target| target.bottom() <= panel_rect.y),
        "nothing over the timeline takes the pointer"
    );
}

#[test]
fn activity_status_bar__a_click_on_the_indicator_shows_the_bottom_panel_at_activity_and_another_hides_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let running = [task("Fetching origin", 2_000, None)];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for is_shown in [true, false] {
        let (_, tree) = draw_running(&mut window, &state, (&running, &[]), &[]);
        let indicator = bounds(find(&tree, Role::Button, "Fetching origin (2 s)"));
        let [moved, clicked] = click_events(indicator);
        let _ = draw_running(&mut window, &state, (&running, &[]), &moved);
        let (intents, _) = draw_running(&mut window, &state, (&running, &[]), &clicked);
        reduce_all(&mut state, without_sizes(&intents));
        let look = panel::look(&state);
        assert_eq!((look.is_shown, look.tab), (is_shown, PanelTab::Activity));
    }
}

#[test]
fn activity_what_is_running__escape_with_the_panel_at_activity_hides_nothing() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    reduce_all(&mut state, vec![Intent::Panel(PanelIntent::ToggleActivity)]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let (intents, tree) = window.draw(&state, &[escape]);
    assert_eq!(intents, []);
    let _still = find(&tree, Role::Region, "What is running");
}

#[test]
fn activity_what_is_running__the_tab_command_log_beside_it_shows_the_commands_gitardo_runs() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    reduce_all(&mut state, vec![Intent::Panel(PanelIntent::ToggleActivity)]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Show the commands Gitardo runs"));
    let intents = window.click(&state, Role::Button, "Command log");
    assert_eq!(intents, [Intent::Panel(PanelIntent::Show(PanelTab::CommandLog))]);
    reduce_all(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "No command has run yet"));
}

#[test]
fn activity_what_is_running__the_tab_is_drawn_empty_and_running_in_each_theme() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    reduce_all(&mut state, vec![Intent::Panel(PanelIntent::ToggleActivity)]);
    let running = [
        task("Fetching origin", 12_400, Some("Sync in the header")),
        task("Reading app \u{d7}4", 2_400, Some("Because src/a.rs changed.")),
        task("Pushing main to origin", 300, None),
    ];
    let ran = [
        ran("Reading lib", 700, Some("Because the window got the focus."), -65),
        ran(
            "Looking for repositories in app",
            1_000,
            Some("Opening the workspace"),
            -3_600,
        ),
    ];
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, WIDE);
        let _ = draw_running(&mut window, &state, (&[], &[]), &[]);
        window.assert_snapshot(&format!("activity-tab-empty-{name}"));
        let _ = draw_running(&mut window, &state, (&running, &ran), &[]);
        window.assert_snapshot(&format!("activity-tab-running-{name}"));
    }
}
