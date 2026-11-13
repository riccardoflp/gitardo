//! A conflict in three columns (`review.columns`): the current side, what is proposed and the incoming side, each with
//! the lines around it, joined by bands from each line proposed to the line of a side that is the same.

use ui::layout::Area;
use ui::{Border, Color, FillRule, Label, Path, Point, Rect, Split, Splitter, TextKind, Ui};

use super::card::CardState;
use super::lines::{Look, code_line, gutter_width, origin_bars, rows_of, text_width, wrapped};
use super::{EVEN, NumberedLine, ReviewIntent, SideColumn};
use crate::intent::Intent;
use crate::number::float_of_count;
use crate::text::review as words;

/// The narrowest the three columns stand side by side; narrower, they stand one above the other.
const NARROWEST: f32 = 600.0;

/// What a column has: its heading, its lines and the lines around them.
struct Column<'a> {
    /// Its heading.
    heading: String,
    /// Its lines, numbered, with how each looks.
    lines: Vec<(&'a NumberedLine, Look<'a>)>,
}

/// A line of a column as placed: its first row and how many rows it takes.
#[derive(Clone, Copy, Debug)]
struct Placed {
    /// Its top.
    top: f32,
    /// Its height.
    height: f32,
}

/// How the lines around a conflict look: faded.
fn faded(ui: &Ui<'_>) -> Look<'static> {
    Look {
        gutter: "",
        color: ui.theme().colors.muted_foreground,
        bars: [None, None],
        tint: None,
    }
}

/// The lines of a side column: the lines before, the side's own lines, those joined to a line proposed with a bar in
/// the side's color and the others faded, then the lines after.
fn side_lines<'a>(
    state: &CardState<'a>,
    ui: &Ui<'_>,
    (column, joins, bar): (&'a SideColumn, &[Option<usize>], Color),
) -> Vec<(&'a NumberedLine, Look<'a>)> {
    let faded = faded(ui);
    let foreground = ui.theme().colors.foreground;
    let own = column.lines.iter().enumerate().map(|(index, line)| {
        let look = if joins.contains(&Some(index)) {
            Look {
                color: foreground,
                bars: [Some(bar), None],
                ..faded
            }
        } else {
            faded
        };
        (line, look)
    });
    state
        .card
        .before
        .iter()
        .map(|line| (line, faded))
        .chain(own)
        .chain(column.after.iter().map(|line| (line, faded)))
        .collect()
}

/// The lines of the middle column: the lines before, the lines proposed with the bars of where they come from, then
/// the lines after.
fn middle_lines<'a>(state: &CardState<'a>, ui: &Ui<'_>) -> Vec<(&'a NumberedLine, Look<'a>)> {
    let card = state.card;
    let faded = faded(ui);
    let foreground = ui.theme().colors.foreground;
    let proposed = card.proposal.iter().flat_map(|proposal| &proposal.lines).map(|line| {
        let (bars, tint) = origin_bars(ui, line.origin);
        let look = Look {
            color: foreground,
            bars,
            tint,
            ..faded
        };
        (&line.line, look)
    });
    card.before
        .iter()
        .map(|line| (line, faded))
        .chain(proposed)
        .chain(card.after.iter().map(|line| (line, faded)))
        .collect()
}

/// The three columns of a card: the lines before, then each column's own lines, then the lines after.
fn columns<'a>(state: &CardState<'a>, ui: &Ui<'_>) -> [Column<'a>; 3] {
    let card = state.card;
    let colors = ui.theme().colors;
    let empty: &[Option<usize>] = &[];
    let (from_current, to_incoming) = card.proposal.as_ref().map_or((empty, empty), |proposal| {
        (&proposal.from_current[..], &proposal.to_incoming[..])
    });
    [
        Column {
            heading: words::side_heading(true, &state.sides.current),
            lines: side_lines(state, ui, (&card.current, from_current, colors.success)),
        },
        Column {
            heading: String::from(words::PROPOSED),
            lines: middle_lines(state, ui),
        },
        Column {
            heading: words::side_heading(false, &state.sides.incoming),
            lines: side_lines(state, ui, (&card.incoming, to_incoming, colors.info)),
        },
    ]
}

/// The widths of the three columns in a width, the gutters of the bands in the middle one.
fn widths(state: &CardState<'_>, width: f32) -> [f32; 3] {
    let (current, incoming) = state.widths;
    let part = |thousandths: u32| width * float_of_count(usize::try_from(thousandths).unwrap_or(0)) / 1000.0;
    let (current, incoming) = (part(current), part(incoming));
    [current, (width - current - incoming).max(0.0), incoming]
}

