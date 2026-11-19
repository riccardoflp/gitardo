//! The editor of code: the lines of a document with their numbers and the colors of their tokens, a caret that blinks,
//! a selection made with Shift and the arrows or by dragging, a word selected by a double click and a line by a
//! triple, the marks of what a search found, and the keys of editing. It changes nothing itself: what the keys and the
//! pointer ask comes back as [`editor::Command`]s, which the view hands to the store that keeps the document, so that
//! every page of a file shows the same text with a cursor of its own. Copying and cutting put the selection on the
//! clipboard; Ctrl+V is the application's, which hands the text of the clipboard in as a paste.
//!
//! Only the lines in sight are drawn, so that a file of a hundred thousand lines costs a screen of them. A line longer
//! than the editor is cut at its right edge, or wraps into rows as wide as the editor ([`LineWrap`]); there is no
//! scrolling across.

mod draw;
mod keys;
mod rows;

pub use draw::code_columns;
pub use keys::{KeyCommands, key_commands};
pub use rows::{CodeRows, LineWrap};

use std::hash::Hash;
use std::ops::Range;
use std::time::Duration;

use editor::{Buffer, Command, Cursor, Position, WrapWidth};
use render::{CodeKind, Point, Rect};

use super::scroll::GLIDE_FRAME;
use super::scroll::bar::BarSpec;
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::input::{Cursor as PointerShape, PointerButton};
use crate::interaction::{Response, Sense, WidgetEvent};
use crate::memory::CodeEditorState;
use crate::number;
use crate::widget::panel::Axis;

/// A part of a line in the color of a kind of token, by its bytes in the line.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeSpan {
    /// Where it is in its line, in bytes.
    pub range: Range<usize>,
    /// What it is.
    pub kind: CodeKind,
}

/// An editor of code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeEditor<'a> {
    /// What it is, for the screen reader: the name of the file.
    pub name: &'a str,
    /// The text.
    pub buffer: &'a Buffer,
    /// The caret and the selection of this editor.
    pub cursor: Cursor,
    /// The colored parts of each line, by line; a line past the end of the list is plain.
    pub colors: &'a [Vec<CodeSpan>],
    /// What a search found, as bytes of the text, in order.
    pub marks: &'a [Range<usize>],
    /// The mark that is current, by its place among them.
    pub current_mark: Option<usize>,
    /// A line lit across the editor, from zero: the line a search found, in its preview.
    pub lit_line: Option<usize>,
    /// Whether typing does nothing: the file at a revision.
    pub is_read_only: bool,
    /// A line to bring to the middle, once for each number: the line a page opens at, a match gone to.
    pub reveal: Option<(u64, usize)>,
    /// Takes the keyboard, once for each number: when its page opens, and when a dialog over it closes.
    pub focus: Option<u64>,
    /// How many columns of room are kept between the line numbers and the text, for what a view draws beside each
    /// line, such as the blame of a file; none keeps no room.
    pub gutter: usize,
    /// How many columns a tab goes to.
    pub tab_width: usize,
    /// What it does with a line longer than it is wide.
    pub wrap: LineWrap,
    /// Whether the numbers of the lines are drawn before them: not in a box where a question is written.
    pub has_line_numbers: bool,
}

/// Where an editor drew its parts, for a view that draws beside its lines or over its scroll bar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CodePlaces {
    /// The room kept between the line numbers and the text, as tall as the editor; empty when none is kept.
    pub gutter: Rect,
    /// The text.
    pub text: Rect,
    /// The height of a line.
    pub line_height: f32,
    /// The width of a column of the font of code.
    pub advance: f32,
    /// Where the first line in sight starts, down: above the top of the text when it is scrolled into.
    pub first_top: f32,
    /// The track of the scroll bar.
    pub track: Rect,
}

