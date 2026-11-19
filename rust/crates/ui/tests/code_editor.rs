//! The editor of code over several frames: a click puts the caret, typed text and keys come back as commands that the
//! test applies to the document as a page would, Ctrl+C copies the selection with its line breaks, and an editor read
//! only gives no edits.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use editor::{Command, Cursor, Document, Movement, Position};
use render::{Rect, ThemeName};
use support::{Window, click, key, shortcut, typed};
use ui::{CodeEditor, CodeEditorResponse, FrameOutput, InputEvent, LineWrap, Modifiers, NamedKey};

/// A document, the cursor of its page and whether it is read only, drawn in a window.
struct Page {
    window: Window,
    document: Document,
    cursor: Cursor,
    is_read_only: bool,
    wrap: LineWrap,
}

impl Page {
    fn new(text: &str, is_read_only: bool) -> Page {
        Page {
            window: Window::new(ThemeName::Dark, (500, 300), 1),
            document: Document::from_bytes(text.as_bytes()).unwrap(),
            cursor: Cursor::default(),
            is_read_only,
            wrap: LineWrap::Cut,
        }
    }

    /// A frame with some input: the commands it gives are done, as the page of a file does them.
    fn step(&mut self, events: &[InputEvent]) -> (CodeEditorResponse, FrameOutput) {
        let editor = CodeEditor {
            gutter: 0,
            name: "app.rs",
            buffer: self.document.buffer(),
            cursor: self.cursor,
            colors: &[],
            marks: &[],
            current_mark: None,
            lit_line: None,
            is_read_only: self.is_read_only,
            reveal: None,
            focus: None,
            tab_width: 4,
            wrap: self.wrap,
            has_line_numbers: true,
        };
        let (answer, output) = self.window.frame(events, |ui| {
            ui.code_editor("editor", Rect::new(0.0, 0.0, 500.0, 300.0), &editor)
        });
        for command in &answer.commands {
            let _changes = editor::apply(&mut self.document, &mut self.cursor, command);
        }
        self.window.wait(16);
        (answer, output)
    }

    fn text(&self) -> &str {
        self.document.buffer().text()
    }
}

#[test]
fn a_click_puts_the_caret_and_what_is_typed_goes_in_there() {
    let mut page = Page::new("fn main() {}\n", false);
    let _ = page.step(&[]);
    let (answer, _) = page.step(&click(480.0, 8.0));
    assert!(
        matches!(
            answer.commands.as_slice(),
            [Command::PlaceAt(Position { line: 0, column: 12 }, false)]
        ),
        "{:?}",
        answer.commands
    );
    let _ = page.step(&[key(NamedKey::Home, Modifiers::NONE)]);
    let _ = page.step(&[typed("pub ")]);
    assert_eq!(page.text(), "pub fn main() {}\n");
    let (answer, _) = page.step(&[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(answer.commands, [Command::Undo]);
    assert_eq!(page.text(), "fn main() {}\n");
}

#[test]
fn ctrl_c_copies_the_selection_and_shift_with_an_arrow_makes_it() {
    let mut page = Page::new("one\ntwo\n", false);
    let _ = page.step(&[]);
    let _ = page.step(&click(200.0, 8.0));
    let _ = page.step(&[key(NamedKey::Home, Modifiers::CONTROL)]);
    let (answer, _) = page.step(&[key(NamedKey::ArrowDown, Modifiers::SHIFT)]);
    assert_eq!(answer.commands, [Command::Move(Movement::Down(1), true)]);
    let (_, output) = page.step(&[shortcut('c', Modifiers::CONTROL)]);
    assert_eq!(output.clipboard.as_deref(), Some("one\n"));
}

#[test]
fn an_editor_read_only_moves_but_gives_no_edits() {
    let mut page = Page::new("let a = 1;\n", true);
    let _ = page.step(&[]);
    let _ = page.step(&click(200.0, 8.0));
    let (answer, _) = page.step(&[
        typed("x"),
        key(NamedKey::Backspace, Modifiers::NONE),
        key(NamedKey::End, Modifiers::NONE),
    ]);
    assert_eq!(answer.commands, [Command::Move(Movement::LineEnd, false)]);
    assert_eq!(page.text(), "let a = 1;\n");
}

#[test]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a few rows and pixels of a small window"
)]
fn a_line_that_wraps_takes_rows_and_a_click_or_a_selection_follows_them() {
    let long = "the quick brown fox jumps over the lazy dog ".repeat(3);
    let mut page = Page::new(&format!("{}\nnext\n", long.trim_end()), false);
    page.wrap = LineWrap::Words;
    let (answer, _) = page.step(&[]);
    let pieces = answer.rows.pieces(page.document.buffer().line(0));
    assert!(pieces.len() >= 2, "{pieces:?}");
    assert_eq!(
        answer.rows.first_row(1),
        pieces.len(),
        "the next line starts after the rows of the first"
    );
    let places = answer.places;
    let second_row = places.first_top + places.line_height * 1.5;
    let second_start = pieces.get(1).unwrap().start;
    let (answer, _) = page.step(&click(places.text.x + places.advance * 0.2, second_row));
    assert_eq!(
        answer.commands,
        [Command::PlaceAt(
            Position {
                line: 0,
                column: second_start
            },
            false
        )]
    );
    let (answer, _) = page.step(&click(
        places.text.right() - 2.0,
        places.first_top + places.line_height * 0.5,
    ));
    assert_eq!(
        answer.commands,
        [Command::PlaceAt(
            Position {
                line: 0,
                column: second_start - 1
            },
            false
        )],
        "past the end of the first row the caret stays on it"
    );
    let below = places.first_top + places.line_height * (pieces.len() as f32 + 0.5);
    let (answer, _) = page.step(&click(places.text.x + places.advance * 2.2, below));
    assert_eq!(
        answer.commands,
        [Command::PlaceAt(Position { line: 1, column: 2 }, false)]
    );
    // The bottom of a row, under its letters, at its first column.
    let x = (places.text.x + places.advance * 0.5).round() as i32;
    let bottom = |row: f32| (places.first_top + places.line_height * row - 1.0).round() as i32;
    let plain = page.window.frame.pixel(x, bottom(2.0));
    let _ = page.step(&[shortcut('a', Modifiers::CONTROL)]);
    let _ = page.step(&[]);
    let selected = page.window.frame.pixel(x, bottom(1.0));
    assert_ne!(selected, plain);
    assert_eq!(
        page.window.frame.pixel(x, bottom(2.0)),
        selected,
        "the second row is selected too"
    );
}
