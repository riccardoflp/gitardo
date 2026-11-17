//! A table of the preview: each column as wide as its widest cell, all of them narrowed together when the table is
//! wider than the page; its head in the strong typeface on a ground of its own, its cells aligned as the table says,
//! and lines between its rows and its columns.

use model::markdown::{Alignment, Block, Piece, Table};
use ui::{AccessRole, Rect, TextStyle};

use super::flow::{Place, flow};
use super::layout::{Column, Ink, Laid, fill};

/// Lays a table from a top, and gives its bottom.
pub(super) fn table(ink: &Ink<'_, '_>, laid: &mut Laid, table: &Table, (column, top): (Column, f32)) -> f32 {
    let looks = ink.looks;
    let widths = widths(ink, table, column.width);
    let lefts: Vec<f32> = widths
        .iter()
        .scan(column.left, |left, width| {
            let this = *left;
            *left += width;
            Some(this)
        })
        .collect();
    let row_layout = RowLayout {
        lefts: &lefts,
        widths: &widths,
        alignments: &table.alignments,
    };
    let start = laid.marks.len();
    let mut bottom = row(ink, laid, (&row_layout, &table.head, looks.strong), (column, top));
    let head = Rect::new(column.left, top, widths.iter().sum(), bottom - top);
    laid.marks.insert(start, fill(head, looks.fill));
    let mut tops = vec![top, bottom];
    for cells in &table.rows {
        bottom = row(ink, laid, (&row_layout, cells, looks.body), (column, bottom));
        tops.push(bottom);
    }
    lines(laid, (&lefts, &widths), &tops, ink);
    bottom
}

/// Where the cells of a row go.
struct RowLayout<'a> {
    /// The left of each column.
    lefts: &'a [f32],
    /// The width of each column.
    widths: &'a [f32],
    /// The alignment of each column.
    alignments: &'a [Alignment],
}

/// Lays the cells of a row from a top, and gives the bottom of the row.
fn row(
    ink: &Ink<'_, '_>,
    laid: &mut Laid,
    (layout, cells, style): (&RowLayout<'_>, &[Vec<Piece>], TextStyle),
    (column, top): (Column, f32),
) -> f32 {
    let padding = ink.looks.padding;
    let mut bottom = top + ink.ui.line_height(style) + 2.0 * padding;
    for (index, (left, width)) in layout.lefts.iter().zip(layout.widths).enumerate() {
        let Some(cell) = cells.get(index) else {
            continue;
        };
        let place = Place {
            left: left + padding,
            width: (width - 2.0 * padding).max(1.0),
            top: top + padding,
            style,
            color: column.color,
            alignment: layout.alignments.get(index).copied().unwrap_or(Alignment::Start),
        };
        let end = flow(ink, laid, cell, place) - ink.looks.line_gap + padding;
        let rect = Rect::new(*left, top, *width, (end - top).max(0.0));
        laid.nodes.push((rect, AccessRole::Label, Block::plain_text(cell)));
        bottom = bottom.max(end);
    }
    bottom
}

/// The width of each column: as wide as its widest cell, all of them narrowed together to the room there is.
fn widths(ink: &Ink<'_, '_>, table: &Table, room: f32) -> Vec<f32> {
    let looks = ink.looks;
    let count = table
        .rows
        .iter()
        .map(Vec::len)
        .chain([table.head.len(), table.alignments.len()])
        .max()
        .unwrap_or(0);
    let measure = |cell: &Vec<Piece>, style| ink.ui.fonts().measure(&Block::plain_text(cell), style);
    let mut widths: Vec<f32> = (0..count)
        .map(|index| {
            let head = table.head.get(index).map_or(0.0, |cell| measure(cell, looks.strong));
            let body = table
                .rows
                .iter()
                .filter_map(|cells| cells.get(index))
                .map(|cell| measure(cell, looks.body))
                .fold(head, f32::max);
            body + 2.0 * looks.padding + looks.line
        })
        .collect();
    let total: f32 = widths.iter().sum();
    if total > room && total > 0.0 {
        for width in &mut widths {
            *width *= room / total;
        }
    }
    widths
}

/// The lines of a table: under each row and at its top, at the left of each column and at its right.
fn lines(laid: &mut Laid, (lefts, widths): (&[f32], &[f32]), tops: &[f32], ink: &Ink<'_, '_>) {
    let looks = ink.looks;
    let (Some(left), Some(first), Some(last)) = (lefts.first(), tops.first(), tops.last()) else {
        return;
    };
    let width: f32 = widths.iter().sum();
    for top in tops {
        laid.marks
            .push(fill(Rect::new(*left, *top, width, looks.line), looks.border));
    }
    let edges = lefts.iter().copied().chain([left + width - looks.line]);
    for x in edges {
        laid.marks
            .push(fill(Rect::new(x, *first, looks.line, last - first), looks.border));
    }
}
