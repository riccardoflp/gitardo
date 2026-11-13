//! The rows of a Merge page: the line of each column with its number, in the color of its region, the words a side
//! changed marked; the bands of the regions in the gutters; git's markers in the result; the labels over the columns;
//! the outline of the region chosen; and a mark for each region beside the scroll bar.

use std::ops::Range;

use model::merge_view::{MergeRow, Region, RegionKind};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Color, Label, Point, Rect, TextKind, TextRun, Theme, Tone, Ui};

use super::columns::Columns;
use super::gutter::{self, GutterSide, TakeSideState};
use super::{MergePageState, MergeText};
use crate::intent::Intent;
use crate::text::merge_view as words;

/// How opaque the color of a region is behind its lines.
const REGION_ALPHA: u8 = 0x30;
/// How opaque it is for a conflict resolved in the file, drawn faintly.
const FAINT_ALPHA: u8 = 0x12;
/// How opaque the lines of git's markers are.
const MARKER_ALPHA: u8 = 0x70;
/// How opaque the words a side changed are, over the color of their line.
const WORD_ALPHA: u8 = 0x60;
/// How opaque the band of a region is in a gutter.
const BAND_ALPHA: u8 = 0xb0;
/// How opaque the band of a conflict resolved in the file is.
const FAINT_BAND_ALPHA: u8 = 0x40;
/// How wide the mark in the margin of the result is, that joins a part of a conflict to its side.
const MARGIN_MARK: f32 = 3.0;

/// The height of a row: a line of code.
pub(crate) fn row_height(ui: &Ui<'_>) -> f32 {
    ui.line_height(ui.text_style(TextKind::Code)) + 4.0
}

/// The color of a region: green for the current side, blue for the incoming side, grey for both, red for a conflict.
pub(crate) fn region_color(kind: RegionKind, theme: &Theme) -> Color {
    let colors = &theme.colors;
    match kind {
        RegionKind::Current => colors.added,
        RegionKind::Incoming => colors.info,
        RegionKind::Both => colors.muted_foreground,
        RegionKind::Conflict => colors.conflict,
    }
}

/// Whether a region is a conflict resolved in the file, which is drawn faintly.
fn is_faint(region: &Region) -> bool {
    region.kind == RegionKind::Conflict && !region.is_unresolved()
}

/// The labels over the columns.
pub(crate) fn labels(ui: &mut Ui<'_>, rect: Rect, (state, columns): (&MergePageState<'_>, &Columns)) {
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.medium;
    let mut named = vec![
        (state.labels.current, columns.current),
        (state.labels.result, columns.result),
        (state.labels.incoming, columns.incoming),
    ];
    if let Some((base, _)) = columns.base {
        named.insert(0, (words::BASE, base));
    }
    for (name, column) in named {
        let cell = inset(
            Rect::new(column.x, rect.y, column.width, rect.height),
            Insets::symmetric(padding, 0.0),
        );
        let _ = ui.label(cell, &Label::new(name).kind(TextKind::SmallStrong).tone(Tone::Muted));
    }
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, ui::Axis::Horizontal);
}

/// What the rows say while the file is read.
pub(crate) fn loading(ui: &mut Ui<'_>, rect: Rect) {
    let height = row_height(ui);
    let cell = inset(
        Rect::new(rect.x, rect.y, rect.width, height),
        Insets::symmetric(ui.theme().sizes.spacing.large, 0.0),
    );
    let _ = ui.label(cell, &Label::new(words::LOADING).tone(Tone::Muted));
}

