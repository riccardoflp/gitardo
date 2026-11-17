//! Compact rows (`graph.compact-rows`), turned on from the store and from the menu of the view, on repositories made for the
//! tests: the height of the rows, the columns at their right end, the labels first, the draft, a stash and the
//! separators of dates on one line, and where the list stays across the switch.

use std::fmt::Write as _;
use std::time::Duration;

use app::settings::Settings;
use app::store::{Effect, Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use testkit::Repository;
use views::Intent;
use views::graph::{GraphIntent, Selected, TimelineSwitch};

use super::histories::{imported, line};
use super::label_overflow::{THEMES, six_labels};
use super::pointer::{apply, bounds, moved, rect, rows_drawn, said_at, wheel};
use super::support::{Window, run};
use super::timeline::{showing_with, written};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const SIZE: (u32, u32) = (1000, 520);

/// The size of a window wide enough for the three columns.
const WIDE: (u32, u32) = (1400, 520);

/// Turns the compact rows on or off in a store.
fn compact(state: &mut State, is_on: bool) -> Vec<Effect> {
    let switch = GraphIntent::Switch(TimelineSwitch::CompactRows, is_on);
    run(state, Message::Intent(Intent::Graph(switch)))
}

/// The rows of the timeline drawn, from the top.
fn timeline_rows(tree: &TreeUpdate) -> Vec<Rect> {
    let history = bounds(tree, Role::ListBox, "History");
    rows_drawn(tree)
        .into_iter()
        .filter(|row| row.x >= history.x && row.right() <= history.right() + 0.5)
        .collect()
}

/// Whether the row selected is among those drawn inside the list, whole.
fn selected_in_sight(tree: &TreeUpdate) -> bool {
    let history = bounds(tree, Role::ListBox, "History");
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption && node.is_selected() == Some(true))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .any(|row| row.y >= history.y - 0.5 && row.bottom() <= history.bottom() + 0.5)
}

/// Where each column of text starts in the right half of a row, from the right end: the pixels of the middle of the row
/// that are not its background, in runs ten pixels apart or more.
fn column_starts(window: &Window, row: Rect) -> Vec<u32> {
    let width = window.frame.width();
    let pixels = window.frame.to_rgba();
    let pixel = |x: u32, y: u32| {
        let at = ((y * width + x) * 4) as usize;
        pixels.get(at..at + 3).map(<[u8]>::to_vec)
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the places of a row are small whole numbers"
    )]
    let (left, right, top, bottom) = (
        (row.x + row.width * 0.5) as u32,
        row.right() as u32 - 1,
        row.y as u32 + 8,
        row.bottom() as u32 - 8,
    );
    let background = pixel(right, top);
    let mut starts = Vec::new();
    let (mut gap, mut last_ink) = (0, None);
    for x in (left..=right).rev() {
        if (top..bottom).any(|y| pixel(x, y) != background) {
            last_ink = Some(x);
            gap = 0;
        } else {
            gap += 1;
            if gap == 10 {
                starts.extend(last_ink.take());
            }
        }
    }
    starts
}

/// A repository whose three commits are months apart, by three authors, with a branch, tags, a stash and a change not
/// committed: what the compact rows are drawn with.
fn months_apart() -> Repository {
    let mut stream = String::new();
    let commits = [
        (82_u64, "Ada Lovelace", "Add the readme"),
        (
            42,
            "Bartholomew Montgomery-Featherstone",
            "Read the settings at startup, and say which file they come from when one of them cannot be read",
        ),
        (2, "Grace Hopper", "Show the branch rail"),
    ];
    for (mark, (days, author, message)) in commits.into_iter().enumerate() {
        let date = testkit::FIRST_COMMIT_TIME - days * 86_400;
        let content = format!("content {mark}\n");
        write!(
            stream,
            "commit refs/heads/main\nmark :{}\ncommitter {author} <author@example.com> {date} +0000\ndata {}\n{message}\nM 644 inline file.txt\ndata {}\n{content}\n",
            mark + 1,
            message.len(),
            content.len()
        )
        .unwrap();
    }
    let repository = imported(&stream);
    let _ = repository.git(&["reset", "--hard", "--quiet"]);
    for tag in ["v1.0", "v1.1"] {
        repository.tag(tag);
    }
    repository.branch("feature");
    std::fs::write(repository.path().join("file.txt"), "stashed\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "half the tags"]);
    std::fs::write(repository.path().join("file.txt"), "changed\n").unwrap();
    repository
}

#[test]
fn graph_compact_rows__turning_it_on_writes_graph_compact_rows_true_in_the_users_settings_and_asks_no_read() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    assert!(
        !state.settings.switches.is_on(TimelineSwitch::CompactRows),
        "off by default"
    );
    let effects = compact(&mut state, true);
    assert_eq!(written(&effects), Some(("graph.compactRows", json::Value::Bool(true))));
    let reads = |effects: &[Effect]| {
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Start(_) | Effect::Cancel(_)))
    };
    assert!(!reads(&effects), "{effects:?}");
    let file = state.user_settings_file.clone().unwrap();
    std::fs::write(&file, "{\n  \"graph.compactRows\": true\n}\n").unwrap();
    let read = app::settings::read(&file, None);
    assert!(read.switches.is_on(TimelineSwitch::CompactRows));
    let effects = run(&mut state, Message::SettingsRead(Box::new(read)));
    assert!(!reads(&effects), "the file read again asks no read either: {effects:?}");
}

