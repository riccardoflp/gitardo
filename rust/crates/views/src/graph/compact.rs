//! Compact rows (`graph.compact-rows`): each commit of the timeline on one line, as a table: its labels, its message,
//! then at the right end its author, its date and its hash, as `graph.columns` asks, in columns of the same widths on
//! every row; the rows are as tall as those of a list.

use ui::layout::{Align, place};
use ui::{Point, Rect, Sense, TextKind, TextRun, Ui};

use super::columns::{ColumnsState, hash_room};
use super::intent::{Switches, TimelineSwitch};
use crate::intent::Intent;
use crate::text::dates::{date_samples, full_date, relative_date};

/// How wide the column of the authors is at most; a longer name is shortened (Q649).
const AUTHOR_WIDTH: f32 = 120.0;
/// The share of the line of a row the column of the authors takes at most, so that a narrow timeline keeps its
/// messages.
const AUTHOR_SHARE: f32 = 0.2;
/// The share of the line of a row the columns take together at most; past it the hash leaves, then the author (Q650).
const COLUMNS_SHARE: f32 = 0.5;

/// How many lines of text a row of the timeline has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum RowLines {
    /// Two: the labels and the message, then the author and the date.
    #[default]
    Two,
    /// One, with `graph.compactRows` on.
    One,
}

impl RowLines {
    /// The lines the switches of the timeline ask for.
    #[must_use]
    pub fn of(switches: Switches) -> RowLines {
        if switches.is_on(TimelineSwitch::CompactRows) {
            RowLines::One
        } else {
            RowLines::Two
        }
    }

    /// How tall each row of the timeline is: with one line, as tall as a row of a list.
    #[must_use]
    pub fn row_height(self, ui: &Ui<'_>) -> f32 {
        match self {
            RowLines::Two => ui.theme().sizes.timeline_row_height,
            RowLines::One => ui.theme().sizes.row_height,
        }
    }
}

/// The widths of the columns at the right end of compact rows, the same on every row; none for a column that is not
/// shown.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CompactWidths {
    /// The author: as wide as a usual name, a fifth of the line at most.
    pub author: f32,
    /// The date: as wide as the widest date the timeline says.
    pub date: f32,
    /// The hash: seven characters.
    pub hash: f32,
}

impl CompactWidths {
    /// The widths of the columns the switches ask for, on a line of a width; the hash is not shown in a narrow window.
    /// Together they take at most a share of the line: on a line too short for them the hash leaves first, then the
    /// author, each as a whole, so that those left still line up.
    #[must_use]
    pub fn of(ui: &Ui<'_>, switches: Switches, is_narrow: bool, line: f32) -> CompactWidths {
        let detail = ui.text_style(TextKind::Detail);
        let widest_date = || {
            date_samples()
                .map(|sample| ui.measure(&sample, detail))
                .fold(0.0_f32, f32::max)
        };
        let mut widths = CompactWidths {
            author: if switches.is_on(TimelineSwitch::Author) {
                AUTHOR_WIDTH.min(line * AUTHOR_SHARE)
            } else {
                0.0
            },
            date: if switches.is_on(TimelineSwitch::Date) {
                widest_date().ceil()
            } else {
                0.0
            },
            hash: hash_room(ui, switches, is_narrow),
        };
        let most = line * COLUMNS_SHARE;
        let gap = ui.theme().sizes.spacing.large;
        if widths.room(gap) > most {
            widths.hash = 0.0;
        }
        if widths.room(gap) > most {
            widths.author = 0.0;
        }
        widths
    }

    /// The room the columns take at the end of a row, each with the gap before it.
    #[must_use]
    pub fn room(&self, gap: f32) -> f32 {
        [self.author, self.date, self.hash]
            .into_iter()
            .filter(|width| *width > 0.0)
            .map(|width| width + gap)
            .sum()
    }
}

/// The columns of a commit on a compact row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompactRow<'a> {
    /// The author, the date and the hash of the commit, and which of them are shown.
    pub columns: ColumnsState<'a>,
    /// How wide each column is.
    pub widths: CompactWidths,
}

/// The author, the date and the hash of a commit at the right end of the rectangle of its row its caller clipped it to,
/// each from the start of its column, in the middle of the row down; the date has its tooltip, the full date.
///
/// Feature: graph.compact-rows
pub fn compact_rows_view(state: &CompactRow<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.large;
    let columns = &state.columns;
    let date = relative_date(columns.date, columns.now);
    let cells = [
        (columns.author, state.widths.author, TextKind::Detail),
        (date.as_str(), state.widths.date, TextKind::Detail),
        (columns.hash, state.widths.hash, TextKind::Hash),
    ];
    let mut x = rect.right() - state.widths.room(gap) + gap;
    for (place_in_row, (text, width, kind)) in cells.into_iter().enumerate() {
        if width <= 0.0 {
            continue;
        }
        let style = ui.text_style(kind);
        let line = place(
            Rect::new(x, rect.y, width, rect.height),
            (width, ui.line_height(style)),
            (Align::Start, Align::Center),
        );
        let run = TextRun {
            text,
            style,
            color: theme.colors.muted_foreground,
            origin: Point::new(line.x, line.y),
            max_width: Some(width),
        };
        let drawn = ui.text(&run);
        if place_in_row == 1 {
            let id = ui.id(("date", columns.hash));
            let response = ui.interact(id, Rect::new(line.x, line.y, drawn, line.height), Sense::HOVER);
            ui.tooltip(&response, &full_date(columns.date));
        }
        x += width + gap;
    }
    Vec::new()
}
