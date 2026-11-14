//! The rows selected in a list, as ranges, so that selecting a million rows costs one range; the anchor a Shift click
//! extends from; and the row of the keyboard, which the arrows move.

#![allow(
    clippy::single_range_in_vec_init,
    reason = "a selection is a list of ranges of rows, and a list of one range is what one range selected is"
)]

use std::ops::Range;

/// The rows selected in a list, the anchor of the selection and the row of the keyboard.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The rows selected, as ranges in order, apart from each other.
    ranges: Vec<Range<usize>>,
    /// The row a Shift click or Shift and an arrow extend the selection from.
    anchor: Option<usize>,
    /// The row of the keyboard: where the arrows start from, and what Enter opens.
    cursor: Option<usize>,
}

impl Selection {
    /// One row selected, with the keyboard on it.
    #[must_use]
    pub fn single(row: usize) -> Selection {
        Selection {
            ranges: Vec::from([row..row + 1]),
            anchor: Some(row),
            cursor: Some(row),
        }
    }

    /// The rows from one to another, both included, with the anchor on the first and the keyboard on the second.
    #[must_use]
    pub fn span(anchor: usize, cursor: usize) -> Selection {
        Selection {
            ranges: Vec::from([anchor.min(cursor)..anchor.max(cursor) + 1]),
            anchor: Some(anchor),
            cursor: Some(cursor),
        }
    }

    /// Whether a row is selected.
    #[must_use]
    pub fn contains(&self, row: usize) -> bool {
        let after = self.ranges.partition_point(|range| range.end <= row);
        self.ranges.get(after).is_some_and(|range| range.contains(&row))
    }

    /// The rows selected, as ranges in order.
    #[must_use]
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }

    /// How many rows are selected.
    #[must_use]
    pub fn count(&self) -> usize {
        self.ranges.iter().map(ExactSizeIterator::len).sum()
    }

    /// The row of the keyboard.
    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// The row the selection extends from.
    #[must_use]
    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    /// Selects one row and nothing else, with the anchor and the keyboard on it: a click, or an arrow.
    pub fn select_only(&mut self, row: usize) {
        *self = Selection::single(row);
    }

    /// Selects the rows from the anchor to a row and nothing else, with the keyboard on the row: a Shift click, or
    /// Shift and an arrow.
    pub fn extend_to(&mut self, row: usize) {
        let anchor = self.anchor.unwrap_or(row);
        *self = Selection::span(anchor, row);
    }

    /// Selects a row if it was not, or leaves it out if it was, keeping the others, with the anchor and the keyboard
    /// on it: a Ctrl click.
    pub fn toggle(&mut self, row: usize) {
        if self.contains(row) {
            self.remove(row);
        } else {
            self.insert(row);
        }
        self.anchor = Some(row);
        self.cursor = Some(row);
    }

    /// Moves the keyboard to a row without changing what is selected: Ctrl and an arrow.
    pub fn move_cursor(&mut self, row: usize) {
        self.cursor = Some(row);
    }

    /// Selects every row of a list of a count, keeping the keyboard where it is.
    pub fn select_all(&mut self, count: usize) {
        self.ranges = if count == 0 { Vec::new() } else { Vec::from([0..count]) };
        self.anchor = self.anchor.or(Some(0)).filter(|_| count > 0);
    }

    /// Selects nothing.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// Leaves out the rows past the end of a list that became shorter.
    pub fn limit(&mut self, count: usize) {
        self.ranges.retain_mut(|range| {
            range.end = range.end.min(count);
            range.start < range.end
        });
        self.anchor = self.anchor.filter(|row| *row < count);
        self.cursor = self.cursor.filter(|row| *row < count);
    }

    /// Adds a row to the ranges, joining the ranges it touches.
    fn insert(&mut self, row: usize) {
        let at = self.ranges.partition_point(|range| range.end < row);
        let joins_next = self.ranges.get(at).is_some_and(|range| range.start <= row + 1);
        let joined = match self.ranges.get_mut(at) {
            Some(range) if joins_next => {
                range.start = range.start.min(row);
                range.end = range.end.max(row + 1);
                true
            }
            _ => false,
        };
        if !joined {
            self.ranges.insert(at, row..row + 1);
            return;
        }
        // The range grown may now touch the one after it.
        let touches = match (self.ranges.get(at), self.ranges.get(at + 1)) {
            (Some(range), Some(next)) => range.end >= next.start,
            _ => false,
        };
        if touches {
            let next = self.ranges.remove(at + 1);
            if let Some(range) = self.ranges.get_mut(at) {
                range.end = next.end;
            }
        }
    }

    /// Takes a row out of the ranges, splitting the range it is in.
    fn remove(&mut self, row: usize) {
        let at = self.ranges.partition_point(|range| range.end <= row);
        let Some(range) = self.ranges.get(at).cloned() else {
            return;
        };
        let pieces: Vec<Range<usize>> = [range.start..row, row + 1..range.end]
            .into_iter()
            .filter(|piece| !piece.is_empty())
            .collect();
        let _ = self.ranges.splice(at..=at, pieces);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_select_one_row_shift_extends_and_ctrl_toggles() {
        let mut selection = Selection::default();
        selection.select_only(5);
        selection.extend_to(8);
        assert_eq!(selection.ranges(), &[5..9]);
        selection.extend_to(2);
        assert_eq!((selection.ranges(), selection.cursor()), (&[2..6][..], Some(2)));
        selection.toggle(4);
        assert_eq!(selection.ranges(), &[2..4, 5..6]);
        selection.toggle(4);
        assert_eq!(selection.ranges(), &[2..6]);
        selection.toggle(7);
        selection.toggle(6);
        assert_eq!(selection.ranges(), &[2..8]);
        assert_eq!(selection.count(), 6);
        assert!(selection.contains(7) && !selection.contains(8) && !selection.contains(1));
    }

    #[test]
    fn a_million_rows_are_one_range_and_a_shorter_list_drops_the_rows_past_its_end() {
        let mut selection = Selection::single(3);
        selection.select_all(1_000_000);
        assert_eq!(
            (selection.ranges(), selection.count()),
            (&[0..1_000_000][..], 1_000_000)
        );
        assert!(selection.contains(999_999));
        selection.limit(10);
        assert_eq!(selection.ranges(), &[0..10]);
        let mut selection = Selection::single(20);
        selection.limit(10);
        assert_eq!((selection.count(), selection.cursor()), (0, None));
    }
}
