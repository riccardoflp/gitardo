//! The rows of a Merge page: the lines of its columns put level, so that every region starts on the same row in each
//! column and the columns scroll together.

use std::ops::Range;

use super::Region;

/// How many lines each column has; none for the base while it is not shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColumnLengths {
    /// The base, when it is shown.
    pub base: Option<usize>,
    /// The current side.
    pub current: usize,
    /// The result.
    pub result: usize,
    /// The incoming side.
    pub incoming: usize,
}

/// A row of the page: a line of each column, or a blank filler where a column has fewer lines there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct MergeRow {
    /// The line of the base.
    pub base: Option<usize>,
    /// The line of the current side.
    pub current: Option<usize>,
    /// The line of the result.
    pub result: Option<usize>,
    /// The line of the incoming side.
    pub incoming: Option<usize>,
    /// The region the row is of, by its place; none between regions.
    pub region: Option<usize>,
}

/// The rows of a page, and the rows of each region.
#[derive(Clone, Debug, PartialEq, Eq, Default, Hash)]
pub struct MergeLayout {
    /// The rows, from the top.
    pub rows: Vec<MergeRow>,
    /// The rows of each region, in order.
    pub regions: Vec<Range<usize>>,
}

/// The lines of the columns in rows: between two regions and in each region, the lines of each column face each other
/// in order, and the column with fewer lines there has blank rows after them. A region has at least one row, so that
/// its band shows even where it took every line out.
#[must_use]
pub fn merge_rows(regions: &[Region], lengths: ColumnLengths) -> MergeLayout {
    let mut layout = MergeLayout::default();
    let mut at = Places::default();
    for (place, region) in regions.iter().enumerate() {
        let until = Places {
            base: region.base.start,
            current: region.current.start,
            result: region.result.start,
            incoming: region.incoming.start,
        };
        stretch(&mut layout, (&mut at, until), (None, lengths.base.is_some()));
        let start = layout.rows.len();
        let until = Places {
            base: region.base.end,
            current: region.current.end,
            result: region.result.end,
            incoming: region.incoming.end,
        };
        stretch(&mut layout, (&mut at, until), (Some(place), lengths.base.is_some()));
        if layout.rows.len() == start {
            layout.rows.push(MergeRow {
                region: Some(place),
                ..MergeRow::default()
            });
        }
        layout.regions.push(start..layout.rows.len());
    }
    let end = Places {
        base: lengths.base.unwrap_or(0),
        current: lengths.current,
        result: lengths.result,
        incoming: lengths.incoming,
    };
    stretch(&mut layout, (&mut at, end), (None, lengths.base.is_some()));
    layout
}

/// Where each column is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Places {
    /// The next line of the base.
    base: usize,
    /// The next line of the current side.
    current: usize,
    /// The next line of the result.
    result: usize,
    /// The next line of the incoming side.
    incoming: usize,
}

/// The rows from where each column is up to a line of each, the lines facing each other in order.
fn stretch(layout: &mut MergeLayout, (at, until): (&mut Places, Places), (region, has_base): (Option<usize>, bool)) {
    let count = |from: usize, to: usize| to.saturating_sub(from);
    let base = if has_base { count(at.base, until.base) } else { 0 };
    let current = count(at.current, until.current);
    let result = count(at.result, until.result);
    let incoming = count(at.incoming, until.incoming);
    let rows = base.max(current).max(result).max(incoming);
    let line = |from: usize, length: usize, offset: usize| (offset < length).then_some(from + offset);
    for offset in 0..rows {
        layout.rows.push(MergeRow {
            base: line(at.base, base, offset),
            current: line(at.current, current, offset),
            result: line(at.result, result, offset),
            incoming: line(at.incoming, incoming, offset),
            region,
        });
    }
    at.base = at.base.max(until.base);
    at.current = at.current.max(until.current);
    at.result = at.result.max(until.result);
    at.incoming = at.incoming.max(until.incoming);
}
