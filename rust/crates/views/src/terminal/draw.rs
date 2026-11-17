//! Drawing the screen of a terminal: each run of cells at its column in the font of code, in the colors of the theme's
//! terminal palette, the cells selected, the cursor.

use model::terminal::{CellColor, CellStyle, Cursor, CursorShape, Screen, Span};
use ui::{Border, Color, Point, Rect, TextRun, TextStyle, Ui};

/// Where the cells of a terminal are: the top left corner of the first, and the size of each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cells {
    /// The top left corner of the first cell.
    pub(crate) origin: Point,
    /// The width of a cell.
    pub(crate) width: f32,
    /// The height of a cell.
    pub(crate) height: f32,
    /// The style of the text.
    pub(crate) style: TextStyle,
}

impl Cells {
    /// The rectangle of some cells of a row.
    pub(crate) fn rect(self, row: usize, column: usize, count: usize) -> Rect {
        Rect::new(
            self.origin.x + to_f32(column) * self.width,
            self.origin.y + to_f32(row) * self.height,
            to_f32(count) * self.width,
            self.height,
        )
    }

    /// The cell under a point, the nearest one inside a screen of a size for a point outside it.
    pub(crate) fn cell_at(self, point: Point, (columns, rows): (usize, usize)) -> (usize, usize) {
        let index = |offset: f32, size: f32, count: usize| {
            let at = (offset / size).floor().max(0.0);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "at least 0, and clamped"
            )]
            let at = at.min(1.0e6) as usize;
            at.min(count.saturating_sub(1))
        };
        (
            index(point.y - self.origin.y, self.height, rows),
            index(point.x - self.origin.x, self.width, columns),
        )
    }
}

/// A count of cells as a length.
#[allow(
    clippy::cast_precision_loss,
    reason = "a screen has far fewer cells than an f32 counts exactly"
)]
fn to_f32(count: usize) -> f32 {
    count as f32
}

/// The colors a style draws with, its text and what is behind it, the inverse and fainter ones too.
fn colors_of(ui: &Ui<'_>, style: CellStyle) -> (Color, Option<Color>) {
    let theme = ui.theme();
    let resolve = |color: CellColor| match color {
        CellColor::Default => None,
        CellColor::Indexed(index) => Some(theme.terminal_color(index)),
        CellColor::Rgb(red, green, blue) => Some(Color::rgb(red, green, blue)),
    };
    let foreground = resolve(style.foreground).unwrap_or(theme.colors.foreground);
    let background = resolve(style.background);
    let (foreground, background) = if style.is_inverse {
        (background.unwrap_or(theme.colors.page_background), Some(foreground))
    } else {
        (foreground, background)
    };
    let foreground = if style.is_dim {
        foreground.with_alpha(160)
    } else {
        foreground
    };
    (foreground, background)
}

/// Draws a screen: the grounds of the runs, the cells selected, the text, then the cursor.
pub(crate) fn draw_screen(ui: &mut Ui<'_>, screen: &Screen, cells: Cells, has_focus: bool) {
    for (row, spans) in screen.rows.iter().enumerate() {
        for span in spans {
            if let (_, Some(background)) = colors_of(ui, span.style) {
                ui.fill_rect(cells.rect(row, span.column, span.width), background);
            }
        }
    }
    let selection = ui.theme().colors.text_selection;
    for &(row, from, to) in &screen.selected {
        ui.fill_rect(cells.rect(row, from, to.saturating_sub(from)), selection);
    }
    for (row, spans) in screen.rows.iter().enumerate() {
        for span in spans {
            draw_span(ui, span, cells, row);
        }
    }
    if let Some(cursor) = screen.cursor {
        draw_cursor(ui, screen, cursor, (cells, has_focus));
    }
}

/// Draws the text of a run, twice a hair apart for bold, as the font of code has no bold of its own here, and a line
/// under it when it is underlined.
fn draw_span(ui: &mut Ui<'_>, span: &Span, cells: Cells, row: usize) {
    let (color, _) = colors_of(ui, span.style);
    let rect = cells.rect(row, span.column, span.width);
    let text = span.text.trim_end();
    if !text.trim().is_empty() {
        let mut run = TextRun {
            text,
            style: cells.style,
            color,
            origin: Point::new(rect.x, rect.y),
            max_width: None,
        };
        let _width = ui.text(&run);
        if span.style.is_bold {
            run.origin.x += (cells.style.size / 16.0).max(0.5);
            let _width = ui.text(&run);
        }
    }
    if span.style.is_underlined {
        let line = Rect::new(rect.x, rect.bottom() - 2.0, rect.width, 1.0);
        ui.fill_rect(line, color);
    }
}

/// Draws the cursor: a block with the character under it in the color of the ground while the terminal has the
/// keyboard, an outline when it has not; a bar or a line for the shapes a program asks for.
fn draw_cursor(ui: &mut Ui<'_>, screen: &Screen, cursor: Cursor, (cells, has_focus): (Cells, bool)) {
    let theme = *ui.theme();
    let color = theme.colors.foreground;
    let rect = cells.rect(cursor.row, cursor.column, 1);
    match (cursor.shape, has_focus) {
        (CursorShape::Block, true) => {
            ui.fill_rect(rect, color);
            if let Some(character) = character_at(screen, cursor) {
                let mut buffer = [0; 4];
                let run = TextRun {
                    text: character.encode_utf8(&mut buffer),
                    style: cells.style,
                    color: theme.colors.page_background,
                    origin: Point::new(rect.x, rect.y),
                    max_width: None,
                };
                let _width = ui.text(&run);
            }
        }
        (CursorShape::Block, false) => ui.border(
            rect,
            Border {
                width: 1.0,
                color,
                radius: 0.0,
            },
        ),
        (CursorShape::Underline, _) => ui.fill_rect(Rect::new(rect.x, rect.bottom() - 2.0, rect.width, 2.0), color),
        (CursorShape::Beam, _) => ui.fill_rect(Rect::new(rect.x, rect.y, 2.0, rect.height), color),
    }
}

/// The character in the cell of the cursor, when there is one.
fn character_at(screen: &Screen, cursor: Cursor) -> Option<char> {
    let span = screen
        .rows
        .get(cursor.row)?
        .iter()
        .find(|span| span.column <= cursor.column && cursor.column < span.column + span.width)?;
    if span.is_wide {
        return span.text.chars().next();
    }
    span.text.chars().nth(cursor.column - span.column)
}
