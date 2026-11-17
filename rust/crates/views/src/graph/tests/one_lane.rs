//! The timeline with every commit on one line (`graph.one-lane`): the dots of its rows and the width of its graph.

use std::collections::HashSet;

use model::graph::{Row, fit_width};

use super::super::cell::Dot;
use super::super::layout::{LayoutInput, TimelineLayout};
use super::super::row::dot_of;
use super::{repository, rows};

/// A history of three branches, `main`, `feature` and `fix`, both merged into `main`.
fn three_branches_two_merges() -> Vec<Row> {
    rows(&[
        "M2: M1 F2",
        "F2: F1",
        "M1: C X2",
        "X2: X1",
        "C: B",
        "X1: B",
        "F1: B",
        "B: A",
        "A:",
    ])
}

/// The timeline of the rows laid out, on one line or with its lanes.
fn laid_out(rows: &[Row], one_lane: bool) -> TimelineLayout {
    TimelineLayout::new(&LayoutInput {
        rows,
        repository: &repository("M2", &[("main", "M2"), ("feature", "F2"), ("fix", "X2")]),
        folded: &HashSet::new(),
        combine_upstream: true,
        separators_at: None,
        one_lane,
    })
}

#[test]
fn graph_one_lane__on_the_dot_of_a_merge_is_a_ring_and_the_dot_of_any_other_commit_is_filled() {
    let rows = three_branches_two_merges();
    let layout = laid_out(&rows, true);
    let dots: Vec<Dot> = layout
        .folded
        .rows
        .iter()
        .zip(&layout.read_of)
        .map(|(shown, read)| dot_of(shown, rows.get(*read)))
        .collect();
    let merges = [Dot::Merge, Dot::Commit, Dot::Merge];
    assert_eq!(dots.get(..3), Some(&merges[..]), "{dots:?}");
    assert!(
        dots.get(3..)
            .is_some_and(|rest| rest.iter().all(|dot| *dot == Dot::Commit)),
        "{dots:?}"
    );
    assert!(layout.cells.iter().all(|cell| cell.lane == 0));
}

#[test]
fn graph_one_lane__on_the_column_of_the_graph_is_as_wide_as_one_lane() {
    let rows = three_branches_two_merges();
    let with_lanes = laid_out(&rows, false);
    assert!(with_lanes.lane_count > 1);
    let layout = laid_out(&rows, true);
    assert_eq!(layout.lane_count, 1);
    let one_lane = fit_width(1, None, 1_200.0).width;
    assert!((fit_width(layout.lane_count, None, 1_200.0).width - one_lane).abs() < 0.5);
    let dragged = fit_width(layout.lane_count, Some(400.0), 1_200.0).width;
    assert!(
        (dragged - one_lane).abs() < 0.5,
        "dragged wider, it is still one lane: {dragged}"
    );
    assert!(fit_width(with_lanes.lane_count, None, 1_200.0).width > one_lane);
}
