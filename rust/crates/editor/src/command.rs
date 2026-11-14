//! What the keys and the pointer ask of a document and the cursor of a page, and doing it.

use crate::buffer::{Buffer, Position};
use crate::cursor::Cursor;
use crate::document::{Change, Document};

/// Where the caret goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Movement {
    /// A character to the left.
    Left,
    /// A character to the right.
    Right,
    /// To the start of the word on the left.
    WordLeft,
    /// To the end of the word on the right.
    WordRight,
    /// So many lines up, keeping the column.
    Up(usize),
    /// So many lines down, keeping the column.
    Down(usize),
    /// To the start of the text of the line, then to its very start.
    LineStart,
    /// To the end of the line.
    LineEnd,
    /// To the start of the text.
    TextStart,
    /// To the end of the text.
    TextEnd,
}

/// How much Backspace and Delete take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Extent {
    /// A character.
    Character,
    /// To the edge of a word.
    Word,
}

/// What is asked of a document and a cursor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    /// Characters typed, in place of the selection.
    Type(String),
    /// Text pasted, in place of the selection.
    Paste(String),
    /// Enter: a line break, with the spaces that start the line before.
    NewLine,
    /// Backspace.
    DeleteBackward(Extent),
    /// Delete.
    DeleteForward(Extent),
    /// The selection taken out, as cutting does once it is copied.
    DeleteSelection,
    /// The caret moved, the selection following when it extends.
    Move(Movement, bool),
    /// The caret put at a place, by the pointer, the selection following when it extends.
    PlaceAt(Position, bool),
    /// The word at a place selected, by a double click.
    SelectWord(Position),
    /// The line of a place selected, by a triple click.
    SelectLine(Position),
    /// Everything selected.
    SelectAll,
    /// The last step taken back.
    Undo,
    /// The last step taken back made again.
    Redo,
}

impl Command {
    /// Whether it changes the text, which a document read only refuses.
    #[must_use]
    pub fn is_edit(&self) -> bool {
        matches!(
            self,
            Command::Type(_)
                | Command::Paste(_)
                | Command::NewLine
                | Command::DeleteBackward(_)
                | Command::DeleteForward(_)
                | Command::DeleteSelection
                | Command::Undo
                | Command::Redo
        )
    }
}

/// Does what a command asks of a document and the cursor of a page, and says how the text changed.
pub fn apply(document: &mut Document, cursor: &mut Cursor, command: &Command) -> Vec<Change> {
    let buffer = document.buffer();
    match command {
        Command::Type(text) => vec![document.replace_selection(cursor, text, true)],
        Command::Paste(text) => vec![document.replace_selection(cursor, text, false)],
        Command::NewLine => {
            let line = buffer.line(buffer.position(cursor.range().start).line);
            let indent: String = line
                .chars()
                .take_while(|character| *character == ' ' || *character == '\t')
                .collect();
            vec![document.replace_selection(cursor, &format!("\n{indent}"), false)]
        }
        Command::DeleteBackward(extent) | Command::DeleteForward(extent) => {
            if !cursor.has_selection() {
                let is_backward = matches!(command, Command::DeleteBackward(_));
                let to = match (is_backward, extent) {
                    (true, Extent::Character) => moved(buffer, cursor, Movement::Left),
                    (true, Extent::Word) => moved(buffer, cursor, Movement::WordLeft),
                    (false, Extent::Character) => moved(buffer, cursor, Movement::Right),
                    (false, Extent::Word) => moved(buffer, cursor, Movement::WordRight),
                };
                cursor.move_to(to, true);
            }
            delete_selection(document, cursor)
        }
        Command::DeleteSelection => delete_selection(document, cursor),
        Command::Undo => document.undo(cursor),
        Command::Redo => document.redo(cursor),
        Command::Move(..)
        | Command::PlaceAt(..)
        | Command::SelectWord(_)
        | Command::SelectLine(_)
        | Command::SelectAll => {
            move_cursor(buffer, cursor, command);
            document.stop_typing();
            Vec::new()
        }
    }
}

/// Takes the selection out, when there is one.
fn delete_selection(document: &mut Document, cursor: &mut Cursor) -> Vec<Change> {
    if cursor.has_selection() {
        vec![document.replace_selection(cursor, "", false)]
    } else {
        Vec::new()
    }
}

