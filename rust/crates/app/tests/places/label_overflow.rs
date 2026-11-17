//! The labels of a commit first on its row, and the list of all of them under the pointer (`graph.label-overflow`), on a
//! repository made for the test whose last commit has six labels.

use std::time::Duration;

use app::store::State;
use model::contract::{FeatureId, Target};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::{Remote, Repository};
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::feature::FeatureIntent;
use views::graph::{GraphIntent, Selected};

use super::pointer::{apply, bounds, click_at, moved, rows_drawn, said_at};
use super::showing;
use super::support::{Window, has};

/// The themes of the snapshots, with the end of their names.
pub const THEMES: [(ThemeName, &str); 4] = [
    (ThemeName::Dark, "dark"),
    (ThemeName::Light, "light"),
    (ThemeName::HighContrastDark, "high-contrast-dark"),
    (ThemeName::HighContrastLight, "high-contrast-light"),
];

/// The size of the window of these tests.
const SIZE: (u32, u32) = (1000, 520);

/// A repository whose last commit has six labels: `feature` checked out, `main` with its upstream `origin/main`,
/// `release`, and the tags `v1.0`, `v1.1` and `v1.2`.
pub fn six_labels() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    let remote = Remote::new(&repository);
    for tag in ["v1.0", "v1.1", "v1.2"] {
        repository.tag(tag);
    }
    repository.branch("release");
    repository.branch("feature");
    repository.checkout("feature");
    (repository, remote)
}

/// The window drawn once, and the tree of the frame.
fn drawn(window: &mut Window, state: &State) -> TreeUpdate {
    window.draw(state, &[]).1
}

/// Rests the pointer on a point for a while, and gives the intents of the frames, applied to the store.
fn rest_on(window: &mut Window, state: &mut State, point: Point, while_: Duration) -> Vec<Intent> {
    let mut intents = window.draw(state, &[moved(point)]).0;
    window.now += while_;
    intents.extend(window.draw(state, &[]).0);
    apply(state, intents.clone());
    intents
}

/// Whether the list of the labels is open.
fn is_open(tree: &TreeUpdate) -> bool {
    has(tree, Role::Dialog, "Labels")
}

/// The rows of the list of the labels, from the top.
fn listed(tree: &TreeUpdate) -> Vec<Rect> {
    let list = bounds(tree, Role::Dialog, "Labels");
    rows_drawn(tree)
        .into_iter()
        .filter(|row| list.x <= row.x && row.bottom() <= list.bottom() + 0.5 && row.y >= list.y - 0.5)
        .collect()
}

/// The label `+N` of the first row, and the window showing the repository with the list open from it.
fn opened() -> (Repository, Remote, State, Window, Rect) {
    let (repository, remote) = six_labels();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    let count = said_at(&tree, "+5", rows_drawn(&tree).first().copied().unwrap());
    let _ = rest_on(&mut window, &mut state, count.center(), Duration::from_millis(400));
    assert!(is_open(&drawn(&mut window, &state)), "the list is open");
    (repository, remote, state, window, count)
}

#[test]
fn graph_label_overflow__a_commit_with_six_labels_starts_its_row_with_those_that_fit_then_plus_n_then_its_message() {
    let (repository, _remote) = six_labels();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    let row = rows_drawn(&tree)[0];
    let feature = said_at(&tree, "feature", row);
    let count = said_at(&tree, "+5", row);
    assert!(feature.right() <= count.x, "the label first, then the count");
    let labels = bounds(&tree, Role::ListBox, "Labels");
    assert!(
        labels.x <= feature.x && count.right() <= labels.right() + 0.5,
        "the labels are a list"
    );
    assert!(
        labels.x - row.x < 0.25 * row.width && labels.width < 0.4 * row.width,
        "the labels start the line of the message, which keeps the rest: {labels:?} {row:?}"
    );
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("timeline-labels-first-{name}"));
    }
}

#[test]
fn graph_label_overflow__resting_300_ms_on_plus_n_opens_under_it_the_list_of_all_six_labels_in_their_order() {
    let (repository, _remote) = six_labels();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = drawn(&mut window, &state);
    let count = said_at(&tree, "+5", rows_drawn(&tree)[0]);
    let early = rest_on(&mut window, &mut state, count.center(), Duration::from_millis(250));
    assert!(
        !early
            .iter()
            .any(|intent| matches!(intent, Intent::Graph(GraphIntent::OpenLabels(_)))),
        "not before 300 ms"
    );
    window.now += Duration::from_millis(100);
    let intents = window.draw(&state, &[]).0;
    apply(&mut state, intents);
    let tree = drawn(&mut window, &state);
    assert!(is_open(&tree));
    let list = bounds(&tree, Role::Dialog, "Labels");
    assert!(
        list.y >= count.bottom(),
        "under the count, not over it: {list:?} {count:?}"
    );
    let rows = listed(&tree);
    let names = ["feature", "main", "release", "v1.0", "v1.1", "v1.2"];
    assert_eq!(rows.len(), names.len());
    for (name, row) in names.iter().zip(&rows) {
        let _ = said_at(&tree, name, *row);
    }
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[moved(count.center())]);
        window.assert_snapshot(&format!("timeline-label-list-{name}"));
    }
}