/// What an editor tells its view.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeEditorResponse {
    /// What happened to the editor as a whole.
    pub response: Response,
    /// What the keys and the pointer ask of the document, in order.
    pub commands: Vec<Command>,
    /// Where the menu of the context was asked for, by its button.
    pub context_menu: Option<Point>,
    /// The lines in sight.
    pub visible_lines: Range<usize>,
    /// Where its parts are.
    pub places: CodePlaces,
    /// The rows its lines take, for a view that draws beside them: a line that wraps takes several.
    pub rows: CodeRows,
}

/// How long the caret shows, and then how long it hides, while it blinks.
const BLINK: Duration = Duration::from_millis(530);

/// The places of the parts of an editor: its line numbers, its text and its scroll bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    /// The column of the line numbers.
    pub(crate) numbers: Rect,
    /// The room kept before the text.
    pub(crate) gutter: Rect,
    /// The text.
    pub(crate) text: Rect,
    /// The height of a line.
    pub(crate) line_height: f32,
    /// The width of a column of the font of code.
    pub(crate) advance: f32,
}

impl Ui<'_> {
    /// Declares an editor of code over a rectangle, and says what was asked of it.
    pub fn code_editor(&mut self, key: impl Hash, rect: Rect, editor: &CodeEditor<'_>) -> CodeEditorResponse {
        let id = self.id(key);
        let mut state = self.memory().code_editors.remove(&id).unwrap_or_default();
        if let Some(serial) = editor.focus.filter(|serial| state.focused != Some(*serial)) {
            state.focused = Some(serial);
            self.request_focus(id);
        }
        let sense = Sense::CLICK | Sense::DRAG | Sense::FOCUS | Sense::SCROLL;
        let (response, events) = self.interact_with_events(id, rect, sense);
        let layout = self.code_layout(rect, editor);
        let rows = CodeRows::new(editor.buffer, wrap_width(editor, layout));
        reveal_asked(&mut state, editor, (layout, &rows));
        let mut outcome = CodeEditorResponse {
            response,
            commands: Vec::new(),
            context_menu: None,
            visible_lines: 0..0,
            places: CodePlaces::default(),
            rows: CodeRows::default(),
        };
        let page = number::index_at(f64::from(layout.text.height), f64::from(layout.line_height)).max(1);
        for event in events {
            self.code_editor_event((editor, layout, page, &rows), &mut state, (event, &mut outcome));
        }
        self.follow_caret(&mut state, editor, (layout, &rows));
        let content = number::wide_of_count(rows.count()) * f64::from(layout.line_height);
        state.scroll.clamp(content - f64::from(layout.text.height));
        if state.scroll.advance(self.now()) {
            let next = self.now() + GLIDE_FRAME;
            self.request_frame_at(next);
        }
        if outcome.response.hovered {
            self.set_cursor(PointerShape::Text);
        }
        let (visible_lines, first_top) = self.draw_code(editor, (layout, &rows), &state, &outcome.response);
        self.access_editor((id, rect), editor, visible_lines.clone());
        outcome.visible_lines = visible_lines;
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: f64::from(layout.text.height),
            content,
        };
        let thickness = self.theme().sizes.scrollbar;
        let track = Rect::new(rect.right() - thickness, rect.y, thickness, rect.height);
        outcome.places = CodePlaces {
            gutter: layout.gutter,
            text: layout.text,
            line_height: layout.line_height,
            advance: layout.advance,
            first_top,
            track,
        };
        self.scroll_bar(id.with("bar"), track, spec, &mut state.scroll);
        let _ = self.memory().code_editors.insert(id, state);
        outcome.rows = rows;
        outcome
    }

    /// Tells the screen reader of an editor, with the lines in sight as its value.
    fn access_editor(
        &mut self,
        (id, rect): (crate::id::WidgetId, Rect),
        editor: &CodeEditor<'_>,
        visible: Range<usize>,
    ) {
        let shown: Vec<&str> = visible.map(|line| editor.buffer.line(line)).collect();
        let value = shown.join("\n");
        self.access_node(
            id,
            rect,
            &Description::new(AccessRole::TextInput, editor.name).value(&value),
        );
    }

    /// Where the parts of an editor go over its rectangle: the line numbers as wide as the largest, five digits at
    /// least, when it has them, then the room it keeps before the text, if any.
    fn code_layout(&self, rect: Rect, editor: &CodeEditor<'_>) -> Layout {
        let theme = self.theme();
        let style = self.text_style(TextKind::Code);
        let advance = self.measure("0000000000", style) / 10.0;
        let digits = editor.buffer.line_count().to_string().len().max(5);
        let numbers_width = if editor.has_line_numbers {
            advance * number::float_of_count(digits) + 2.0 * theme.sizes.spacing.medium
        } else {
            0.0
        };
        let gutter_width = if editor.gutter == 0 {
            0.0
        } else {
            advance * number::float_of_count(editor.gutter) + 2.0 * theme.sizes.spacing.medium
        };
        let gutter = Rect::new(rect.x + numbers_width, rect.y, gutter_width, rect.height);
        let text_x = gutter.right() + theme.sizes.spacing.medium;
        let text_width = (rect.right() - theme.sizes.scrollbar - text_x).max(0.0);
        Layout {
            numbers: Rect::new(rect.x, rect.y, numbers_width, rect.height),
            gutter,
            text: Rect::new(text_x, rect.y, text_width, rect.height),
            line_height: self.line_height(style) + 4.0,
            advance,
        }
    }

    /// Scrolls the least that keeps the caret in sight when it moved since the last frame, and starts its blink again.
    fn follow_caret(&self, state: &mut CodeEditorState, editor: &CodeEditor<'_>, (layout, rows): (Layout, &CodeRows)) {
        if state.head == Some(editor.cursor.head) {
            return;
        }
        let is_first = state.head.is_none();
        state.head = Some(editor.cursor.head);
        state.changed_at = self.now();
        if is_first {
            return;
        }
        let height = f64::from(layout.line_height);
        let top = number::wide_of_count(caret_row(editor, rows)) * height;
        state.scroll.reveal((top, top + height), f64::from(layout.text.height));
    }

    /// Applies an event to an editor: a press places the caret or selects, a drag selects, the wheel scrolls, a key
    /// moves or edits, and typed or pasted text goes in, unless the editor is read only.
    fn code_editor_event(
        &mut self,
        (editor, layout, page, rows): (&CodeEditor<'_>, Layout, usize, &CodeRows),
        state: &mut CodeEditorState,
        (event, outcome): (WidgetEvent, &mut CodeEditorResponse),
    ) {
        let commands = &mut outcome.commands;
        match event {
            WidgetEvent::Pressed {
                button,
                position,
                modifiers,
                click_count,
            } => {
                let at = code_position(editor, (layout, rows), state, position);
                let offset = editor.buffer.offset(at);
                match (button, click_count) {
                    (PointerButton::Primary, 1) => commands.push(Command::PlaceAt(at, modifiers.shift())),
                    (PointerButton::Primary, 2) => commands.push(Command::SelectWord(at)),
                    (PointerButton::Primary, _) => commands.push(Command::SelectLine(at)),
                    (PointerButton::Secondary, _) => {
                        if !editor.cursor.range().contains(&offset) {
                            commands.push(Command::PlaceAt(at, false));
                        }
                        outcome.context_menu = Some(position);
                    }
                    _ => {}
                }
            }
            WidgetEvent::Dragged { position, .. } => {
                let at = code_position(editor, (layout, rows), state, position);
                commands.push(Command::PlaceAt(at, true));
            }
            WidgetEvent::Wheel { delta } => state.scroll.scroll_by(f64::from(delta.y), self.now()),
            WidgetEvent::Key(key) => {
                let asked = keys::key_commands(key, page, editor.is_read_only);
                if asked.is_used {
                    self.claim_key(outcome.response.id, key);
                }
                if asked.copies {
                    let copied = editor.buffer.slice_as_file(editor.cursor.range());
                    if !copied.is_empty() {
                        self.copy_to_clipboard(copied);
                    }
                }
                outcome.commands.extend(asked.commands);
            }
            WidgetEvent::Text(text) if !editor.is_read_only => {
                let typed: String = text.chars().filter(|character| !character.is_control()).collect();
                if !typed.is_empty() {
                    commands.push(Command::Type(typed));
                }
            }
            WidgetEvent::Paste(text) if !editor.is_read_only => commands.push(Command::Paste(text)),
            // The other events do not change an editor.
            _ => {}
        }
    }
}

