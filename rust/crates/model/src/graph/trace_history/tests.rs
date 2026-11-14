#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{CellTrace, Trace, trace_history};
use crate::graph::lanes::{GraphCell, Half, lanes};
use crate::graph::row::{GraphInput, Row};
use crate::graph::testing::{id, rows};

/// The rows, their cells, and the trace of what is wanted.
fn traced(lines: &[&str], wanted: &Trace) -> (Vec<GraphCell>, Vec<CellTrace>) {
    let rows = rows(lines);
    let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
    let cells = lanes(&inputs);
    let traces = trace_history(&rows, &cells, wanted);
    (cells, traces)
}

/// Whether each row's dot is in the history.
fn dots(traces: &[CellTrace]) -> Vec<bool> {
    traces.iter().map(|trace| trace.is_in).collect()
}

#[test]
fn graph_trace_history__what_a_commit_reaches_leaves_out_a_line_from_another_tip() {
    let (cells, traces) = traced(&["A: B", "B: C", "X: C", "C:"], &Trace::Reaches(id("A")));
    assert_eq!(dots(&traces), [true, true, false, true]);
    assert!(traces[0].lines_in.iter().all(|line_in| *line_in));
    assert!(
        traces[1]
            .lines_in
            .iter()
            .zip(&cells[1].lines)
            .all(|(line_in, line)| *line_in == (line.from == 0))
    );
    assert!(
        traces[2]
            .lines_in
            .iter()
            .zip(&cells[2].lines)
            .all(|(line_in, line)| *line_in == (line.from == 0))
    );
    for (line, line_in) in cells[3].lines.iter().zip(&traces[3].lines_in) {
        assert_eq!(
            *line_in,
            line.from == 0,
            "the line from X into C is not in it: {line:?}"
        );
    }
}

#[test]
fn graph_trace_history__what_a_branch_has_that_another_has_not_is_its_own_commits() {
    let lines = ["F2: F1", "M2: M1", "F1: M1", "M1: M0", "M0:"];
    let wanted = Trace::HasNot {
        commit: id("F2"),
        other: id("M2"),
    };
    let (cells, traces) = traced(&lines, &wanted);
    assert_eq!(dots(&traces), [true, false, true, false, false]);
    let into_base = cells[2]
        .lines
        .iter()
        .zip(&traces[2].lines_in)
        .find(|(line, _)| line.half == Half::Lower && line.from == cells[2].lane);
    assert_eq!(into_base.map(|(_, line_in)| *line_in), Some(false));
}

#[test]
fn graph_trace_history__what_a_commit_reaches_together_with_what_reaches_it() {
    let (_, traces) = traced(&["A: B", "X: C", "B: C", "C:"], &Trace::Around(id("B")));
    assert_eq!(dots(&traces), [true, false, true, true]);
}

#[test]
fn graph_trace_history__nothing_is_known_against_a_commit_not_among_the_rows() {
    let wanted = Trace::HasNot {
        commit: id("A"),
        other: id("Z"),
    };
    let (_, traces) = traced(&["A: B", "B:"], &wanted);
    assert_eq!(dots(&traces), [false, false]);
    assert!(traces.iter().flat_map(|trace| &trace.lines_in).all(|line_in| !line_in));
}

#[test]
fn graph_trace_history__a_lane_two_lines_run_into_belongs_to_it_if_either_does() {
    // B goes down to A in lane 0, and the merge M joins that lane on its way to A: the lane is in the history of B,
    // whatever M is, and the line of M into it is not.
    let (cells, traces) = traced(&["B: A", "M: X A", "X: A", "A:"], &Trace::Reaches(id("B")));
    assert_eq!(dots(&traces), [true, false, false, true]);
    for (line, line_in) in cells[1].lines.iter().zip(&traces[1].lines_in) {
        assert_eq!(*line_in, line.from == 0 && line.to == 0, "{line:?}");
    }
    for (line, line_in) in cells[2].lines.iter().zip(&traces[2].lines_in) {
        assert_eq!(*line_in, line.from == 0, "{line:?}");
    }
}
