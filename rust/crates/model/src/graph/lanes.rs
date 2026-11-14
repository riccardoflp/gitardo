//! Where each commit's dot goes in the graph, and the lines between commits, row by row.

use std::collections::HashMap;

use crate::CommitId;
use crate::graph::row::GraphInput;

/// How many colors the lines are drawn in: color `n` is drawn in the color `n % PALETTE_SIZE` of the palette.
pub const PALETTE_SIZE: usize = 8;

/// The half of a row a line is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Half {
    /// From a lane at the top edge of the row to a lane at its middle.
    Upper,
    /// From a lane at the middle of the row to a lane at its bottom edge.
    Lower,
}

/// One line of a cell of the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellLine {
    /// The half of the row it is drawn in.
    pub half: Half,
    /// The lane it starts from: at the top edge for the upper half, at the middle for the lower half.
    pub from: usize,
    /// The lane it goes to: at the middle for the upper half, at the bottom edge for the lower half.
    pub to: usize,
    /// Its color, numbered in the order lanes got them.
    pub color: usize,
    /// Whether it is dashed: a line of the uncommitted changes or of a stash.
    pub is_dashed: bool,
}

/// The cell of the graph of one row: its dot and the lines through it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphCell {
    /// The lane of its dot.
    pub lane: usize,
    /// The color of its dot, numbered in the order lanes got them.
    pub color: usize,
    /// The lines of the cell.
    pub lines: Vec<CellLine>,
}

impl GraphCell {
    /// How many lanes the cell spans: one more than the rightmost lane its dot or a line is in.
    #[must_use]
    pub fn lane_count(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.from.max(line.to))
            .fold(self.lane, usize::max)
            + 1
    }
}

/// Lays out the lines of the graph: for each row, in the order given (newest first, each commit before its parents),
/// the lane and color of its dot and the lines of its cell.
///
/// A commit takes the lane that waits for it (the leftmost one that is not dashed when several do), or, when none does,
/// the leftmost free lane with a new color. The other lanes that wait for it end at its dot. Its first parent goes on
/// in its own lane, even when another lane already waits for it, so that a branch is seen to leave its base; each
/// other parent goes to the lane that waits for it, or to a new one. Free lanes at the right end are dropped.
///
/// Feature: graph.lanes
#[must_use]
pub fn lanes(rows: &[GraphInput<'_>]) -> Vec<GraphCell> {
    let mut layout = Layout {
        numbers: HashMap::with_capacity(rows.len() + rows.len() / 4),
        ..Layout::default()
    };
    rows.iter().map(|row| layout.cell(row)).collect()
}

/// A lane that waits for a commit further down.
#[derive(Debug, Clone, Copy)]
struct Waiting {
    /// The number the layout gave the commit it waits for.
    commit: usize,
    /// The color of the lane, which it keeps while it waits.
    color: usize,
    /// Whether its line is dashed.
    is_dashed: bool,
}

/// The lanes as they are between two rows, while the rows are laid out one after the other.
#[derive(Debug, Default)]
struct Layout<'rows> {
    /// Each lane, free or waiting for a commit.
    lanes: Vec<Option<Waiting>>,
    /// The number of each commit seen, as a row or as a parent, so that lanes compare numbers and not hashes.
    numbers: HashMap<&'rows CommitId, usize>,
    /// The color the next new lane gets.
    next_color: usize,
}