/// The room of a row while the lines of an editor wrap: as many columns as its text has; none while they are cut.
fn wrap_width(editor: &CodeEditor<'_>, layout: Layout) -> Option<WrapWidth> {
    match editor.wrap {
        LineWrap::Cut => None,
        LineWrap::Words => Some(WrapWidth {
            columns: number::index_at(f64::from(layout.text.width), f64::from(layout.advance)).max(1),
            tab_width: editor.tab_width,
        }),
    }
}

/// The row the caret is on, from the first row of the text.
fn caret_row(editor: &CodeEditor<'_>, rows: &CodeRows) -> usize {
    let position = editor.buffer.position(editor.cursor.head);
    let pieces = rows.pieces(editor.buffer.line(position.line));
    rows.first_row(position.line) + editor::row_of(&pieces, position.column)
}

/// Brings to the middle the line the view asks for, once for each number it gives.
fn reveal_asked(state: &mut CodeEditorState, editor: &CodeEditor<'_>, (layout, rows): (Layout, &CodeRows)) {
    let Some((serial, line)) = editor.reveal.filter(|(serial, _)| state.revealed != Some(*serial)) else {
        return;
    };
    state.revealed = Some(serial);
    let height = f64::from(layout.line_height);
    let top = number::wide_of_count(rows.first_row(line)) * height;
    state
        .scroll
        .jump((top + height * 0.5 - f64::from(layout.text.height) * 0.5).max(0.0));
    state.head = Some(editor.cursor.head);
}

