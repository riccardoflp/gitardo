//! Every commit of the timeline on one line, without lanes (`graph.one-lane`).

use crate::graph::lanes::{CellLine, GraphCell, Half};
use crate::graph::row::GraphInput;

/// Lays out the rows on one line: for each row, its dot in the first lane, in the color of the first lane, a line from
/// the top edge to the dot when a row is above it and from the dot to the bottom edge when a row is below it. The line
/// between two rows is dashed when the upper one's is, as the line of the uncommitted changes and of a stash are.
///
/// Feature: graph.one-lane
#[must_use]
pub fn one_lane(rows: &[GraphInput<'_>]) -> Vec<GraphCell> {
    one_lane_after(None, rows)
}

/// Lays out on one line rows added under others laid out so, as [`one_lane`] lays them out with the rows above:
/// `above` is the cell of the last row above them, and whether that row's line is dashed. Gives it the line down to
/// them, which it did not have while no row was under it.
#[must_use]
pub fn one_lane_after(above: Option<(&mut GraphCell, bool)>, rows: &[GraphInput<'_>]) -> Vec<GraphCell> {
    let mut cells = laid_out(rows);
    if let (Some((cell, is_dashed)), Some(first)) = (above, cells.first_mut()) {
        if !cell.lines.iter().any(|line| line.half == Half::Lower) {
            cell.lines.push(line(Half::Lower, is_dashed));
        }
        first.lines.insert(0, line(Half::Upper, is_dashed));
    }
    cells
}

/// The line of the one lane in one half of a row.
fn line(half: Half, is_dashed: bool) -> CellLine {
    CellLine {
        half,
        from: 0,
        to: 0,
        color: 0,
        is_dashed,
    }
}

/// The rows laid out on one line, the first with no line up and the last with no line down.
fn laid_out(rows: &[GraphInput<'_>]) -> Vec<GraphCell> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let above = index.checked_sub(1).and_then(|above| rows.get(above));
            let mut lines = Vec::with_capacity(2);
            if let Some(above) = above {
                lines.push(line(Half::Upper, above.is_dashed));
            }
            if index + 1 < rows.len() {
                lines.push(line(Half::Lower, row.is_dashed));
            }
            GraphCell {
                lane: 0,
                color: 0,
                lines,
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{one_lane, one_lane_after};
    use crate::graph::row::{GraphInput, Row};
    use crate::graph::testing::rows;
    use crate::graph::{GraphCell, Half, lanes};

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

    #[test]
    fn graph_one_lane__on_every_row_of_a_history_with_three_branches_and_two_merges_has_its_dot_in_the_first_lane() {
        let rows = three_branches_two_merges();
        let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
        assert!(
            lanes(&inputs).iter().any(|cell| cell.lane > 0),
            "with its lanes, the history has more than one"
        );
        let cells = one_lane(&inputs);
        assert_eq!(cells.len(), rows.len());
        assert!(cells.iter().all(|cell| cell.lane == 0 && cell.color == 0), "{cells:?}");
        assert!(cells.iter().all(|cell| cell.lane_count() == 1));
        let halves = |cell: &GraphCell| cell.lines.iter().map(|line| line.half).collect::<Vec<_>>();
        assert_eq!(cells.first().map(halves), Some(vec![Half::Lower]));
        assert_eq!(cells.get(4).map(halves), Some(vec![Half::Upper, Half::Lower]));
        assert_eq!(cells.last().map(halves), Some(vec![Half::Upper]));
    }

    #[test]
    fn graph_one_lane__on_rows_laid_out_under_others_are_laid_out_as_all_of_them_at_once() {
        let rows = three_branches_two_merges();
        let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
        for cut in 1..inputs.len() {
            let (above, under) = inputs.split_at(cut);
            let mut cells = one_lane(above);
            let is_dashed = above.last().is_some_and(|row| row.is_dashed);
            let added = one_lane_after(cells.last_mut().map(|cell| (cell, is_dashed)), under);
            cells.extend(added);
            assert_eq!(cells, one_lane(&inputs), "cut before row {cut}");
        }
    }
}