#[test]
fn graph_compact_rows__on_each_commit_is_a_row_as_tall_as_a_row_of_a_list_and_off_as_before() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let before = timeline_rows(&window.draw(&state, &[]).1);
    assert!(before.iter().all(|row| (row.height - 52.0).abs() < 0.5), "{before:?}");
    let _ = compact(&mut state, true);
    let rows = timeline_rows(&window.draw(&state, &[]).1);
    assert_eq!(rows.len(), before.len());
    assert!(rows.iter().all(|row| (row.height - 26.0).abs() < 0.5), "{rows:?}");
    let _ = compact(&mut state, false);
    assert_eq!(timeline_rows(&window.draw(&state, &[]).1), before);
}

#[test]
fn graph_compact_rows__on_the_dates_start_at_the_same_place_on_every_row_at_the_right_end() {
    let repository = months_apart();
    let mut state = showing(repository.path().to_path_buf());
    let _ = compact(&mut state, true);
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("timeline-compact-{name}"));
    }
    let mut wide = Window::new(ThemeName::Dark, WIDE);
    let tree = wide.draw(&state, &[]).1;
    wide.assert_snapshot("timeline-compact-wide-dark");
    // The draft and the stash have no columns; the three commits have an author, a date and a hash of their own
    // lengths, each starting where the one of the other rows starts.
    let starts: Vec<Vec<u32>> = timeline_rows(&tree)
        .into_iter()
        .skip(2)
        .map(|row| column_starts(&wide, row).into_iter().take(3).collect())
        .collect();
    assert_eq!(starts.len(), 3);
    assert_eq!(starts[0].len(), 3, "the hash, the date and the author: {starts:?}");
    // A letter starts a pixel before or after another, by its shape.
    let lines_up = |row: &Vec<u32>| row.iter().zip(&starts[0]).all(|(one, other)| one.abs_diff(*other) <= 1);
    assert!(starts.iter().all(lines_up), "{starts:?}");
}

#[test]
fn graph_compact_rows__on_a_line_too_short_for_the_three_columns_the_hash_and_the_author_leave_and_the_date_stays() {
    let repository = months_apart();
    let mut state = showing(repository.path().to_path_buf());
    let _ = compact(&mut state, true);
    let mut wide = Window::new(ThemeName::Dark, WIDE);
    let last = *timeline_rows(&wide.draw(&state, &[]).1).last().unwrap();
    assert_eq!(column_starts(&wide, last).len(), 3);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let last = *timeline_rows(&window.draw(&state, &[]).1).last().unwrap();
    let columns = column_starts(&window, last);
    assert_eq!(columns.len(), 1, "the date alone: {columns:?}");
    assert!(
        f64::from(columns[0]) > f64::from(last.x + last.width * 0.5),
        "{columns:?}"
    );
}

#[test]
fn graph_compact_rows__on_the_draft_and_a_stash_are_rows_as_tall_as_those_of_the_commits() {
    let repository = months_apart();
    let mut state = showing(repository.path().to_path_buf());
    let _ = compact(&mut state, true);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = window.draw(&state, &[]).1;
    let rows = timeline_rows(&tree);
    assert_eq!(rows.len(), 5, "the draft, the stash and three commits");
    assert!(rows.iter().all(|row| (row.height - 26.0).abs() < 0.5), "{rows:?}");
    // The draft keeps its buttons on its one line.
    let draft = rows[0];
    for name in ["Review", "Commit"] {
        let button = bounds(&tree, Role::Button, name);
        assert!(
            button.y >= draft.y && button.bottom() <= draft.bottom(),
            "{name}: {button:?} {draft:?}"
        );
    }
}

