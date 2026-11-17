//! The bottom panel (`panel.bottom`): `` Ctrl+` ``, its height kept for the workspace, the way to it from the Activity
//! drawer and from the bar of views (`graph.view-bar`), and the Command log in it (`log.show`).

use app::persisted::Persisted;
use app::settings::Settings;
use app::store::{Message, State, panel};
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent, PointerMove};
use views::Intent;
use views::panel::{PanelIntent, PanelTab};

use super::support::{Window, find, has, run, says, shortcut};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);

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

/// The rectangle of the bottom panel in a frame.
fn panel_rect(tree: &TreeUpdate) -> Rect {
    bounds(find(tree, Role::Region, "Bottom panel"))
}

/// Reduces intents in turn.
fn reduce_all(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = run(state, Message::Intent(intent));
    }
}

/// A press of the main button at a point, or its release.
fn button_at(position: Point) -> PointerButtonEvent {
    PointerButtonEvent {
        button: PointerButton::Primary,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    }
}

/// The pointer moved to a point.
fn moved(position: Point) -> InputEvent {
    InputEvent::PointerMoved(PointerMove {
        position,
        modifiers: Modifiers::NONE,
    })
}

#[test]
fn panel_bottom__ctrl_backtick_shows_the_panel_at_the_terminal_with_the_focus_and_hides_it_pressed_again_there() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, tree) = window.draw(&state, &[shortcut('`', Modifiers::CONTROL)]);
    assert!(!has(&tree, Role::Region, "Bottom panel"));
    assert_eq!(intents, [Intent::Panel(PanelIntent::Open)]);
    reduce_all(&mut state, intents);
    let look = panel::look(&state);
    assert!(look.is_shown && look.tab == PanelTab::Terminal, "{look:?}");
    let (_, tree) = window.draw(&state, &[]);
    let rect = panel_rect(&tree);
    let (intents, _) = window.draw(&state, &[shortcut('`', Modifiers::CONTROL)]);
    assert_eq!(
        intents,
        [Intent::Panel(PanelIntent::Hide)],
        "the panel has the keyboard: {rect:?}"
    );
    reduce_all(&mut state, intents);
    assert!(!panel::look(&state).is_shown);
}

#[test]
fn panel_bottom__show_the_commands_gitardo_runs_in_the_activity_drawer_shows_the_panel_at_the_command_log() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _opened = run(&mut state, Message::Intent(Intent::OpenActivity));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Show the commands Gitardo runs");
    assert_eq!(intents, [Intent::ShowCommandLog]);
    reduce_all(&mut state, intents);
    let look = panel::look(&state);
    assert!(look.is_shown && look.tab == PanelTab::CommandLog, "{look:?}");
    let (_, tree) = window.draw(&state, &[]);
    let rect = panel_rect(&tree);
    assert!(says(&tree, "No command has run yet"));
    assert!(
        rect.bottom() > 600.0,
        "the panel is at the foot of the window: {rect:?}"
    );
}

#[test]
fn panel_bottom__the_height_dragged_to_300_px_is_there_after_the_window_is_opened_again_on_the_same_workspace() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    reduce_all(&mut state, vec![Intent::Panel(PanelIntent::Open)]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rect = panel_rect(&tree);
    assert!(
        (rect.height - 240.0).abs() < 1.0,
        "the panel starts at 240 px: {rect:?}"
    );
    let start = Point::new(rect.x + rect.width * 0.5, rect.y - 4.0);
    let end = Point::new(start.x, start.y - 60.0);
    let (intents, _) = window.draw(
        &state,
        &[
            moved(start),
            InputEvent::PointerPressed(button_at(start)),
            moved(end),
            InputEvent::PointerReleased(button_at(end)),
        ],
    );
    assert_eq!(intents, [Intent::Panel(PanelIntent::SetHeight(Some(300.0)))]);
    reduce_all(&mut state, intents);
    let kept = Persisted::from_text(&state.persisted.to_text());
    let mut again = State::new(kept, Settings::default(), None);
    let _started = run(&mut again, Message::Started { folders: vec![folder] });
    let look = panel::look(&again);
    assert!(look.is_shown && look.height == Some(300.0), "{look:?}");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&again, &[]);
    assert!((panel_rect(&tree).height - 300.0).abs() < 1.0);
}

#[test]
fn panel_bottom__the_panel_is_under_the_timeline_and_the_inspector_and_the_rail_keeps_its_height() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    reduce_all(&mut state, vec![Intent::ShowCommandLog]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rect = panel_rect(&tree);
    let search = bounds(find(&tree, Role::SearchInput, "Find by message, author or hash"));
    let rail = bounds(find(&tree, Role::Navigation, "Branches"));
    assert!(
        rail.bottom() > rect.y + rect.height * 0.5,
        "the rail keeps its height: {rail:?}"
    );
    assert!(
        rect.x < search.x && rect.x > 200.0,
        "the panel starts after the rail: {rect:?}"
    );
    assert!(
        rect.right() > 980.0,
        "the panel reaches the right of the inspector: {rect:?}"
    );
    window.assert_snapshot("bottom-panel-dark");
}

#[test]
fn graph_view_bar__a_click_on_terminal_shows_the_bottom_panel_and_another_hides_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Terminal");
    assert_eq!(intents, [Intent::Panel(PanelIntent::Toggle)]);
    reduce_all(&mut state, intents);
    assert!(panel::look(&state).is_shown);
    let (_, tree) = window.draw(&state, &[]);
    let _shown = panel_rect(&tree);
    let bar_terminal = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Terminal"))
        .map(|(_, node)| bounds(node))
        .min_by(|one, other| one.x.total_cmp(&other.x))
        .unwrap();
    let center = Point::new(
        bar_terminal.x + bar_terminal.width * 0.5,
        bar_terminal.y + bar_terminal.height * 0.5,
    );
    let intents = window.click_at(&state, center, 1);
    assert_eq!(intents, [Intent::Panel(PanelIntent::Toggle)]);
    reduce_all(&mut state, intents);
    assert!(!panel::look(&state).is_shown);
}

#[test]
fn log_show__it_is_in_the_bottom_panel_and_no_longer_in_the_activity_drawer() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    state.log.set_level(engine::LogLevel::Debug);
    state
        .log
        .record_failure(model::Timestamp(super::support::NOW), "fatal: not a git repository");
    reduce_all(&mut state, vec![Intent::OpenActivity, Intent::ShowCommandLog]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(find(&tree, Role::ListBox, "Command log"));
    let rect = panel_rect(&tree);
    assert!(
        rect.contains(Point::new(list.x + 1.0, list.y + 1.0)),
        "{list:?} in {rect:?}"
    );
    assert!(!has(&tree, Role::Button, "Hide the commands"));
    reduce_all(&mut state, vec![Intent::CloseActivity]);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("command-log-dark");
}