/// Where each line of a column goes, from a top, its text wrapped in a width.
fn place_lines(ui: &Ui<'_>, column: &Column<'_>, (top, width, gutter): (f32, f32, f32)) -> Vec<Placed> {
    let row = ui.theme().sizes.row_height;
    let text = text_width(ui, width, gutter);
    let mut at = top;
    column
        .lines
        .iter()
        .map(|(line, _)| {
            let height = row * float_of_count(rows_of(ui, &line.text, text));
            let placed = Placed { top: at, height };
            at += height;
            placed
        })
        .collect()
}

/// The height of the columns of a card in a width.
pub(super) fn height(state: &CardState<'_>, ui: &Ui<'_>, width: f32) -> f32 {
    let row = ui.theme().sizes.row_height;
    let gutter = gutter_width(ui, 9999);
    let columns = columns(state, ui);
    let column_height = |column: &Column<'_>, width: f32| {
        row + place_lines(ui, column, (0.0, width, gutter))
            .iter()
            .map(|placed| placed.height)
            .sum::<f32>()
    };
    if width < NARROWEST {
        return columns.iter().map(|column| column_height(column, width)).sum();
    }
    let band = ui.theme().sizes.spacing.huge;
    let widths = widths(state, width);
    columns
        .iter()
        .zip(widths)
        .enumerate()
        .map(|(index, (column, width))| column_height(column, if index == 1 { width - 2.0 * band } else { width }))
        .fold(0.0, f32::max)
}

/// A conflict in three columns, over the rectangle its caller clipped it to: the current side, what is proposed and
/// the incoming side, each line wrapped, a band joining each line proposed to the line of a side that is the same, the
/// pointer outlining a line and the lines joined to it, and the lines between the columns dragged to make one wider.
///
/// Feature: review.columns
pub fn columns_view(state: &CardState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let row = theme.sizes.row_height;
    let gutter = gutter_width(ui, 9999);
    let columns = columns(state, ui);
    if rect.width < NARROWEST {
        let mut area = Area::new(rect);
        for column in &columns {
            let placed = place_lines(ui, column, (0.0, rect.width, gutter));
            let height = row + placed.iter().map(|placed| placed.height).sum::<f32>();
            draw_column(ui, column, (area.top(height), gutter), &[]);
        }
        return Vec::new();
    }
    let band = theme.sizes.spacing.huge;
    let intents = dragged_widths(state, ui, (rect, band));
    let rects = column_rects(state, (rect, band));
    let placed: Vec<Vec<Placed>> = columns
        .iter()
        .zip(rects)
        .map(|(column, column_rect)| place_lines(ui, column, (column_rect.y + row, column_rect.width, gutter)))
        .collect();
    let lit = lit_lines(state, ui, (&rects, &placed));
    bands(state, ui, (&rects, &placed));
    for (index, (column, column_rect)) in columns.iter().zip(rects).enumerate() {
        let lit: Vec<usize> = lit
            .iter()
            .filter(|(which, _)| *which == index)
            .map(|(_, line)| *line)
            .collect();
        draw_column(ui, column, (column_rect, gutter), &lit);
    }
    intents
}

/// The lines between the columns, dragged to make a column wider for every conflict at once, or put back even by a
/// double click.
fn dragged_widths(state: &CardState<'_>, ui: &mut Ui<'_>, (rect, band): (Rect, f32)) -> Vec<Intent> {
    let [current_width, _, incoming_width] = widths(state, rect.width);
    let first = ui.splitter(
        "current split",
        rect,
        &Splitter {
            split: Split::Columns,
            default_size: rect.width * float_of_count(usize::try_from(EVEN).unwrap_or(0)) / 1000.0,
            least_first: band * 3.0,
            least_second: band * 6.0,
            size: Some(current_width),
            step: 0.0,
        },
    );
    let second = ui.splitter(
        "incoming split",
        first.second,
        &Splitter {
            split: Split::Columns,
            default_size: first.second.width * 0.5,
            least_first: band * 3.0,
            least_second: band * 3.0,
            size: Some(first.second.width - incoming_width),
            step: 0.0,
        },
    );
    if first.is_reset || second.is_reset {
        return vec![Intent::Review(ReviewIntent::Widths(None))];
    }
    if !first.is_changed && !second.is_changed {
        return Vec::new();
    }
    let thousandths = |width: f32| {
        let share = (width / rect.width * 1000.0).round().clamp(0.0, 1000.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=1000 above"
        )]
        let share = share as u32;
        share
    };
    vec![Intent::Review(ReviewIntent::Widths(Some((
        thousandths(first.first.width),
        thousandths(second.second.width),
    ))))]
}

