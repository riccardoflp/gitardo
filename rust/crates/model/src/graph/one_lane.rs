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
    let line = |half: Half, is_dashed: bool| CellLine {
        half,
        from: 0,
        to: 0,
        color: 0,
        is_dashed,
    };
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
    use super::one_lane;
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
}
