//! Which commits and which parts of the lines belong to the history of a commit, for the graph to bring them out.

use crate::CommitId;
use crate::graph::lanes::{GraphCell, Half};
use crate::graph::reach::Ancestry;
use crate::graph::row::Row;

/// The history wanted, among the rows loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trace {
    /// What a commit reaches: itself, its parents, and theirs.
    Reaches(CommitId),
    /// What a commit has that another has not: what the first reaches less what the second reaches.
    HasNot {
        /// The commit whose history is wanted.
        commit: CommitId,
        /// The commit whose history is taken away from it.
        other: CommitId,
    },
    /// What a commit reaches, together with what reaches it: the commits made on top of it.
    Around(CommitId),
}

/// What of one row belongs to the history.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellTrace {
    /// Whether its dot does.
    pub is_in: bool,
    /// Whether each line of its cell does, in the order of the cell's lines.
    pub lines_in: Vec<bool>,
}

/// For each row, whether its dot and each line of its cell belong to the history wanted. A line belongs to it when the
/// commit it comes down from does and so does the parent it goes to; where two lines run into one lane, the lane
/// belongs to it if either does. What a commit has that another has not is none when the other is not among the rows.
///
/// Feature: graph.trace-history
#[must_use]
pub fn trace_history(rows: &[Row], cells: &[GraphCell], wanted: &Trace) -> Vec<CellTrace> {
    let ancestry = Ancestry::new(rows);
    let rows_in = rows_in(&ancestry, wanted);
    let targets = line_targets(cells);
    let mut top: Vec<bool> = Vec::new();
    let mut traces = Vec::with_capacity(cells.len());
    for (index, (cell, targets)) in cells.iter().zip(&targets).enumerate() {
        let is_in = rows_in.get(index).copied().unwrap_or(false);
        let mut bottom: Vec<bool> = Vec::with_capacity(top.len());
        let mut lines_in = Vec::with_capacity(cell.lines.len());
        for (line, target) in cell.lines.iter().zip(targets) {
            let from_lane_in = top.get(line.from).copied().unwrap_or(false);
            let source_in = match line.half {
                Half::Lower if line.from == cell.lane => is_in,
                Half::Lower | Half::Upper => from_lane_in,
            };
            let target_in = target.and_then(|row| rows_in.get(row).copied()).unwrap_or(false);
            lines_in.push(source_in && target_in);
            if line.half == Half::Lower {
                if bottom.len() <= line.to {
                    bottom.resize(line.to + 1, false);
                }
                if let Some(lane) = bottom.get_mut(line.to) {
                    *lane = *lane || source_in;
                }
            }
        }
        traces.push(CellTrace { is_in, lines_in });
        top = bottom;
    }
    traces
}

/// For each row, whether its commit belongs to the history wanted.
fn rows_in(ancestry: &Ancestry<'_>, wanted: &Trace) -> Vec<bool> {
    let none = || vec![false; ancestry.len()];
    match wanted {
        Trace::Reaches(commit) => ancestry
            .row_of(commit)
            .map_or_else(none, |row| ancestry.reached_from(&[row])),
        Trace::HasNot { commit, other } => match (ancestry.row_of(commit), ancestry.row_of(other)) {
            (Some(row), Some(other)) => {
                let mut wanted = ancestry.reached_from(&[row]);
                for (row_in, taken) in wanted.iter_mut().zip(ancestry.reached_from(&[other])) {
                    *row_in = *row_in && !taken;
                }
                wanted
            }
            _ => none(),
        },
        Trace::Around(commit) => ancestry.row_of(commit).map_or_else(none, |row| {
            let mut wanted = ancestry.reached_from(&[row]);
            for (row_in, reaching) in wanted.iter_mut().zip(ancestry.reaching(row)) {
                *row_in = *row_in || reaching;
            }
            wanted
        }),
    }
}

/// For each line of each cell, the row of the commit it goes to; none when that commit is further down than the rows.
/// A line into the dot goes to the row's commit; any other line goes on in its lane to the commit the lane waits for,
/// which the rows under it say.
fn line_targets(cells: &[GraphCell]) -> Vec<Vec<Option<usize>>> {
    let mut targets = vec![Vec::new(); cells.len()];
    let mut below: Vec<Option<usize>> = Vec::new();
    for (index, cell) in cells.iter().enumerate().rev() {
        let mut above: Vec<Option<usize>> = Vec::new();
        let row_targets: Vec<Option<usize>> = cell
            .lines
            .iter()
            .map(|line| {
                let target = match line.half {
                    Half::Upper if line.to == cell.lane => Some(index),
                    Half::Upper | Half::Lower => below.get(line.to).copied().flatten(),
                };
                if line.half == Half::Upper {
                    if above.len() <= line.from {
                        above.resize(line.from + 1, None);
                    }
                    if let Some(lane) = above.get_mut(line.from) {
                        *lane = target;
                    }
                }
                target
            })
            .collect();
        if let Some(slot) = targets.get_mut(index) {
            *slot = row_targets;
        }
        below = above;
    }
    targets
}

#[cfg(test)]
mod tests;
