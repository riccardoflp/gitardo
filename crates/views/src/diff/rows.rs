//! The rows of a Diff page: the header of each file, the rows of its two sides with their line numbers, the colors of
//! the changes and the words changed in them, the band of each change in the gutter, the bars of the folds, and the
//! names over the columns and the marks of the changes beside the scroll bar.

use std::ops::Range;

use model::diff::{AlignedRow, LineChangeKind, RowLine};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{CodeKind, CodeSpan, Color, Label, Point, Rect, Sense, TextKind, TextRun, Tone, Ui};

use super::columns::Columns;
use super::{DiffIntent, DiffPageIntent, DiffPageState, DiffText, FileDiff, PageLine};
use crate::intent::Intent;
use crate::text::diff as words;

/// How opaque the color of the words changed is, over the color of their line.
const WORD_ALPHA: u8 = 0x60;
/// How opaque the band of a change is in the gutter.
const BAND_ALPHA: u8 = 0xb0;
/// How opaque a band, and the colors of the lines, are for a change staged already.
const STAGED_ALPHA: u8 = 0x40;

/// The height of a row of the page: a line of code.
pub(crate) fn row_height(ui: &Ui<'_>) -> f32 {
    ui.line_height(ui.text_style(TextKind::Code)) + 4.0
}

