//! The edge of the graph, the lines beside the branch rail and the inspector, and the branch rail put away: dragged and
//! clicked in the window, and kept for the workspace when it is opened again.

use std::path::Path;

use app::settings::Settings;
use app::store::{Message, State};
use platform::accesskit::Role;
use render::{Point, Rect, ThemeName};
use views::graph::{GraphEdgeState, GraphIntent, Pane};
use views::{FilterIntent, Intent};

use super::folds::feature_left_main;
use super::pointer::{apply, bounds, click_at, drag};
use super::support::{Window, has, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);
/// The button that puts the branch rail away.
const HIDE: &str = "Hide the branches, tags and stashes beside the commits";
/// The button that brings it back.
const SHOW: &str = "Show the branches, tags and stashes beside the commits";

/// The window opened again on the workspace of a store, with what it kept.
fn opened_again(state: &State, folder: &Path) -> State {
    let mut again = State::new(state.persisted.clone(), Settings::default(), None);
    let _effects = run(
        &mut again,
        Message::Started {
            folders: vec![folder.to_path_buf()],
        },
    );
    again
}

/// The widths of the graph the intents set.
fn graph_widths(intents: &[Intent]) -> Vec<Option<f32>> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Graph(GraphIntent::SetGraphWidth(width)) => Some(*width),
            _ => None,
        })
        .collect()
}

/// The list of the commits and where the edge of the graph is in it.
fn edge_of(window: &mut Window, state: &State) -> (Rect, Point) {
    let (_, tree) = window.draw(state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let wanted = super::pointer::graph_width_of(state);
    let edge = GraphEdgeState {
        lanes: state.timeline.layout.lane_count,
        wanted,
    };
    let x = list.x + edge.fit(list.width).width + 0.5;
    (list, Point::new(x, list.y + list.height * 0.5))
}

#[test]
fn graph_resize_graph__dragging_the_edge_past_the_natural_width_of_the_graph_stops_at_it() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (list, edge) = edge_of(&mut window, &state);
    let natural = GraphEdgeState {
        lanes: state.timeline.layout.lane_count,
        wanted: None,
    }
    .fit(list.width)
    .natural_width;
    let intents = drag(&mut window, &state, edge, Point::new(edge.x - 10.0, edge.y));
    let narrowed = graph_widths(&intents).last().copied().flatten().unwrap();
    assert!(
        (narrowed - (natural - 10.0)).abs() < 0.5,
        "dragged to {narrowed}, natural {natural}"
    );
    apply(&mut state, intents);
    let (_, edge) = edge_of(&mut window, &state);
    let intents = drag(&mut window, &state, edge, Point::new(edge.x + 300.0, edge.y));
    let widened = graph_widths(&intents).last().copied().flatten().unwrap();
    assert!(
        (widened - natural).abs() < 0.5,
        "dragged to {widened}, natural {natural}"
    );
}

#[test]
fn graph_resize_graph__a_double_click_on_the_edge_brings_back_the_width_the_graph_chooses() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    apply(&mut state, vec![Intent::Graph(GraphIntent::SetGraphWidth(Some(24.0)))]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, edge) = edge_of(&mut window, &state);
    let intents = click_at(&mut window, &state, edge, 2);
    assert_eq!(graph_widths(&intents).last(), Some(&None));
    apply(&mut state, intents);
    assert_eq!(super::pointer::graph_width_of(&state), None);
}

#[test]
fn graph_resize_graph__the_width_dragged_is_there_after_the_window_is_opened_again() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, edge) = edge_of(&mut window, &state);
    let intents = drag(&mut window, &state, edge, Point::new(edge.x - 10.0, edge.y));
    let dragged = graph_widths(&intents).last().copied().flatten().unwrap();
    apply(&mut state, intents);
    let again = opened_again(&state, repository.path());
    assert_eq!(super::pointer::graph_width_of(&again), Some(dragged));
}

#[test]
fn graph_resize_graph__the_width_dragged_stays_while_the_timeline_is_narrowed_to_the_branch_checked_out() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, edge) = edge_of(&mut window, &state);
    let intents = drag(&mut window, &state, edge, Point::new(edge.x - 10.0, edge.y));
    let dragged = graph_widths(&intents).last().copied().flatten().unwrap();
    apply(&mut state, intents);
    let current = model::filter::preset(model::filter::Preset::Current);
    let _effects = run(&mut state, Message::Intent(Intent::Filter(FilterIntent::Show(current))));
    assert!(state.timeline.shown.is_some());
    assert_eq!(super::pointer::graph_width_of(&state), Some(dragged));
}

#[test]
fn graph_resize_panes__a_double_click_on_the_line_before_the_inspector_gives_it_320_px_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let at_start = bounds(&tree, Role::Complementary, "Selected commit");
    apply(
        &mut state,
        vec![Intent::Graph(GraphIntent::SetPane(Pane::Inspector, Some(400.0)))],
    );
    let (_, tree) = window.draw(&state, &[]);
    let wide = bounds(&tree, Role::Complementary, "Selected commit");
    assert!((wide.width - at_start.width - 80.0).abs() < 0.5, "{wide:?}");
    let line = Point::new(wide.x - 0.5, wide.y + wide.height * 0.5);
    let intents = click_at(&mut window, &state, line, 2);
    assert!(
        intents.contains(&Intent::Graph(GraphIntent::SetPane(Pane::Inspector, None))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(bounds(&tree, Role::Complementary, "Selected commit"), at_start);
}

#[test]
fn graph_resize_panes__the_sizes_are_there_after_the_window_is_closed_and_opened_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rail = bounds(&tree, Role::Navigation, "Branches");
    let line = Point::new(rail.right() + 0.5, rail.y + rail.height * 0.5);
    let intents = drag(&mut window, &state, line, Point::new(line.x + 60.0, line.y));
    apply(&mut state, intents);
    let again = opened_again(&state, repository.path());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&again, &[]);
    let kept = bounds(&tree, Role::Navigation, "Branches");
    assert!((kept.width - rail.width - 60.0).abs() < 0.5, "{kept:?}");
}

#[test]
fn graph_toggle_references__a_click_hides_the_branch_rail_and_another_brings_it_back_at_the_width_it_had() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    apply(
        &mut state,
        vec![Intent::Graph(GraphIntent::SetPane(Pane::Rail, Some(260.0)))],
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, HIDE);
    assert_eq!(intents, [Intent::Graph(GraphIntent::ToggleReferences)]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Navigation, "Branches"));
    assert!(
        bounds(&tree, Role::Main, "Timeline").x < 60.0,
        "the commits take its room, beside the bar of views"
    );
    let intents = window.click(&state, Role::Button, SHOW);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(bounds(&tree, Role::Navigation, "Branches").width, 260.0);
}

#[test]
fn graph_toggle_references__a_hidden_branch_rail_stays_hidden_after_the_window_is_opened_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    apply(&mut state, vec![Intent::Graph(GraphIntent::ToggleReferences)]);
    let again = opened_again(&state, repository.path());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&again, &[]);
    assert!(!has(&tree, Role::Navigation, "Branches"));
    assert!(has(&tree, Role::Button, SHOW));
}