/// Moves the cursor as a command that changes no text asks.
fn move_cursor(buffer: &Buffer, cursor: &mut Cursor, command: &Command) {
    match command {
        Command::Move(movement, is_extending) => {
            let goal = cursor.goal;
            let to = if !is_extending && cursor.has_selection() && matches!(movement, Movement::Left | Movement::Right)
            {
                if *movement == Movement::Left {
                    cursor.range().start
                } else {
                    cursor.range().end
                }
            } else {
                moved(buffer, cursor, *movement)
            };
            let keeps_goal = matches!(movement, Movement::Up(_) | Movement::Down(_));
            let column = goal.unwrap_or_else(|| character_column(buffer, cursor.head));
            cursor.move_to(to, *is_extending);
            cursor.goal = keeps_goal.then_some(column);
        }
        Command::PlaceAt(position, is_extending) => {
            cursor.move_to(buffer.offset(*position), *is_extending);
            cursor.goal = None;
        }
        Command::SelectWord(position) => {
            let at = buffer.offset(*position);
            let (start, end) = word_around(buffer, at);
            *cursor = Cursor {
                anchor: start,
                head: end,
                goal: None,
            };
        }
        Command::SelectLine(position) => {
            let range = buffer.line_range(position.line);
            let end = if position.line + 1 < buffer.line_count() {
                range.end + 1
            } else {
                range.end
            };
            *cursor = Cursor {
                anchor: range.start,
                head: end,
                goal: None,
            };
        }
        Command::SelectAll => {
            *cursor = Cursor {
                anchor: 0,
                head: buffer.text().len(),
                goal: None,
            };
        }
        _ => {}
    }
}

/// Where a movement takes the caret.
fn moved(buffer: &Buffer, cursor: &Cursor, movement: Movement) -> usize {
    let text = buffer.text();
    let head = cursor.head;
    let position = buffer.position(head);
    match movement {
        Movement::Left => text
            .get(..head)
            .and_then(|before| before.char_indices().last())
            .map_or(0, |(at, _)| at),
        Movement::Right => text
            .get(head..)
            .and_then(|after| after.chars().next())
            .map_or(head, |character| head + character.len_utf8()),
        Movement::WordLeft => word_start_before(text, head),
        Movement::WordRight => word_end_after(text, head),
        Movement::Up(lines) | Movement::Down(lines) => {
            let line = if matches!(movement, Movement::Up(_)) {
                position.line.checked_sub(lines)
            } else {
                Some(position.line + lines).filter(|line| *line < buffer.line_count())
            };
            let Some(line) = line else {
                return if matches!(movement, Movement::Up(_)) {
                    0
                } else {
                    text.len()
                };
            };
            let column = cursor.goal.unwrap_or_else(|| character_column(buffer, head));
            let range = buffer.line_range(line);
            let bytes: usize = buffer.line(line).chars().take(column).map(char::len_utf8).sum();
            range.start + bytes
        }
        Movement::LineStart => {
            let range = buffer.line_range(position.line);
            let line = buffer.line(position.line);
            let indent = line.len() - line.trim_start().len();
            if position.column == indent {
                range.start
            } else {
                range.start + indent
            }
        }
        Movement::LineEnd => buffer.line_range(position.line).end,
        Movement::TextStart => 0,
        Movement::TextEnd => text.len(),
    }
}

/// The column of a place, in characters from the start of its line.
fn character_column(buffer: &Buffer, offset: usize) -> usize {
    let position = buffer.position(offset);
    let line = buffer.line(position.line);
    line.get(..position.column).map_or(0, |before| before.chars().count())
}

/// Whether a character is part of a word.
fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// The start of the word before a place, past the spaces and the line breaks before it.
fn word_start_before(text: &str, at: usize) -> usize {
    let before = text.get(..at).unwrap_or_default();
    let mut characters = before.char_indices().rev().peekable();
    while characters.next_if(|(_, character)| character.is_whitespace()).is_some() {}
    let Some(&(_, first)) = characters.peek() else {
        return 0;
    };
    let wordy = is_word(first);
    let mut start = at;
    for (index, character) in characters {
        if character.is_whitespace() || is_word(character) != wordy {
            break;
        }
        start = index;
    }
    start.min(at)
}

/// The end of the word after a place, past the spaces before it.
fn word_end_after(text: &str, at: usize) -> usize {
    let after = text.get(at..).unwrap_or_default();
    let mut characters = after.char_indices().peekable();
    while characters.next_if(|(_, character)| character.is_whitespace()).is_some() {}
    let Some(&(_, first)) = characters.peek() else {
        return text.len();
    };
    let wordy = is_word(first);
    for (index, character) in characters {
        if character.is_whitespace() || is_word(character) != wordy {
            return at + index;
        }
    }
    text.len()
}

/// The word around a place, or the run of other characters there.
fn word_around(buffer: &Buffer, at: usize) -> (usize, usize) {
    let position = buffer.position(at);
    let range = buffer.line_range(position.line);
    let line = buffer.line(position.line);
    let wordy = line
        .get(position.column..)
        .and_then(|after| after.chars().next())
        .is_some_and(is_word);
    let start = line
        .get(..position.column)
        .unwrap_or_default()
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_word(*character) == wordy && !character.is_whitespace())
        .last()
        .map_or(position.column, |(index, _)| index);
    let end = line
        .get(position.column..)
        .unwrap_or_default()
        .char_indices()
        .find(|(_, character)| is_word(*character) != wordy || character.is_whitespace())
        .map_or(line.len(), |(index, _)| position.column + index);
    (range.start + start, range.start + end)
}