/// A row of the page: its cells, the bands of its region, and on the first row of a region the buttons of its bands.
pub(crate) fn row(
    ui: &mut Ui<'_>,
    (state, text): (&MergePageState<'_>, &MergeText),
    (row, columns): (&ui::Row, &Columns),
) -> Vec<Intent> {
    let Some(merge_row) = state.layout.rows.get(row.index) else {
        return Vec::new();
    };
    let region = merge_row
        .region
        .and_then(|place| text.regions.get(place).map(|region| (place, region)));
    let rect = row.rect;
    let across = |column: Rect| Rect::new(column.x, rect.y, column.width, rect.height);
    cells(ui, (text, merge_row, region.map(|(_, region)| region)), (rect, columns));
    let mut intents = Vec::new();
    if let Some((place, region)) = region {
        if let Some((_, base_gutter)) = columns.base {
            band(ui, across(base_gutter), region);
        }
        if region.kind != RegionKind::Incoming {
            band(ui, across(columns.left), region);
        }
        if region.kind != RegionKind::Current {
            band(ui, across(columns.right), region);
        }
        let is_first = state
            .layout
            .regions
            .get(place)
            .is_some_and(|rows| rows.start == row.index);
        if is_first {
            intents.extend(buttons(ui, (state, text), (place, region), (rect, columns)));
        }
        let rows = state.layout.regions.get(place).cloned().unwrap_or_default();
        if state.current == Some(place) {
            outline(
                ui,
                (rect, columns),
                (row.index == rows.start, row.index + 1 == rows.end),
            );
        }
    }
    intents
}

/// The cells of a row, one for each column shown.
fn cells(
    ui: &mut Ui<'_>,
    (text, row, region): (&MergeText, &MergeRow, Option<&Region>),
    (rect, columns): (Rect, &Columns),
) {
    let theme = *ui.theme();
    let across = |column: Rect| Rect::new(column.x, rect.y, column.width, rect.height);
    let tint = |is_changed: bool| match region {
        Some(region) if is_changed => {
            let alpha = if is_faint(region) { FAINT_ALPHA } else { REGION_ALPHA };
            region_color(region.kind, &theme).with_alpha(alpha)
        }
        _ => Color::TRANSPARENT,
    };
    let kind = region.map(|region| region.kind);
    let current_changed = kind.is_some_and(|kind| kind != RegionKind::Incoming);
    let incoming_changed = kind.is_some_and(|kind| kind != RegionKind::Current);
    if let Some((base, _)) = columns.base {
        let line = row
            .base
            .map(|line| (line, text.base_line(line), &[] as &[Range<usize>]));
        cell(ui, across(base), (line, tint(true)));
    }
    let marks_of = |marks: &'_ [Vec<Range<usize>>], start: usize, line: usize| -> Vec<Range<usize>> {
        marks.get(line.saturating_sub(start)).cloned().unwrap_or_default()
    };
    let current_marks = match (row.current, region) {
        (Some(line), Some(region)) => marks_of(&region.current_marks, region.current.start, line),
        _ => Vec::new(),
    };
    let current = row
        .current
        .map(|line| (line, text.current_line(line), current_marks.as_slice()));
    cell(ui, across(columns.current), (current, tint(current_changed)));
    result_cell(ui, (text, row.result, region), across(columns.result));
    let incoming_marks = match (row.incoming, region) {
        (Some(line), Some(region)) => marks_of(&region.incoming_marks, region.incoming.start, line),
        _ => Vec::new(),
    };
    let incoming = row
        .incoming
        .map(|line| (line, text.incoming_line(line), incoming_marks.as_slice()));
    cell(ui, across(columns.incoming), (incoming, tint(incoming_changed)));
}

/// The cell of the result: its line in the color of its region, git's markers marked, and the two parts of a conflict
/// git still marks joined to their sides by a mark in the margin.
fn result_cell(ui: &mut Ui<'_>, (text, line, region): (&MergeText, Option<usize>, Option<&Region>), rect: Rect) {
    let theme = *ui.theme();
    let colors = theme.colors;
    let Some(line) = line else {
        ui.fill_rect(rect, colors.card_background);
        return;
    };
    let mut background = region.map_or(Color::TRANSPARENT, |region| {
        let alpha = if is_faint(region) { FAINT_ALPHA } else { REGION_ALPHA };
        region_color(region.kind, &theme).with_alpha(alpha)
    });
    let block = region.and_then(|region| region.marked.as_ref().filter(|_| region.is_unresolved()));
    if let Some(block) = block {
        let is_marker = !block.current.contains(&line) && !block.incoming.contains(&line) && block.span.contains(&line);
        if is_marker {
            background = colors.conflict.with_alpha(MARKER_ALPHA);
        }
    }
    cell(ui, rect, (Some((line, text.result_line(line), &[])), background));
    if let Some(block) = block {
        if block.current.contains(&line) {
            ui.fill_rect(Rect::new(rect.x, rect.y, MARGIN_MARK, rect.height), colors.added);
        } else if block.incoming.contains(&line) {
            ui.fill_rect(
                Rect::new(rect.right() - MARGIN_MARK, rect.y, MARGIN_MARK, rect.height),
                colors.info,
            );
        }
    }
}

/// A line of a cell: its number, its text and the words changed in it.
type CellLine<'a> = (usize, &'a str, &'a [Range<usize>]);

/// A cell: its line number and its text on the color of its region, with the words changed marked; a blank filler
/// with no number where the column has no line in the row.
fn cell(ui: &mut Ui<'_>, rect: Rect, (line, background): (Option<CellLine<'_>>, Color)) {
    let theme = *ui.theme();
    let colors = theme.colors;
    let Some((number, text, marks)) = line else {
        ui.fill_rect(rect, colors.card_background);
        return;
    };
    if !background.is_transparent() {
        ui.fill_rect(rect, background);
    }
    let style = ui.text_style(TextKind::Code);
    let spacing = ui.theme().sizes.spacing.medium;
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing, 0.0)), spacing);
    let number_cell = parts.left(ui.measure("00000", style));
    let said = (number + 1).to_string();
    let width = ui.measure(&said, style);
    let number_rect = Rect::new(number_cell.right() - width, number_cell.y, width, number_cell.height);
    code(ui, number_rect, (&said, colors.muted_foreground));
    let text_cell = parts.rest();
    for range in marks {
        let before = |end: usize| text.get(..end).unwrap_or_default().replace('\t', "    ");
        let start = ui.measure(&before(range.start), style);
        let end = ui.measure(&before(range.end), style);
        let left = (text_cell.x + start).min(text_cell.right());
        let marked = Rect::new(
            left,
            text_cell.y + 1.0,
            ((text_cell.x + end).min(text_cell.right()) - left).max(0.0),
            text_cell.height - 2.0,
        );
        ui.fill_rect(marked, colors.foreground.with_alpha(WORD_ALPHA / 2));
    }
    code(ui, text_cell, (&text.replace('\t', "    "), colors.foreground));
}