/// The names over the two columns: **Working tree**, **Index**, `HEAD`, a branch, a commit, or **Empty**.
pub(crate) fn names(ui: &mut Ui<'_>, rect: Rect, (state, columns): (&DiffPageState<'_>, &Columns)) {
    let theme = *ui.theme();
    let (left, right) = state.names;
    let padding = theme.sizes.spacing.medium;
    for (name, column) in [(left, columns.left), (right, columns.right)] {
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

/// A row of the page.
pub(crate) fn row(ui: &mut Ui<'_>, state: &DiffPageState<'_>, (row, columns): (&ui::Row, &Columns)) -> Vec<Intent> {
    let Some(line) = state.lines.get(row.index) else {
        return Vec::new();
    };
    match line {
        PageLine::Header(file) => {
            header(ui, state, (*file, row.rect));
            Vec::new()
        }
        PageLine::Note(file) => {
            note(ui, state, (*file, row.rect));
            Vec::new()
        }
        PageLine::Folded(file, rows) => folded(ui, state, (*file, rows, row.rect)),
        PageLine::Row(file, index) => {
            if let Some(FileDiff::Text(text)) = state.diffs.get(*file)
                && let Some(aligned) = text.aligned.rows.get(*index)
            {
                let current = current_rows(state, *file, text);
                sides(ui, text, (aligned, *index, current), (row.rect, columns));
                let first_of = aligned.change.filter(|change| {
                    text.aligned
                        .changes
                        .get(*change)
                        .is_some_and(|rows| rows.rows.start == *index)
                });
                if let Some(change) = first_of {
                    let gutter = Rect::new(columns.gutter.x, row.rect.y, columns.gutter.width, row.rect.height);
                    return super::gutter::change_buttons(ui, state, (*file, text, change), gutter);
                }
            }
            Vec::new()
        }
    }
}

/// The header of a file: its path, `old → new` for a renamed file.
fn header(ui: &mut Ui<'_>, state: &DiffPageState<'_>, (file, rect): (usize, Rect)) {
    let Some(diff_file) = state.files.get(file) else {
        return;
    };
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.card_background);
    let cell = inset(rect, Insets::symmetric(theme.sizes.spacing.medium, 0.0));
    let _ = ui.label(cell, &Label::new(&diff_file.shown_path()).kind(TextKind::Medium));
}

/// What a file with no rows says.
fn note(ui: &mut Ui<'_>, state: &DiffPageState<'_>, (file, rect): (usize, Rect)) {
    let said = match state.diffs.get(file) {
        Some(FileDiff::Loading) | None => words::LOADING,
        Some(FileDiff::Binary) => words::LOOKS_BINARY,
        Some(FileDiff::TooComplex) => words::TOO_MANY_DIFFERENCES,
        Some(FileDiff::Failed) => words::COULD_NOT_READ,
        Some(FileDiff::Text(_)) => words::NO_CHANGE,
    };
    let cell = inset(rect, Insets::symmetric(ui.theme().sizes.spacing.large, 0.0));
    let _ = ui.label(cell, &Label::new(said).tone(Tone::Muted));
}

/// The bar of a fold, **⋯ 12 unchanged lines**, which shows its rows when clicked.
fn folded(ui: &mut Ui<'_>, state: &DiffPageState<'_>, (file, rows, rect): (usize, &Range<usize>, Rect)) -> Vec<Intent> {
    let theme = *ui.theme();
    // Above the sides, which take the pointer over the rows to place the caret (`diff.edit`).
    let response = ui.interact_above(
        ui.id(("diff fold bar", state.id, file, rows.start)),
        rect,
        Sense::CLICK | Sense::HOVER,
    );
    let background = if response.hovered {
        theme.colors.list_hover
    } else {
        theme.colors.card_background
    };
    ui.fill_rect(rect, background);
    let said = words::unchanged_lines(rows.len());
    let cell = inset(rect, Insets::symmetric(theme.sizes.spacing.large, 0.0));
    let _ = ui.label(cell, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
    ui.describe(response.id, rect, &ui::Description::new(ui::AccessRole::Button, &said));
    ui.tooltip(&response, words::CLICK_TO_EXPAND);
    if response.clicked {
        return vec![Intent::Diff(DiffIntent::Page(
            state.id,
            DiffPageIntent::Unfold(file, rows.start),
        ))];
    }
    Vec::new()
}

/// The rows of the current change, when it is of this file.
fn current_rows(state: &DiffPageState<'_>, file: usize, text: &DiffText) -> Option<Range<usize>> {
    let place = state.changes.get(state.current?)?;
    (place.file == file)
        .then(|| text.aligned.changes.get(place.change).map(|change| change.rows.clone()))
        .flatten()
}

/// The two sides of a row and the gutter between them.
fn sides(
    ui: &mut Ui<'_>,
    text: &DiffText,
    (aligned, index, current): (&AlignedRow, usize, Option<Range<usize>>),
    (rect, columns): (Rect, &Columns),
) {
    let colors = ui.theme().colors;
    let change = aligned
        .change
        .and_then(|change| text.aligned.changes.get(change).map(|rows| (change, rows.kind)));
    let is_staged = aligned.change.is_some_and(|change| text.is_staged(change));
    let fade = |color: Color| {
        if is_staged {
            color.with_alpha(STAGED_ALPHA)
        } else {
            color
        }
    };
    let left = Rect::new(columns.left.x, rect.y, columns.left.width, rect.height);
    let right = Rect::new(columns.right.x, rect.y, columns.right.width, rect.height);
    let gutter = Rect::new(columns.gutter.x, rect.y, columns.gutter.width, rect.height);
    let (left_color, right_color) = match change {
        Some(_) => (fade(colors.removed_background), fade(colors.added_background)),
        None => (Color::TRANSPARENT, Color::TRANSPARENT),
    };
    side(
        ui,
        (left, aligned.left.as_ref()),
        (
            |line| text.left_line(line),
            &text.left_colors,
            left_color,
            colors.removed,
        ),
    );
    side(
        ui,
        (right, aligned.right.as_ref()),
        (
            |line| text.right_line(line),
            &text.right_colors,
            right_color,
            colors.added,
        ),
    );
    if let Some((_, kind)) = change {
        let color = match kind {
            LineChangeKind::Added => colors.added,
            LineChangeKind::Removed => colors.removed,
            LineChangeKind::Changed => colors.modified,
        };
        let band = inset(gutter, Insets::symmetric(gutter.width * 0.25, 0.0));
        let alpha = if is_staged { STAGED_ALPHA } else { BAND_ALPHA };
        ui.fill_rect(band, color.with_alpha(alpha));
    }
    if let Some(current) = current.filter(|rows| rows.contains(&index)) {
        outline(ui, (rect, columns), (index == current.start, index + 1 == current.end));
    }
}

/// One side of a row: its line number and its text in the colors of its code on the color of its change, with the
/// words changed marked; a filler row, blank and with no number.
fn side<'text>(
    ui: &mut Ui<'_>,
    (rect, line): (Rect, Option<&RowLine>),
    (text_of, code_colors, background, mark): (impl Fn(usize) -> &'text str, &[Vec<CodeSpan>], Color, Color),
) {
    let colors = ui.theme().colors;
    let Some(line) = line else {
        ui.fill_rect(rect, colors.card_background);
        return;
    };
    if !background.is_transparent() {
        ui.fill_rect(rect, background);
    }
    let style = ui.text_style(TextKind::Code);
    let (number_cell, cell) = cells(ui, rect);
    let number = (line.line + 1).to_string();
    let width = ui.measure(&number, style);
    let number_rect = Rect::new(number_cell.right() - width, number_cell.y, width, number_cell.height);
    code(ui, number_rect, (&number, colors.muted_foreground));
    for range in &line.marks {
        let original = text_of(line.line);
        let start = ui.measure(
            &original.get(..range.start).unwrap_or_default().replace('\t', "    "),
            style,
        );
        let end = ui.measure(
            &original.get(..range.end).unwrap_or_default().replace('\t', "    "),
            style,
        );
        let left = (cell.x + start).min(cell.right());
        let marked = Rect::new(
            left,
            cell.y + 1.0,
            ((cell.x + end).min(cell.right()) - left).max(0.0),
            cell.height - 2.0,
        );
        ui.fill_rect(marked, mark.with_alpha(WORD_ALPHA));
    }
    let spans = code_colors.get(line.line).map_or(&[][..], Vec::as_slice);
    colored_code(ui, cell, (text_of(line.line), spans));
}

/// The cell of the number and the cell of the text of one side of a row.
fn cells(ui: &Ui<'_>, rect: Rect) -> (Rect, Rect) {
    let spacing = ui.theme().sizes.spacing.medium;
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing, 0.0)), spacing);
    let number_cell = parts.left(ui.measure("00000", ui.text_style(TextKind::Code)));
    (number_cell, parts.rest())
}