impl<'rows> Layout<'rows> {
    /// The cell of the next row, and the lanes as they are under it.
    fn cell(&mut self, row: &GraphInput<'rows>) -> GraphCell {
        let commit = row.id.map(|id| self.number(id));
        let (lane, color) = match commit.and_then(|commit| self.lane_waiting_for(commit)) {
            Some(lane) => (lane, self.color_of(lane)),
            None => (self.free_lane(), self.new_color()),
        };
        let mut lines = Vec::with_capacity(self.lanes.len() * 2 + row.parents.len());
        self.pass_through(commit, lane, &mut lines);
        let mut parents: Vec<&CommitId> = Vec::with_capacity(row.parents.len());
        for parent in row.parents {
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
        if let Some((&first, others)) = parents.split_first() {
            let first = self.number(first);
            self.wait(lane, first, color, row.is_dashed);
            lines.push(line(Half::Lower, (lane, lane), color, row.is_dashed));
            for &parent in others {
                let parent = self.number(parent);
                let (to, color) = self.lane_for_parent(parent, row.is_dashed);
                lines.push(line(Half::Lower, (lane, to), color, row.is_dashed));
            }
        }
        while self.lanes.last().is_some_and(Option::is_none) {
            let _ = self.lanes.pop();
        }
        GraphCell { lane, color, lines }
    }

    /// The lines of the lanes at the top of the row: those that wait for the commit end at its dot and are free from
    /// there on, the others go straight through both halves.
    fn pass_through(&mut self, commit: Option<usize>, lane: usize, lines: &mut Vec<CellLine>) {
        for (index, slot) in self.lanes.iter_mut().enumerate() {
            let Some(waiting) = *slot else {
                continue;
            };
            if Some(waiting.commit) == commit {
                lines.push(line(Half::Upper, (index, lane), waiting.color, waiting.is_dashed));
                *slot = None;
            } else {
                lines.push(line(Half::Upper, (index, index), waiting.color, waiting.is_dashed));
                lines.push(line(Half::Lower, (index, index), waiting.color, waiting.is_dashed));
            }
        }
    }

    /// The lane another parent of a merge goes to, and its color: the lane that already waits for it, or else a new
    /// lane with a new color.
    fn lane_for_parent(&mut self, parent: usize, is_dashed: bool) -> (usize, usize) {
        if let Some(lane) = self.lane_waiting_for(parent) {
            return (lane, self.color_of(lane));
        }
        let lane = self.free_lane();
        let color = self.new_color();
        self.wait(lane, parent, color, is_dashed);
        (lane, color)
    }

    /// The lane that waits for a commit: the leftmost one that is not dashed, or else the leftmost.
    fn lane_waiting_for(&self, commit: usize) -> Option<usize> {
        let mut dashed = None;
        for (index, waiting) in self.lanes.iter().enumerate() {
            match waiting {
                Some(waiting) if waiting.commit == commit && !waiting.is_dashed => return Some(index),
                Some(waiting) if waiting.commit == commit && dashed.is_none() => dashed = Some(index),
                _ => {}
            }
        }
        dashed
    }

    /// The leftmost free lane, or a new one at the right.
    fn free_lane(&self) -> usize {
        self.lanes.iter().position(Option::is_none).unwrap_or(self.lanes.len())
    }

    /// Makes a lane wait for a commit, adding lanes up to it when it is a new one at the right.
    fn wait(&mut self, lane: usize, commit: usize, color: usize, is_dashed: bool) {
        if self.lanes.len() <= lane {
            self.lanes.resize(lane + 1, None);
        }
        if let Some(slot) = self.lanes.get_mut(lane) {
            *slot = Some(Waiting {
                commit,
                color,
                is_dashed,
            });
        }
    }

    /// The color of a lane that waits.
    fn color_of(&self, lane: usize) -> usize {
        self.lanes
            .get(lane)
            .copied()
            .flatten()
            .map_or(0, |waiting| waiting.color)
    }

    /// The next color.
    fn new_color(&mut self) -> usize {
        let color = self.next_color;
        self.next_color += 1;
        color
    }

    /// The number of a commit, given the first time it is seen.
    fn number(&mut self, id: &'rows CommitId) -> usize {
        let next = self.numbers.len();
        *self.numbers.entry(id).or_insert(next)
    }
}

/// A line of a cell, from one lane to another.
fn line(half: Half, (from, to): (usize, usize), color: usize, is_dashed: bool) -> CellLine {
    CellLine {
        half,
        from,
        to,
        color,
        is_dashed,
    }
}

#[cfg(test)]
mod tests;
