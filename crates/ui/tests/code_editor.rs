//! The editor of code over several frames: a click puts the caret, typed text and keys come back as commands that the
//! test applies to the document as a page would, Ctrl+C copies the selection with its line breaks, and an editor read
//! only gives no edits.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use editor::{Command, Cursor, Document, Movement, Position};
use render::{Rect, ThemeName};
use support::{Window, click, key, shortcut, typed};
use ui::{CodeEditor, CodeEditorResponse, FrameOutput, InputEvent, Modifiers, NamedKey};

/// A document, the cursor of its page and whether it is read only, drawn in a window.
struct Page {
    window: Window,
    document: Document,
    cursor: Cursor,
    is_read_only: bool,
}

impl Page {
    fn new(text: &str, is_read_only: bool) -> Page {
        Page {
            window: Window::new(ThemeName::Dark, (500, 300), 1),
            document: Document::from_bytes(text.as_bytes()).unwrap(),
            cursor: Cursor::default(),
            is_read_only,
        }
    }

    /// A frame with some input: the commands it gives are done, as the page of a file does them.
    fn step(&mut self, events: &[InputEvent]) -> (CodeEditorResponse, FrameOutput) {
        let editor = CodeEditor {
            name: "app.rs",
            buffer: self.document.buffer(),
            cursor: self.cursor,
            colors: &[],
            marks: &[],
            current_mark: None,
            is_read_only: self.is_read_only,
            reveal: None,
            tab_width: 4,
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
