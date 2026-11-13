//! The lines of code of the review: a number at the left, or a mark in its place, a bar in the color of where the
//! line comes from, a tint, and the text, cut at the right or wrapped.

use model::review::Origin;
use ui::{Color, Point, Rect, TextRun};
use ui::{TextKind, Ui};

/// How a line of code looks.
#[derive(Clone, Copy, Debug)]
pub(super) struct Look<'a> {
    /// Its number, or the mark in its place (`+`, `-`), or nothing.
    pub gutter: &'a str,
    /// The color of its text.
    pub color: Color,
    /// The bars at its left: one, two for a line of both sides, or none.
    pub bars: [Option<Color>; 2],
    /// The tint behind it.
    pub tint: Option<Color>,
}

/// The width of the numbers of a card: as wide as the largest of them.
pub(super) fn gutter_width(ui: &Ui<'_>, largest: usize) -> f32 {
    let style = ui.text_style(TextKind::Code);
    ui.measure(&largest.max(9).to_string(), style) + ui.theme().sizes.spacing.medium
}

/// The colors of where a line comes from: green for the current side, blue for the incoming one, both for a line of
/// both sides, grey for a line of the base, purple for a line neither has; and the tint of a new line.
pub(super) fn origin_bars(ui: &Ui<'_>, origin: Origin) -> ([Option<Color>; 2], Option<Color>) {
    let colors = ui.theme().colors;
    match origin {
        Origin::Current => ([Some(colors.success), None], None),
        Origin::Incoming => ([Some(colors.info), None], None),
        Origin::Both => ([Some(colors.success), Some(colors.info)], None),
        Origin::Base => ([Some(colors.muted_foreground), None], None),
        Origin::New => ([Some(new_color(ui)), None], Some(new_color(ui).with_alpha(32))),
        Origin::Blank => ([None, None], None),
    }
}

/// The color of the lines neither side has.
pub(super) fn new_color(ui: &Ui<'_>) -> Color {
    let lanes = ui.theme().lanes;
    lanes.get(2).copied().unwrap_or(ui.theme().colors.accent)
}

/// Draws a line of code over a row, cut at its right, with its gutter of a given width.
pub(super) fn code_line(ui: &mut Ui<'_>, rect: Rect, (text, gutter): (&str, f32), look: &Look<'_>) {
    let theme = *ui.theme();
    if let Some(tint) = look.tint {
        ui.fill_rect(rect, tint);
    }
    let bar = theme.sizes.border * 3.0;
    for (index, color) in look.bars.iter().enumerate() {
        if let Some(color) = color {
            let left = rect.x + crate::number::float_of_count(index) * bar;
            ui.fill_rect(Rect::new(left, rect.y, bar, rect.height), *color);
        }
    }
    let start = rect.x + 2.0 * bar + theme.sizes.spacing.small;
    let style = ui.text_style(TextKind::Code);
    let top = rect.y + (rect.height - ui.line_height(style)) * 0.5;
    let number_width = ui.measure(look.gutter, style);
    let _ = ui.text(&TextRun {
        text: look.gutter,
        style,
        color: theme.colors.muted_foreground,
        origin: Point::new(start + gutter - theme.sizes.spacing.medium - number_width, top),
        max_width: None,
    });
    let text_left = start + gutter;
    let _ = ui.text(&TextRun {
        text: &text.replace('\t', "    "),
        style,
        color: look.color,
        origin: Point::new(text_left, top),
        max_width: Some((rect.right() - text_left).max(0.0)),
    });
}

/// The width a line's text has in a column, after its bars and its gutter.
pub(super) fn text_width(ui: &Ui<'_>, width: f32, gutter: f32) -> f32 {
    let theme = ui.theme();
    (width - 6.0 * theme.sizes.border - theme.sizes.spacing.small - gutter).max(1.0)
}

/// A line cut into the rows it takes in a width: each as long as fits, the first keeping the number.
pub(super) fn wrapped(ui: &Ui<'_>, text: &str, width: f32) -> Vec<String> {
    let style = ui.text_style(TextKind::Code);
    let text = text.replace('\t', "    ");
    let mut rows = Vec::new();
    let mut row = String::new();
    for character in text.chars() {
        row.push(character);
        if ui.measure(&row, style) > width && row.chars().count() > 1 {
            let _ = row.pop();
            rows.push(std::mem::take(&mut row));
            row.push(character);
        }
    }
    rows.push(row);
    rows
}

/// How many rows a line takes in a width.
pub(super) fn rows_of(ui: &Ui<'_>, text: &str, width: f32) -> usize {
    let style = ui.text_style(TextKind::Code);
    if ui.measure(&text.replace('\t', "    "), style) <= width {
        1
    } else {
        wrapped(ui, text, width).len()
    }
}
