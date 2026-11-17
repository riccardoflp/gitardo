//! The branch each line of the graph comes from: for the names that stay at the top of the list while scrolling.

use crate::graph::lanes::{GraphCell, Half};

/// For each row, and each lane a line comes down into it in, the row of the tip that line is the history of: the first
/// commit up that line that has a name (a branch, a remote branch, a tag, `HEAD` or a stash), or a commit nothing comes
/// down to. `named` says which rows have a name. A commit passes its own line's tip, or itself when it is named or
/// nothing comes down to it, to its first parent; a line to another parent of a merge has no tip until a named commit
/// is on it, since which branch was merged is not known. A lane with no line coming down has none.
///
/// Feature: graph.lane-tips
#[must_use]
pub fn lane_tips(cells: &[GraphCell], named: &[bool]) -> Vec<Vec<Option<usize>>> {
    lane_tips_after(Vec::new(), (cells, named), 0).0
}

/// The tips of the lines of rows laid out under others, as [`lane_tips`] gives them with the rows above: `bottom` is
/// the tip of each lane at the bottom edge of the row above them, and `first` the number of their first row. Gives
/// their tips, and the tip of each lane at the bottom edge of the last of them.
#[must_use]
pub fn lane_tips_after(
    mut bottom: Vec<Option<usize>>,
    (cells, named): (&[GraphCell], &[bool]),
    first: usize,
) -> (Vec<Vec<Option<usize>>>, Vec<Option<usize>>) {
    let mut tips = Vec::with_capacity(cells.len());
    for (index, cell) in cells.iter().enumerate() {
        let row = first + index;
        let top = std::mem::take(&mut bottom);
        let tip_at_top = |lane: usize| top.get(lane).copied().flatten();
        let mut coming: Vec<Option<usize>> = Vec::new();
        let mut has_line_in = false;
        for line in cell.lines.iter().filter(|line| line.half == Half::Upper) {
            set(&mut coming, line.from, tip_at_top(line.from));
            has_line_in = has_line_in || line.to == cell.lane;
        }
        let is_named = named.get(index).copied().unwrap_or(false);
        let passed = if is_named || !has_line_in {
            Some(row)
        } else {
            tip_at_top(cell.lane)
        };
        let lower = || cell.lines.iter().filter(|line| line.half == Half::Lower);
        let mut straight: Vec<bool> = Vec::new();
        for line in lower().filter(|line| line.from != cell.lane) {
            set(&mut bottom, line.to, tip_at_top(line.from));
            if straight.len() <= line.to {
                straight.resize(line.to + 1, false);
            }
            if let Some(is_straight) = straight.get_mut(line.to) {
                *is_straight = true;
            }
        }
        for line in lower().filter(|line| line.from == cell.lane) {
            if !straight.get(line.to).copied().unwrap_or(false) {
                set(&mut bottom, line.to, (line.to == cell.lane).then_some(passed).flatten());
            }
        }
        tips.push(coming);
    }
    (tips, bottom)
}

/// Sets the tip of a lane, adding lanes up to it.
fn set(lanes: &mut Vec<Option<usize>>, lane: usize, tip: Option<usize>) {
    if lanes.len() <= lane {
        lanes.resize(lane + 1, None);
    }
    if let Some(slot) = lanes.get_mut(lane) {
        *slot = tip;
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::lane_tips;
    use crate::graph::lanes::lanes;
    use crate::graph::row::{GraphInput, Row};
    use crate::graph::testing::rows;

    /// The tips of the lines coming into each row, from rows written `X: P Q` and which of them are named.
    fn tips(lines: &[&str], named: &[bool]) -> Vec<Vec<Option<usize>>> {
        let rows = rows(lines);
        let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
        lane_tips(&lanes(&inputs), named)
    }

    #[test]
    fn graph_lane_tips__every_row_under_a_tip_on_its_line_has_that_tip() {
        // `feature` at F2, `main` at M2: F2 F1 on lane 0 down to the base B, M2 M1 on lane 1.
        let lines = ["F2: F1", "M2: M1", "F1: B", "M1: B", "B: A", "A:"];
        let named = [true, true, false, false, false, false];
        let found = tips(&lines, &named);
        assert_eq!(found[0], []);
        assert_eq!(found[1], [Some(0)]);
        assert_eq!(found[2], [Some(0), Some(1)]);
        assert_eq!(found[3], [Some(0), Some(1)]);
        assert_eq!(found[4], [Some(0), Some(1)]);
        assert_eq!(found[5], [Some(0)]);
    }

    #[test]
    fn graph_lane_tips__the_line_of_a_merged_branch_has_no_tip_until_a_named_commit() {
        // `main` at M merges X, which has no label, then T, which has one, is on that line.
        let lines = ["M: B X", "X: T", "T: B", "B:"];
        let found = tips(&lines, &[true, false, true, false]);
        assert_eq!(found[1], [Some(0), None]);
        assert_eq!(found[2], [Some(0), None]);
        assert_eq!(found[3], [Some(0), Some(2)]);
    }

    #[test]
    fn graph_lane_tips__a_commit_nothing_comes_down_to_is_a_tip_of_its_own() {
        let found = tips(&["C: B", "B: A", "A:"], &[false, false, false]);
        assert_eq!(found, [vec![], vec![Some(0)], vec![Some(0)]]);
    }
}
