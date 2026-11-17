//! The keys, the pages asked for, the label under the pointer, the marks and the sticky names.

use std::time::Duration;

use model::graph::Row;
use ui::{Modifiers, NamedKey};

use super::super::hover::{HoverState, hover_intent};
use super::super::intent::{GraphIntent, HoverTarget, HoverWay, Selected};
use super::super::keyboard::key_intent;
use super::super::load_more::{LoadMoreState, wants_more};
use super::super::marks::mark_positions;
use super::super::sticky::sticky_names;
use super::{Fixture, feature_and_main, id, key, repository, rows, select};

#[test]
fn graph_keyboard__home_selects_the_commit_checked_out_from_anywhere_in_the_list() {
    let fixture = Fixture::new(
        rows(&["E: D", "D: C", "C: B", "B: A", "A:"]),
        &repository("C", &[("main", "C")]),
        Some(Selected::Commit(id("A"))),
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::Home, Modifiers::NONE)),
        Some(select("C"))
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowDown, Modifiers::NONE)),
        None
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowUp, Modifiers::NONE)),
        Some(select("B"))
    );
}

#[test]
fn graph_keyboard__alt_down_on_a_merge_selects_its_first_parent_not_the_commit_under_it() {
    let fixture = Fixture::new(
        rows(&["M: B F", "F: A", "B: A", "A:"]),
        &repository("M", &[("main", "M")]),
        Some(Selected::Commit(id("M"))),
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowDown, Modifiers::ALT)),
        Some(select("B"))
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowDown, Modifiers::NONE)),
        Some(select("F"))
    );
}

#[test]
fn graph_keyboard__alt_up_on_the_last_commit_of_a_merged_branch_selects_the_merge() {
    let fixture = Fixture::new(
        rows(&["N: M", "M: B F2", "F2: F1", "B: A", "F1: A", "A:"]),
        &repository("N", &[("main", "N")]),
        Some(Selected::Commit(id("F2"))),
    );
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowUp, Modifiers::ALT)),
        Some(select("M"))
    );
    let on_its_line = Fixture::new(
        fixture.rows.clone(),
        &repository("N", &[]),
        Some(Selected::Commit(id("F1"))),
    );
    assert_eq!(
        key_intent(&on_its_line.history(), &key(NamedKey::ArrowUp, Modifiers::ALT)),
        Some(select("F2"))
    );
}

#[test]
fn graph_keyboard__shift_alt_down_selects_the_next_merge_down() {
    let fixture = Fixture::new(
        rows(&["D: M2", "M2: M1 Y", "Y: B", "M1: C X", "X: B", "C: B", "B: A", "A:"]),
        &repository("D", &[("main", "D")]),
        Some(Selected::Commit(id("M2"))),
    );
    let shift_alt = Modifiers::SHIFT.with(Modifiers::ALT);
    assert_eq!(
        key_intent(&fixture.history(), &key(NamedKey::ArrowDown, shift_alt)),
        Some(select("M1"))
    );
    assert_eq!(key_intent(&fixture.history(), &key(NamedKey::ArrowUp, shift_alt)), None);
}

#[test]
fn graph_keyboard__enter_on_the_draft_opens_nothing() {
    let mut all = vec![Row::Uncommitted {
        parent: Some(id("B")),
        files: 1,
    }];
    all.extend(rows(&["B: A", "A:"]));
    let draft = Fixture::new(all.clone(), &repository("B", &[("main", "B")]), Some(Selected::Draft));
    assert_eq!(
        key_intent(&draft.history(), &key(NamedKey::Enter, Modifiers::NONE)),
        None
    );
    let commit = Fixture::new(all, &repository("B", &[("main", "B")]), Some(Selected::Commit(id("B"))));
    assert_eq!(
        key_intent(&commit.history(), &key(NamedKey::Enter, Modifiers::NONE)),
        Some(GraphIntent::Open(id("B")))
    );
    assert_eq!(
        key_intent(&commit.history(), &key(NamedKey::ArrowUp, Modifiers::NONE)),
        Some(GraphIntent::Select(Selected::Draft))
    );
}

/// Where the list stands with some commits loaded and the last one drawn.
fn standing(loaded: usize, last_drawn: usize) -> LoadMoreState {
    LoadMoreState {
        loaded,
        last_drawn: Some(last_drawn),
        has_more: true,
        is_reading: false,
        asked_at: None,
    }
}

#[test]
fn graph_load_more__with_100000_commits_loaded_and_more_row_99500_starts_the_next_read_and_row_99499_not_yet() {
    assert!(wants_more(&standing(100_000, 99_500)));
    assert!(!wants_more(&standing(100_000, 99_499)));
    let at_the_end = LoadMoreState {
        has_more: false,
        ..standing(100_000, 99_999)
    };
    assert!(!wants_more(&at_the_end));
}