/// The place in the text nearest to a point of the window, on the row of a line under it: past the end of a row that
/// wraps, the place before its last character, so that the caret stays on that row.
fn code_position(
    editor: &CodeEditor<'_>,
    (layout, rows): (Layout, &CodeRows),
    state: &CodeEditorState,
    point: Point,
) -> Position {
    let y = f64::from(point.y - layout.text.y) + state.scroll.offset();
    let (line, row) = rows.line_at(number::index_at(y.max(0.0), f64::from(layout.line_height)));
    let wanted = ((point.x - layout.text.x) / layout.advance).max(0.0);
    let text = editor.buffer.line(line);
    let pieces = rows.pieces(text);
    let piece = pieces.get(row).cloned().unwrap_or(0..text.len());
    let mut columns = 0.0_f32;
    let mut last = piece.start;
    for (index, character) in text.get(piece.clone()).unwrap_or_default().char_indices() {
        let width = number::float_of_count(draw::columns_of(character, columns, editor.tab_width));
        if wanted < columns + width * 0.5 {
            return Position {
                line,
                column: piece.start + index,
            };
        }
        columns += width;
        last = piece.start + index;
    }
    let is_last_row = row + 1 >= pieces.len();
    Position {
        line,
        column: if is_last_row { piece.end } else { last },
    }
}

/// Whether the caret shows at a moment of its blink, and the moment it turns next.
pub(crate) fn blink(changed_at: Duration, now: Duration) -> (bool, Duration) {
    let since = now.saturating_sub(changed_at);
    let phases = crate::number::steps(since.as_millis(), BLINK.as_millis());
    let next = changed_at + BLINK * u32::try_from(phases + 1).unwrap_or(u32::MAX);
    (phases.is_multiple_of(2), next)
}
