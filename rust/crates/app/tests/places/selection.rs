//! What is selected, in the colors of each theme (`settings.theme`, D69): a commit selected in the timeline stands on
//! the selection of the theme while its list has the focus, the accent mixed into the ground in the dark and the
//! light theme, and on the neutral selection of a list without the focus once the window loses it.

use app::store::{Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::{Appearance, Color, Rect, Theme};
use ui::InputEvent;
use views::Intent;
use views::graph::{GraphIntent, Selected, TimelineSwitch};

use super::label_overflow::THEMES;
use super::pointer::{apply, bounds, click_at, rect};
use super::support::{Window, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const SIZE: (u32, u32) = (1000, 520);

/// The row of the timeline that is selected, as the tree says it.
fn selected_row(tree: &TreeUpdate) -> Rect {
    let history = bounds(tree, Role::ListBox, "History");
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption && node.is_selected() == Some(true))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|row| row.x >= history.x && row.right() <= history.right() + 0.5)
        .unwrap_or_else(|| panic!("no row of the timeline is selected"))
}

/// The part of the pixels of a rectangle of the window that have a color, from 0 to 1.
fn part_in(window: &Window, within: Rect, color: Color) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an i32 counts"
    )]
    let whole = |value: f32| value.round() as i32;
    let (mut found, mut all) = (0_u16, 0_u16);
    for row in whole(within.y)..whole(within.bottom()) {
        for column in whole(within.x)..whole(within.right()) {
            all += 1;
            if window.frame.pixel(column, row) == Some(color) {
                found += 1;
            }
        }
    }
    f32::from(found) / f32::from(all.max(1))
}

/// Selects the commit of the third row of the timeline with a click, which gives its list the focus, and draws the
/// window again. Returns the row.
fn select_a_commit(window: &mut Window, state: &mut State) -> Rect {
    let tree = window.draw(state, &[]).1;
    let history = bounds(&tree, Role::ListBox, "History");
    let third = super::pointer::rows_drawn(&tree)
        .into_iter()
        .filter(|row| row.x >= history.x && row.right() <= history.right() + 0.5)
        .nth(2)
        .unwrap_or_else(|| panic!("the timeline has fewer than three rows"));
    let intents = click_at(window, state, third.center(), 1);
    apply(state, intents);
    assert!(matches!(state.timeline.selected, Some(Selected::Commit(_))));
    selected_row(&window.draw(state, &[]).1)
}

/// Checks that the selected row stands on the selection of the theme, and without the focus of the window on the
/// selection of a list without the focus.
fn assert_tinted(window: &mut Window, state: &State, (row, theme): (Rect, &Theme), name: &str) {
    let colors = theme.colors;
    // The ground of a row is most of it: its text, its labels and its rounded corners are the rest.
    let tinted = part_in(window, row, colors.list_selection);
    assert!(tinted > 0.5, "{name}: {tinted} of the selected row is its selection");
    let _ = window.draw(state, &[InputEvent::FocusLost]);
    let (tinted, neutral) = (
        part_in(window, row, colors.list_selection),
        part_in(window, row, colors.list_inactive_selection),
    );
    assert!(
        tinted < 0.01 && neutral > 0.5,
        "{name}: without the focus {tinted} of the row is the selection and {neutral} the one without the focus"
    );
    let _ = window.draw(state, &[InputEvent::FocusGained]);
}

#[test]
fn settings_theme__a_commit_selected_in_the_timeline_stands_on_the_accent_mixed_into_the_ground_while_its_list_has_the_focus()
 {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    for (name, label) in THEMES {
        let theme = Theme::new(name, Appearance::Dark);
        let mut window = Window::new(name, SIZE);
        let row = select_a_commit(&mut window, &mut state);
        assert_tinted(&mut window, &state, (row, &theme), label);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("timeline-selected-{label}"));
    }
}

#[test]
fn settings_theme__a_commit_selected_in_the_compact_rows_stands_on_the_same_selection() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let switch = GraphIntent::Switch(TimelineSwitch::CompactRows, true);
    let _ = run(&mut state, Message::Intent(Intent::Graph(switch)));
    for (name, label) in THEMES {
        let theme = Theme::new(name, Appearance::Dark);
        let mut window = Window::new(name, SIZE);
        let row = select_a_commit(&mut window, &mut state);
        assert_tinted(&mut window, &state, (row, &theme), label);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("timeline-compact-selected-{label}"));
    }
}