/// The rectangles of the three columns, the middle one between the gutters of the bands.
fn column_rects(state: &CardState<'_>, (rect, band): (Rect, f32)) -> [Rect; 3] {
    let [current_width, middle_width, incoming_width] = widths(state, rect.width);
    [
        Rect::new(rect.x, rect.y, current_width, rect.height),
        Rect::new(
            rect.x + current_width + band,
            rect.y,
            (middle_width - 2.0 * band).max(0.0),
            rect.height,
        ),
        Rect::new(
            rect.x + current_width + middle_width,
            rect.y,
            incoming_width,
            rect.height,
        ),
    ]
}

/// Draws a column: its heading, then its lines, wrapped, those the pointer lights outlined.
fn draw_column(ui: &mut Ui<'_>, column: &Column<'_>, (rect, gutter): (Rect, f32), lit: &[usize]) {
    let theme = *ui.theme();
    let row = theme.sizes.row_height;
    let mut area = Area::new(rect);
    let _ = ui.label(area.top(row), &Label::new(&column.heading).kind(TextKind::SmallStrong));
    let text = text_width(ui, rect.width, gutter);
    for (index, (line, look)) in column.lines.iter().enumerate() {
        let pieces = wrapped(ui, &line.text, text);
        let line_rect = area.top(row * float_of_count(pieces.len()));
        let number = line.number.to_string();
        for (piece_index, piece) in pieces.iter().enumerate() {
            let piece_rect = Rect::new(
                line_rect.x,
                line_rect.y + row * float_of_count(piece_index),
                line_rect.width,
                row,
            );
            let gutter_said = if piece_index == 0 { number.as_str() } else { "" };
            code_line(
                ui,
                piece_rect,
                (piece, gutter),
                &Look {
                    gutter: gutter_said,
                    ..*look
                },
            );
        }
        if lit.contains(&index) {
            ui.border(
                line_rect,
                Border {
                    width: theme.sizes.border,
                    radius: 0.0,
                    color: theme.colors.focus_ring,
                },
            );
        }
    }
}

/// The lines of the middle column joined to the lines of the sides, by their places among the lines of each column.
fn joins(state: &CardState<'_>) -> Vec<(usize, usize, usize)> {
    let card = state.card;
    let Some(proposal) = &card.proposal else {
        return Vec::new();
    };
    let before = card.before.len();
    let mut joins = Vec::new();
    for (index, (current, incoming)) in proposal.from_current.iter().zip(&proposal.to_incoming).enumerate() {
        if let Some(line) = current {
            joins.push((0, before + line, before + index));
        }
        if let Some(line) = incoming {
            joins.push((2, before + line, before + index));
        }
    }
    joins
}

/// The bands between the columns: green from a line of the current side, blue to a line of the incoming side.
fn bands(state: &CardState<'_>, ui: &mut Ui<'_>, (rects, placed): (&[Rect; 3], &[Vec<Placed>])) {
    let colors = ui.theme().colors;
    let [current_rect, middle_rect, incoming_rect] = *rects;
    for (side, side_line, middle_line) in joins(state) {
        let (Some(side_placed), Some(middle_placed)) = (
            placed.get(side).and_then(|lines| lines.get(side_line)),
            placed.get(1).and_then(|lines| lines.get(middle_line)),
        ) else {
            continue;
        };
        let (side_x, middle_x, color) = if side == 0 {
            (current_rect.right(), middle_rect.x, colors.success)
        } else {
            (incoming_rect.x, middle_rect.right(), colors.info)
        };
        let mut path = Path::new();
        let _ = path
            .move_to(Point::new(side_x, side_placed.top))
            .line_to(Point::new(middle_x, middle_placed.top))
            .line_to(Point::new(middle_x, middle_placed.top + middle_placed.height))
            .line_to(Point::new(side_x, side_placed.top + side_placed.height))
            .close();
        ui.fill_path(path, color.with_alpha(64), FillRule::NonZero);
    }
}

/// The lines the pointer lights: the line under it and the lines joined to it, by column and place.
fn lit_lines(state: &CardState<'_>, ui: &Ui<'_>, (rects, placed): (&[Rect; 3], &[Vec<Placed>])) -> Vec<(usize, usize)> {
    let Some(pointer) = ui.pointer() else {
        return Vec::new();
    };
    let hovered = rects
        .iter()
        .zip(placed)
        .enumerate()
        .find_map(|(column, (rect, lines))| {
            if pointer.x < rect.x || pointer.x > rect.right() {
                return None;
            }
            let line = lines
                .iter()
                .position(|line| pointer.y >= line.top && pointer.y < line.top + line.height)?;
            Some((column, line))
        });
    let Some((column, line)) = hovered else {
        return Vec::new();
    };
    let mut lit = vec![(column, line)];
    for (side, side_line, middle_line) in joins(state) {
        if column == 1 && middle_line == line {
            lit.push((side, side_line));
        } else if column == side && side_line == line {
            lit.push((1, middle_line));
        }
    }
    lit
}