#[test]
fn graph_compact_rows__unchecking_a_column_takes_it_out_and_leaves_the_others() {
    let repository = months_apart();
    let mut state = showing(repository.path().to_path_buf());
    let _ = compact(&mut state, true);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = window.draw(&state, &[]).1;
    let all = window.frame.to_rgba();
    let last = *timeline_rows(&tree).last().unwrap();
    let columns = column_starts(&window, last);
    assert_eq!(columns.len(), 3, "the hash, the date and the author of a short message");
    for column in [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash] {
        let switch = GraphIntent::Switch(column, false);
        let _effects = run(&mut state, Message::Intent(Intent::Graph(switch)));
        let _ = window.draw(&state, &[]);
        assert!(window.frame.to_rgba() != all, "{column:?} was drawn");
        assert_eq!(column_starts(&window, last).len(), 2, "without {column:?}");
        let switch = GraphIntent::Switch(column, true);
        let _effects = run(&mut state, Message::Intent(Intent::Graph(switch)));
    }
    let _ = window.draw(&state, &[]);
    assert!(window.frame.to_rgba() == all, "all three are back");
}

#[test]
fn graph_compact_rows__on_the_labels_come_first_and_resting_on_plus_n_lists_them() {
    let (repository, _remote) = six_labels();
    let mut state = showing(repository.path().to_path_buf());
    let _ = compact(&mut state, true);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = window.draw(&state, &[]).1;
    let row = timeline_rows(&tree)[0];
    let labels = bounds(&tree, Role::ListBox, "Labels");
    assert!(labels.x - row.x < 0.25 * row.width, "{labels:?} {row:?}");
    let count = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Label)
        .filter_map(|(_, node)| node.label().or(node.value()))
        .find(|text| text.starts_with('+'))
        .map(str::to_owned)
        .unwrap();
    let count = said_at(&tree, &count, row);
    let mut intents = window.draw(&state, &[moved(count.center())]).0;
    window.now += Duration::from_millis(400);
    intents.extend(window.draw(&state, &[]).0);
    apply(&mut state, intents);
    let tree = window.draw(&state, &[]).1;
    let list = bounds(&tree, Role::Dialog, "Labels");
    assert!(list.y >= count.bottom(), "{list:?} {count:?}");
}

#[test]
fn graph_compact_rows__the_switch_is_in_the_menu_of_the_header_checked_while_it_is_on() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let mut window = Window::new(ThemeName::Dark, (1000, 900));
    let _ = window.draw(&state, &[]);
    let _ = window.click(&state, Role::Button, "View");
    let intents = window.click(&state, Role::MenuItem, "Compact rows");
    assert_eq!(
        intents,
        [Intent::Graph(GraphIntent::Switch(TimelineSwitch::CompactRows, true))]
    );
    apply(&mut state, intents);
    assert!(state.settings.switches.is_on(TimelineSwitch::CompactRows));
    let _ = window.click(&state, Role::Button, "View");
    let intents = window.click(&state, Role::MenuItem, "Compact rows");
    assert_eq!(
        intents,
        [Intent::Graph(GraphIntent::Switch(TimelineSwitch::CompactRows, false))]
    );
}

#[test]
fn graph_compact_rows__the_row_at_the_top_stays_there_across_the_switch_and_the_commit_selected_stays_in_sight() {
    let repository = imported(&line(60));
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = window.draw(&state, &[]).1;
    let history = bounds(&tree, Role::ListBox, "History");
    let _ = wheel(&mut window, &state, history.center(), 6.0);
    let tree = window.draw(&state, &[]).1;
    let top = timeline_rows(&tree)[0];
    let third = timeline_rows(&tree)[2];
    let _ = compact(&mut state, true);
    let tree = window.draw(&state, &[]).1;
    let rows = timeline_rows(&tree);
    assert!(
        (rows[0].y - history.y).abs() <= (top.y - history.y).abs() + 0.5,
        "{rows:?} {top:?}"
    );
    // The commit at the third row in sight, selected with a click, is in sight on its lower row, and still in sight
    // when the rows are tall again; then the last commit in sight of the lower rows, which the tall rows would leave
    // under the list.
    let _ = compact(&mut state, false);
    let _ = window.draw(&state, &[]);
    let intents = super::pointer::click_at(&mut window, &state, third.center(), 1);
    apply(&mut state, intents);
    assert!(matches!(state.timeline.selected, Some(Selected::Commit(_))));
    let _ = compact(&mut state, true);
    assert!(selected_in_sight(&window.draw(&state, &[]).1));
    let tree = window.draw(&state, &[]).1;
    let last = *timeline_rows(&tree)
        .iter()
        .rfind(|row| row.bottom() <= history.bottom())
        .unwrap();
    let intents = super::pointer::click_at(&mut window, &state, last.center(), 1);
    apply(&mut state, intents);
    assert!(selected_in_sight(&window.draw(&state, &[]).1));
    let _ = compact(&mut state, false);
    assert!(
        selected_in_sight(&window.draw(&state, &[]).1),
        "the commit selected was in sight, and stays in sight on the taller rows"
    );
}