/// Code, in the middle of a rectangle down, cut at its right.
fn code(ui: &mut Ui<'_>, rect: Rect, (text, color): (&str, Color)) {
    let style = ui.text_style(TextKind::Code);
    let line = place(rect, (rect.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text,
        style,
        color,
        origin: Point::new(line.x, line.y),
        max_width: Some(rect.width),
    };
    let _ = ui.text(&run);
}

/// The band of a region in a gutter, in its color; faint for a conflict resolved in the file.
fn band(ui: &mut Ui<'_>, gutter: Rect, region: &Region) {
    let theme = *ui.theme();
    let alpha = if is_faint(region) { FAINT_BAND_ALPHA } else { BAND_ALPHA };
    let strip = Rect::new(gutter.x + gutter.width * 0.5 - 2.0, gutter.y, 4.0, gutter.height);
    ui.fill_rect(strip, region_color(region.kind, &theme).with_alpha(alpha));
}

/// The buttons on the bands of a region, on its first row: those of the current side in the left gutter, those of the
/// incoming side in the right one.
fn buttons(
    ui: &mut Ui<'_>,
    (state, text): (&MergePageState<'_>, &MergeText),
    (place_of, region): (usize, &Region),
    (rect, columns): (Rect, &Columns),
) -> Vec<Intent> {
    let offers = text.offers.get(place_of).copied().unwrap_or_default();
    let proposal = text
        .proposal_of(place_of)
        .map(|(index, proposal)| (index, words::rules_proposal(proposal.reason)));
    let mut intents = Vec::new();
    let sides = [
        (GutterSide::Current, columns.left, region.kind != RegionKind::Incoming),
        (GutterSide::Incoming, columns.right, region.kind != RegionKind::Current),
    ];
    for (side, gutter_rect, has_band) in sides {
        if !has_band {
            continue;
        }
        let take = TakeSideState {
            page: state.id,
            region: place_of,
            side,
            is_conflict: region.kind == RegionKind::Conflict,
            offers,
            proposal: proposal
                .as_ref()
                .filter(|_| side == GutterSide::Current)
                .map(|(index, said)| (*index, said.as_str())),
        };
        let cell = Rect::new(gutter_rect.x, rect.y, gutter_rect.width, rect.height);
        intents.extend(ui.clip(cell, |ui| gutter::take_side_view(&take, ui)));
    }
    intents
}

/// The outline of the region chosen across every column: its sides, its top on its first row, its bottom on its last.
fn outline(ui: &mut Ui<'_>, (rect, columns): (Rect, &Columns), (is_first, is_last): (bool, bool)) {
    let theme = *ui.theme();
    let color = theme.colors.focus_ring;
    let thickness = theme.sizes.focus_ring;
    let (left, right) = columns.span();
    let span = Rect::new(left, rect.y, right - left, rect.height);
    ui.fill_rect(Rect::new(span.x, span.y, thickness, span.height), color);
    ui.fill_rect(
        Rect::new(span.right() - thickness, span.y, thickness, span.height),
        color,
    );
    if is_first {
        ui.fill_rect(Rect::new(span.x, span.y, span.width, thickness), color);
    }
    if is_last {
        ui.fill_rect(
            Rect::new(span.x, span.bottom() - thickness, span.width, thickness),
            color,
        );
    }
}

/// A mark beside the scroll bar for each region, in its color, where it is among the rows.
pub(crate) fn scroll_marks(ui: &mut Ui<'_>, (state, text): (&MergePageState<'_>, &MergeText), rect: Rect) {
    let total = state.layout.rows.len();
    if total == 0 {
        return;
    }
    let theme = *ui.theme();
    let width = ui.theme().sizes.scrollbar * 0.5;
    let to_float = crate::number::float_of_count;
    for (region, rows) in text.regions.iter().zip(&state.layout.regions) {
        let y = rect.y + rect.height * to_float(rows.start) / to_float(total);
        let height = 2.0_f32.max(rect.height * to_float(rows.len()) / to_float(total));
        ui.fill_rect(
            Rect::new(rect.right() - width, y, width, height),
            region_color(region.kind, &theme),
        );
    }
}