#[test]
fn graph_load_more__scrolling_on_while_the_read_runs_does_not_start_it_again() {
    let reading = LoadMoreState {
        is_reading: true,
        ..standing(100_000, 99_700)
    };
    assert!(!wants_more(&reading));
    let asked = LoadMoreState {
        asked_at: Some(100_000),
        ..standing(100_000, 99_800)
    };
    assert!(!wants_more(&asked));
}

#[test]
fn graph_load_more__with_pages_of_100_scrolling_to_the_50th_row_loads_pages_until_600_commits_are_loaded() {
    let mut loaded = 100;
    while wants_more(&standing(loaded, 50)) {
        loaded += 100;
    }
    assert_eq!(loaded, 600);
}

#[test]
fn graph_hover_history__resting_150_ms_on_a_label_brings_out_its_history() {
    let target = HoverTarget {
        commit: id("F"),
        way: HoverWay::Reaches,
    };
    let at = |now: u64| HoverState {
        current: None,
        left_at: None,
        under_pointer: Some((target.clone(), Duration::from_millis(1_000))),
        now: Duration::from_millis(now),
    };
    assert_eq!(hover_intent(&at(1_149)), (None, Some(Duration::from_millis(1_150))));
    assert_eq!(
        hover_intent(&at(1_150)).0,
        Some(GraphIntent::Hover(Some(target.clone())))
    );
}

#[test]
fn graph_hover_history__moving_the_pointer_across_labels_quickly_does_not_fade_the_timeline() {
    for (name, since) in [("F", 1_000), ("G", 1_060), ("H", 1_120)] {
        let state = HoverState {
            current: None,
            left_at: None,
            under_pointer: Some((
                HoverTarget {
                    commit: id(name),
                    way: HoverWay::Reaches,
                },
                Duration::from_millis(since),
            )),
            now: Duration::from_millis(since + 60),
        };
        assert_eq!(hover_intent(&state).0, None, "{name}");
    }
    let shown = HoverTarget {
        commit: id("F"),
        way: HoverWay::Reaches,
    };
    let left = |now| HoverState {
        current: Some(&shown),
        left_at: Some(Duration::from_millis(2_000)),
        under_pointer: None,
        now: Duration::from_millis(now),
    };
    assert_eq!(hover_intent(&left(2_050)).0, None);
    assert_eq!(hover_intent(&left(2_100)).0, Some(GraphIntent::Hover(None)));
}

#[test]
fn graph_scroll_marks__with_1000_commits_loaded_and_the_one_checked_out_at_row_0_its_mark_is_at_the_top() {
    let marks = mark_positions(&[0], 1_001, 500.0);
    assert_eq!(marks.len(), 1);
    assert!(marks[0] < 1.0, "{marks:?}");
}

#[test]
fn graph_scroll_marks__fourteen_matches_show_their_marks_merged_where_they_fall_on_the_same_pixel() {
    let found: Vec<usize> = (0..7).chain(5_000..5_007).collect();
    let marks = mark_positions(&found, 50_000, 500.0);
    assert_eq!(marks.len(), 2, "{marks:?}");
    let apart: Vec<usize> = (0..14).map(|index| index * 1_000).collect();
    assert_eq!(mark_positions(&apart, 14_000, 500.0).len(), 14);
}

#[test]
fn graph_scroll_marks__selecting_a_commit_moves_the_mark_of_the_selection_to_it() {
    let all = rows(&["C: B", "B: A", "A:"]);
    let first = Fixture::new(all.clone(), &repository("C", &[]), Some(Selected::Commit(id("C"))));
    let second = Fixture::new(all, &repository("C", &[]), Some(Selected::Commit(id("A"))));
    assert_eq!(super::super::row::selected_row(&first.history()), Some(0));
    assert_eq!(super::super::row::selected_row(&second.history()), Some(2));
    assert!(mark_positions(&[2], 3, 300.0)[0] > mark_positions(&[0], 3, 300.0)[0]);
}

#[test]
fn graph_sticky_branch_names__scrolled_past_the_tip_of_feature_while_its_line_goes_on_it_stays_at_the_top() {
    let fixture = feature_and_main();
    let names = sticky_names(&fixture.layout, &fixture.rows, 3);
    let words: Vec<&str> = names.names.iter().map(|name| name.name).collect();
    assert_eq!(words, ["feature", "main"]);
    let feature = names.names.first().unwrap_or_else(|| panic!("no name"));
    assert_eq!(feature.commit, &id("F2"));
    assert_eq!(Some(feature.color), fixture.layout.cells.first().map(|cell| cell.color));
}

#[test]
fn graph_sticky_branch_names__at_the_top_of_the_list_there_is_no_sticky_name() {
    let fixture = feature_and_main();
    assert_eq!(sticky_names(&fixture.layout, &fixture.rows, 0).names, []);
}