#[test]
fn graph_label_overflow__a_double_click_on_main_in_the_list_checks_it_out() {
    let (_repository, _remote, mut state, mut window, _count) = opened();
    let tree = drawn(&mut window, &state);
    let main = listed(&tree)[1];
    let intents = click_at(&mut window, &state, main.center(), 2);
    let check_out = Intent::Feature(FeatureIntent::Run(
        FeatureId("branch.checkout"),
        Target::Branch(String::from("main")),
    ));
    assert!(intents.contains(&check_out), "{intents:?}");
    apply(&mut state, intents);
    assert!(!is_open(&drawn(&mut window, &state)), "the list closes");
}

#[test]
fn graph_label_overflow__a_click_on_a_label_of_the_list_selects_its_commit() {
    let (_repository, _remote, state, mut window, _count) = opened();
    let tree = drawn(&mut window, &state);
    let tag = listed(&tree)[4];
    let intents = click_at(&mut window, &state, tag.center(), 1);
    let selected = intents.iter().find_map(|intent| match intent {
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit))) => Some(commit.clone()),
        _ => None,
    });
    let first = state.timeline.rows.first().and_then(model::graph::Row::id).cloned();
    assert_eq!(selected, first);
}

#[test]
fn graph_label_overflow__a_right_click_on_a_label_of_the_list_opens_its_menu() {
    let (_repository, _remote, mut state, mut window, _count) = opened();
    let tree = drawn(&mut window, &state);
    let release = listed(&tree)[2];
    let right = ui::PointerButtonEvent {
        button: ui::PointerButton::Secondary,
        position: release.center(),
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let _ = window.draw(&state, &[moved(release.center())]);
    let (intents, _) = window.draw(
        &state,
        &[InputEvent::PointerPressed(right), InputEvent::PointerReleased(right)],
    );
    apply(&mut state, intents);
    let tree = drawn(&mut window, &state);
    let items: Vec<String> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect();
    assert!(
        items.iter().any(|item| item.contains("release")),
        "the menu of release: {items:?}"
    );
}

#[test]
fn graph_label_overflow__resting_on_a_label_of_the_list_brings_out_its_history() {
    let (_repository, _remote, mut state, mut window, _count) = opened();
    let tree = drawn(&mut window, &state);
    let tag = listed(&tree)[3];
    assert!(
        state.timeline.hover.is_some(),
        "the history of the commit stands out from resting on +5"
    );
    let _ = rest_on(&mut window, &mut state, tag.center(), Duration::from_millis(300));
    let _ = rest_on(&mut window, &mut state, tag.center(), Duration::from_millis(300));
    assert!(
        state.timeline.hover.is_some() && state.timeline.hover_left_at.is_none(),
        "it stays while the pointer rests on a label of the list, past the 100 ms it lingers after leaving the labels"
    );
}

#[test]
fn graph_label_overflow__leaving_the_list_and_its_labels_or_escape_closes_it_and_it_does_not_open_again_in_place() {
    let (_repository, _remote, mut state, mut window, count) = opened();
    let tree = drawn(&mut window, &state);
    let list = bounds(&tree, Role::Dialog, "Labels");
    let _ = rest_on(&mut window, &mut state, list.center(), Duration::from_millis(50));
    assert!(
        is_open(&drawn(&mut window, &state)),
        "the pointer went from the count into the list"
    );
    let _ = rest_on(
        &mut window,
        &mut state,
        Point::new(5.0, 500.0),
        Duration::from_millis(50),
    );
    assert!(!is_open(&drawn(&mut window, &state)), "the pointer left both");

    let _ = rest_on(&mut window, &mut state, count.center(), Duration::from_millis(400));
    assert!(is_open(&drawn(&mut window, &state)));
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let intents = window.draw(&state, &[escape]).0;
    apply(&mut state, intents);
    window.now += Duration::from_secs(1);
    let intents = window.draw(&state, &[]).0;
    apply(&mut state, intents);
    assert!(
        !is_open(&drawn(&mut window, &state)),
        "Escape closed it, and resting on in place does not open it again"
    );
}

#[test]
fn graph_label_overflow__a_row_whose_labels_all_fit_opens_no_list() {
    let (repository, _remote) = six_labels();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, (1800, 520));
    let tree = drawn(&mut window, &state);
    let row = rows_drawn(&tree)[0];
    let release = said_at(&tree, "release", row);
    let _ = said_at(&tree, "v1.2", row);
    let _ = rest_on(&mut window, &mut state, release.center(), Duration::from_millis(600));
    assert!(state.timeline.label_list.is_none());
    assert!(!is_open(&drawn(&mut window, &state)));
}

#[test]
fn graph_label_overflow__near_the_foot_of_the_window_the_list_opens_above_its_label() {
    let (repository, _remote) = six_labels();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, (1000, 200));
    let tree = drawn(&mut window, &state);
    let count = said_at(&tree, "+5", rows_drawn(&tree)[0]);
    let _ = rest_on(&mut window, &mut state, count.center(), Duration::from_millis(400));
    let tree = drawn(&mut window, &state);
    let list = bounds(&tree, Role::Dialog, "Labels");
    assert!(
        list.bottom() <= count.y,
        "above the count, not over it: {list:?} {count:?}"
    );
}
