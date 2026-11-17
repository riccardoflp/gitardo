//! The screen of a terminal as plain values: its rows as runs of cells of one look, the cursor, the cells selected and
//! the text they hold, and the modes the keys are sent in.

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Term, TermMode, point_to_viewport};
use alacritty_terminal::vte::ansi::{self, NamedColor};
use model::terminal::{CellColor, CellStyle, Cursor, CursorShape, Screen, Span};

/// The screen of a grid.
pub(crate) fn of<T: EventListener>(term: &Term<T>) -> Screen {
    let content = term.renderable_content();
    let offset = content.display_offset;
    let lines = term.screen_lines();
    let mut rows: Vec<Vec<Span>> = vec![Vec::new(); lines];
    for indexed in content.display_iter {
        let Some(point) = point_to_viewport(offset, indexed.point) else {
            continue;
        };
        if let Some(row) = rows.get_mut(point.line) {
            add_cell(row, point.column.0, indexed.cell);
        }
    }
    let cursor = point_to_viewport(offset, content.cursor.point).and_then(|point| {
        let shape = match content.cursor.shape {
            ansi::CursorShape::Block | ansi::CursorShape::HollowBlock => CursorShape::Block,
            ansi::CursorShape::Underline => CursorShape::Underline,
            ansi::CursorShape::Beam => CursorShape::Beam,
            ansi::CursorShape::Hidden => return None,
        };
        (offset == 0).then_some(Cursor {
            row: point.line,
            column: point.column.0,
            shape,
        })
    });
    let selected = content
        .selection
        .map(|range| selected_cells(range, (lines, term.columns()), offset))
        .unwrap_or_default();
    let mode = content.mode;
    Screen {
        columns: term.columns(),
        rows,
        cursor,
        selected,
        selected_text: term.selection_to_string().filter(|text| !text.is_empty()),
        scrolled: offset,
        is_application_cursor: mode.contains(TermMode::APP_CURSOR),
        is_bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
        is_alternate_screen: mode.contains(TermMode::ALT_SCREEN),
    }
}

/// The cells of a selection on a screen of some rows and columns scrolled up by some lines: each row with its first
/// column and the column after its last.
fn selected_cells(
    range: SelectionRange,
    (lines, columns): (usize, usize),
    offset: usize,
) -> Vec<(usize, usize, usize)> {
    (0..lines)
        .filter_map(|row| {
            let line = i32::try_from(row).ok()? - i32::try_from(offset).ok()?;
            if line < range.start.line.0 || line > range.end.line.0 {
                return None;
            }
            let from = if line == range.start.line.0 {
                range.start.column.0
            } else {
                0
            };
            let to = if line == range.end.line.0 {
                range.end.column.0 + 1
            } else {
                columns
            };
            Some((row, from, to))
        })
        .collect()
}

/// Adds a cell to its row: to the last run when it looks the same and follows it, else as a run of its own; a blank
/// cell with nothing behind it starts no run. A wide character is a run of its own, two columns wide, and the cell
/// after it, which only holds its place, is left out.
fn add_cell(row: &mut Vec<Span>, column: usize, cell: &Cell) {
    if cell
        .flags
        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
    {
        return;
    }
    let style = style_of(cell);
    let is_wide = cell.flags.contains(Flags::WIDE_CHAR);
    let is_blank = (cell.c == ' ' || cell.c == '\t' || cell.flags.contains(Flags::HIDDEN))
        && style.background == CellColor::Default
        && !style.is_inverse
        && !style.is_underlined;
    let character = if cell.flags.contains(Flags::HIDDEN) || cell.c == '\t' {
        ' '
    } else {
        cell.c
    };
    let mut text = String::from(character);
    if let Some(marks) = cell.zerowidth() {
        text.extend(marks);
    }
    let width = if is_wide { 2 } else { 1 };
    if let Some(last) = row.last_mut()
        && !is_wide
        && last.style == style
        && last.column + last.width == column
        && !last.is_wide
    {
        last.text.push_str(&text);
        last.width += 1;
        return;
    }
    if is_blank {
        return;
    }
    row.push(Span {
        column,
        width,
        is_wide,
        text,
        style,
    });
}

/// How a cell looks.
fn style_of(cell: &Cell) -> CellStyle {
    let flags = cell.flags;
    CellStyle {
        foreground: color_of(cell.fg),
        background: color_of(cell.bg),
        is_bold: flags.contains(Flags::BOLD),
        is_italic: flags.contains(Flags::ITALIC),
        is_underlined: flags.intersects(Flags::ALL_UNDERLINES),
        is_dim: flags.contains(Flags::DIM),
        is_inverse: flags.contains(Flags::INVERSE),
    }
}

/// A color of the parser as a color of a cell: the named colors are the 16 of the theme, their dim forms too.
fn color_of(color: ansi::Color) -> CellColor {
    match color {
        ansi::Color::Spec(rgb) => CellColor::Rgb(rgb.r, rgb.g, rgb.b),
        ansi::Color::Indexed(index) => CellColor::Indexed(index),
        ansi::Color::Named(named) => named_color(named),
    }
}

/// A named color as a color of a cell.
fn named_color(named: NamedColor) -> CellColor {
    let index = match named {
        NamedColor::Foreground
        | NamedColor::Background
        | NamedColor::Cursor
        | NamedColor::BrightForeground
        | NamedColor::DimForeground => return CellColor::Default,
        NamedColor::DimBlack => 0,
        NamedColor::DimRed => 1,
        NamedColor::DimGreen => 2,
        NamedColor::DimYellow => 3,
        NamedColor::DimBlue => 4,
        NamedColor::DimMagenta => 5,
        NamedColor::DimCyan => 6,
        NamedColor::DimWhite => 7,
        other => other as usize,
    };
    u8::try_from(index).map_or(CellColor::Default, CellColor::Indexed)
}
