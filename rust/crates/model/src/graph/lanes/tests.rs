#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{CellLine, GraphCell, Half, lanes};
use crate::graph::row::{GraphInput, Row};
use crate::graph::testing::{commit_at, rows};

/// The cells of rows, with the lines of each cell in a fixed order.
fn cells_of(rows: &[Row]) -> Vec<GraphCell> {
    let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
    lanes(&inputs)
        .into_iter()
        .map(|mut cell| {
            cell.lines.sort();
            cell
        })
        .collect()
}

fn cell(lane: usize, color: usize, lines: &[CellLine]) -> GraphCell {
    let mut lines = lines.to_vec();
    lines.sort();
    GraphCell { lane, color, lines }
}

fn up(from: usize, to: usize, color: usize) -> CellLine {
    CellLine {
        half: Half::Upper,
        from,
        to,
        color,
        is_dashed: false,
    }
}

fn down(from: usize, to: usize, color: usize) -> CellLine {
    CellLine {
        half: Half::Lower,
        from,
        to,
        color,
        is_dashed: false,
    }
}

fn dashed(line: CellLine) -> CellLine {
    CellLine {
        is_dashed: true,
        ..line
    }
}

#[test]
fn graph_lanes__a_line_of_three_commits_is_one_lane_of_one_color() {
    let cells = cells_of(&rows(&["C: B", "B: A", "A:"]));
    assert_eq!(
        cells,
        [
            cell(0, 0, &[down(0, 0, 0)]),
            cell(0, 0, &[up(0, 0, 0), down(0, 0, 0)]),
            cell(0, 0, &[up(0, 0, 0)]),
        ]
    );
}

#[test]
fn graph_lanes__a_tip_on_a_commit_another_lane_waits_for_ends_in_its_lane() {
    let cells = cells_of(&rows(&["F2: F1", "F1: M1", "M2: M1", "M1:"]));
    assert_eq!(
        cells,
        [
            cell(0, 0, &[down(0, 0, 0)]),
            cell(0, 0, &[up(0, 0, 0), down(0, 0, 0)]),
            cell(1, 1, &[up(0, 0, 0), down(0, 0, 0), down(1, 1, 1)]),
            cell(0, 0, &[up(0, 0, 0), up(1, 0, 1)]),
        ]
    );
}

#[test]
fn graph_lanes__a_merge_opens_a_lane_for_its_second_parent_and_it_is_dropped_after() {
    let cells = cells_of(&rows(&["M: A B", "B: A", "A:"]));
    assert_eq!(
        cells,
        [
            cell(0, 0, &[down(0, 0, 0), down(0, 1, 1)]),
            cell(1, 1, &[up(0, 0, 0), down(0, 0, 0), up(1, 1, 1), down(1, 1, 1)]),
            cell(0, 0, &[up(0, 0, 0), up(1, 0, 1)]),
        ]
    );
    assert_eq!(cells[2].lane_count(), 2);
}

#[test]
fn graph_lanes__the_uncommitted_changes_have_a_dashed_line_down_to_their_commit() {
    let changes = Row::Uncommitted {
        parent: Some(crate::graph::testing::id("C")),
        files: 1,
    };
    let mut all = vec![changes];
    all.extend(rows(&["C:"]));
    let cells = cells_of(&all);
    assert_eq!(
        cells,
        [cell(0, 0, &[dashed(down(0, 0, 0))]), cell(0, 0, &[dashed(up(0, 0, 0))])]
    );
}

#[test]
fn graph_lanes__a_lane_left_free_by_a_merged_branch_goes_to_the_next_tip_with_a_new_color() {
    let cells = cells_of(&rows(&["M: A B", "B: A", "A: R", "T: R", "R:"]));
    assert_eq!(cells[1].lane, 1);
    assert_eq!(cells[3].lane, 1);
    assert_eq!(cells[3].color, 2);
    assert_eq!(cells[4], cell(0, 0, &[up(0, 0, 0), up(1, 0, 2)]));
}

#[test]
fn graph_lanes__twenty_branches_open_at_once_take_twenty_lanes_then_one() {
    let mut lines: Vec<String> = (1..=20).map(|branch| format!("T{branch}: R")).collect();
    lines.extend([String::from("R: Q"), String::from("Q:")]);
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let cells = cells_of(&rows(&lines));
    let widest = cells.iter().map(GraphCell::lane_count).max().unwrap();
    assert_eq!(widest, 20);
    assert_eq!(cells[19].lane, 19);
    assert_eq!(cells[20].lane, 0);
    assert_eq!(cells[21], cell(0, 0, &[up(0, 0, 0)]));
}

#[test]
fn graph_lanes__a_commit_waited_for_by_a_stash_and_the_history_goes_on_the_history_lane() {
    let stash = Row::Stash {
        commit: commit_at("S", &["X"], 300),
        index: 0,
    };
    let mut all = vec![stash];
    all.extend(rows(&["Y: X", "X:"]));
    let cells = cells_of(&all);
    assert_eq!(cells[0], cell(0, 0, &[dashed(down(0, 0, 0))]));
    assert_eq!(cells[2], cell(1, 1, &[dashed(up(0, 1, 0)), up(1, 1, 1)]));
}

#[test]
fn graph_lanes__rows_given_without_parents_have_no_line_down() {
    let cells = cells_of(&rows(&["A:", "B:", "C:"]));
    assert!(cells.iter().all(|cell| cell.lane == 0 && cell.lines.is_empty()));
    assert_eq!(cells.iter().map(|cell| cell.color).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn graph_lanes__a_second_parent_already_waited_for_joins_its_lane() {
    let cells = cells_of(&rows(&["B: A", "M: X A", "X: A", "A:"]));
    assert_eq!(
        cells[1],
        cell(1, 1, &[up(0, 0, 0), down(0, 0, 0), down(1, 1, 1), down(1, 0, 0)])
    );
}