/// Where the text of one side of a row is drawn: after its number.
pub(crate) fn text_cell(ui: &Ui<'_>, rect: Rect) -> Rect {
    cells(ui, rect).1
}

/// A line of code in the colors of its tokens, its tabs as four spaces, cut at the right of a rectangle.
fn colored_code(ui: &mut Ui<'_>, rect: Rect, (line, spans): (&str, &[CodeSpan])) {
    let theme = *ui.theme();
    let style = ui.text_style(TextKind::Code);
    let mut parts: Vec<(Range<usize>, CodeKind)> = Vec::with_capacity(spans.len() * 2 + 1);
    let mut at = 0;
    for span in spans.iter().filter(|span| span.range.end <= line.len()) {
        if span.range.start < at {
            continue;
        }
        if span.range.start > at {
            parts.push((at..span.range.start, CodeKind::Plain));
        }
        parts.push((span.range.clone(), span.kind));
        at = span.range.end;
    }
    if at < line.len() {
        parts.push((at..line.len(), CodeKind::Plain));
    }
    for (range, kind) in parts {
        let before = line.get(..range.start).unwrap_or_default().replace('\t', "    ");
        let x = rect.x + ui.measure(&before, style);
        if x >= rect.right() {
            break;
        }
        let text = line.get(range).unwrap_or_default().replace('\t', "    ");
        let color = theme.code.color(kind, theme.colors.foreground);
        code(ui, Rect::new(x, rect.y, rect.right() - x, rect.height), (&text, color));
    }
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

/// The outline of the current change across both sides and the gutter: its sides, its top on its first row, its bottom
/// on its last.
fn outline(ui: &mut Ui<'_>, (rect, columns): (Rect, &Columns), (is_first, is_last): (bool, bool)) {
    let theme = *ui.theme();
    let color = theme.colors.focus_ring;
    let thickness = theme.sizes.focus_ring;
    let span = Rect::new(
        columns.left.x,
        rect.y,
        columns.right.right() - columns.left.x,
        rect.height,
    );
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

/// A mark beside the scroll bar for each change, in its color, where it is among the rows of the page.
pub(crate) fn scroll_marks(ui: &mut Ui<'_>, state: &DiffPageState<'_>, rect: Rect) {
    let total = state.lines.len();
    if total == 0 {
        return;
    }
    let colors = ui.theme().colors;
    let width = ui.theme().sizes.scrollbar * 0.5;
    let to_float = |count: usize| crate::number::float_of_count(count);
    for place in state.changes {
        let kind = match state.diffs.get(place.file) {
            Some(FileDiff::Text(text)) => text.aligned.changes.get(place.change).map(|change| change.kind),
            _ => None,
        };
        let color = match kind {
            Some(LineChangeKind::Added) => colors.added,
            Some(LineChangeKind::Removed) => colors.removed,
            Some(LineChangeKind::Changed) => colors.modified,
            None => continue,
        };
        let y = rect.y + rect.height * to_float(place.line) / to_float(total);
        ui.fill_rect(
            Rect::new(
                rect.right() - width,
                y,
                width,
                2.0_f32.max(rect.height / to_float(total)),
            ),
            color,
        );
    }
}
