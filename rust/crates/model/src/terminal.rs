//! What a terminal of the bottom panel shows (`terminal.view`), as plain values: its rows as runs of cells of one look,
//! the cursor, the cells selected and the text they hold, and the modes the keys are sent in.

/// A color of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellColor {
    /// The color of the text, or of the ground behind it.
    Default,
    /// A color of the 256 of xterm: the 16 of the theme first, then a cube of 6×6×6 and 24 greys.
    Indexed(u8),
    /// A color given in red, green and blue.
    Rgb(u8, u8, u8),
}

/// How a cell looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a look of its own that a program may ask for with the others"
)]
pub struct CellStyle {
    /// The color of the text.
    pub foreground: CellColor,
    /// The color behind it.
    pub background: CellColor,
    /// Bold.
    pub is_bold: bool,
    /// Italic.
    pub is_italic: bool,
    /// Underlined.
    pub is_underlined: bool,
    /// Fainter.
    pub is_dim: bool,
    /// The text and the ground swapped.
    pub is_inverse: bool,
}

impl Default for CellStyle {
    fn default() -> CellStyle {
        CellStyle {
            foreground: CellColor::Default,
            background: CellColor::Default,
            is_bold: false,
            is_italic: false,
            is_underlined: false,
            is_dim: false,
            is_inverse: false,
        }
    }
}

/// Cells of a row that follow each other and look the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The column of the first.
    pub column: usize,
    /// How many columns they take.
    pub width: usize,
    /// Whether it is one character two columns wide, as a character of Chinese or Japanese is.
    pub is_wide: bool,
    /// What they hold.
    pub text: String,
    /// How they look.
    pub style: CellStyle,
}

/// The shape of the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    /// A block over the cell.
    Block,
    /// A line under it.
    Underline,
    /// A bar before it.
    Beam,
}

/// The cursor, where it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    /// Its row on the screen.
    pub row: usize,
    /// Its column.
    pub column: usize,
    /// Its shape.
    pub shape: CursorShape,
}

/// What a terminal shows.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a mode of its own that a program sets apart from the others"
)]
pub struct Screen {
    /// How many columns.
    pub columns: usize,
    /// The rows, from the top, each as its runs of cells; the cells with nothing in them and nothing behind are left out.
    pub rows: Vec<Vec<Span>>,
    /// The cursor, when it is shown and on the screen.
    pub cursor: Option<Cursor>,
    /// The cells selected on the screen, each row with its first column and the column after its last.
    pub selected: Vec<(usize, usize, usize)>,
    /// The text selected, when something is, also where it is not on the screen.
    pub selected_text: Option<String>,
    /// How many lines up into what it printed before the screen is scrolled.
    pub scrolled: usize,
    /// Whether the arrows send what a program that draws the whole screen asks for (`DECCKM`).
    pub is_application_cursor: bool,
    /// Whether what is pasted is wrapped in the marks that tell the program it was pasted.
    pub is_bracketed_paste: bool,
    /// Whether the program draws on the second screen, as an editor or a pager does, which has no scrollback.
    pub is_alternate_screen: bool,
}

impl Screen {
    /// What it shows as text, a line for each row, each run of cells at its column, without the blanks at the end of a
    /// row.
    #[must_use]
    pub fn text(&self) -> String {
        let rows: Vec<String> = self.rows.iter().map(|row| row_text(row)).collect();
        rows.join("\n")
    }
}

/// The text of a row: each run at its column, blanks between them.
fn row_text(row: &[Span]) -> String {
    let mut text = String::new();
    let mut column = 0;
    for span in row {
        if span.column > column {
            text.extend(std::iter::repeat_n(' ', span.column - column));
        }
        text.push_str(&span.text);
        column = span.column + span.width;
    }
    text.trim_end().to_owned()
}

/// A selection asked for in a terminal, in cells of the screen shown, each a row from its top and a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Where it starts: where the pointer was pressed.
    pub from: (usize, usize),
    /// Where it ends: where the pointer is.
    pub to: (usize, usize),
    /// Whether it is the word under `from`, for a double click.
    pub is_word: bool,
}
