//! The text of an input box being edited: one line, a caret, a selection from an anchor to the caret, and the steps to
//! undo and redo, with no drawing in it.

use std::ops::Range;

use super::words;

/// The most steps kept to undo.
const MOST_UNDO_STEPS: usize = 100;

/// Where the caret goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    /// One character back.
    Left,
    /// One character on.
    Right,
    /// To the start of the word before.
    WordLeft,
    /// To the start of the next word.
    WordRight,
    /// To the start of the line.
    Home,
    /// To the end of the line.
    End,
}

/// Whether moving the caret keeps the anchor of the selection where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selecting {
    /// The selection is dropped, and the anchor follows the caret.
    Move,
    /// The anchor stays, and the selection runs from it to the caret: Shift with the keys, or a drag.
    Extend,
}

/// How much Backspace or Delete removes when nothing is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// One character.
    Character,
    /// To the edge of a word, with Ctrl.
    Word,
}

/// The text, caret and selection at one moment, to go back to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Snapshot {
    /// The text.
    text: String,
    /// The caret.
    caret: usize,
    /// The anchor.
    anchor: usize,
}

/// What the last edit was, so that typing a word is undone in one step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LastEdit {
    /// None since the caret last moved by itself.
    #[default]
    Nothing,
    /// Characters typed one after the other.
    Typing,
    /// Characters removed one after the other.
    Removing,
}

/// One line of text being edited.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableLine {
    /// The text, never with a line break.
    text: String,
    /// The caret, a byte at the start of a character or the end.
    caret: usize,
    /// Where the selection starts; the selection is empty when it is the caret.
    anchor: usize,
    /// The states to go back to, the latest last.
    undo: Vec<Snapshot>,
    /// The states undone, to go forward to, the latest last.
    redo: Vec<Snapshot>,
    /// What the last edit was.
    last_edit: LastEdit,
}

impl EditableLine {
    /// A line with a text, line breaks made spaces, and the caret at its end.
    #[must_use]
    pub fn new(text: &str) -> EditableLine {
        let text = single_line(text);
        let end = text.len();
        EditableLine {
            text,
            caret: end,
            anchor: end,
            ..EditableLine::default()
        }
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The caret, in bytes of the text.
    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selection, in bytes of the text, from its start to its end; empty when nothing is selected.
    #[must_use]
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    /// The text selected.
    #[must_use]
    pub fn selected_text(&self) -> &str {
        self.text.get(self.selection()).unwrap_or_default()
    }

    /// Selects from an anchor to the caret, each moved to the start of the character it falls in.
    pub fn select(&mut self, anchor: usize, caret: usize) {
        self.anchor = words::floor_boundary(&self.text, anchor);
        self.caret = words::floor_boundary(&self.text, caret);
        self.last_edit = LastEdit::Nothing;
    }

    /// Selects all of the text, the caret at its end.
    pub fn select_all(&mut self) {
        self.select(0, self.text.len());
    }

    /// Selects the word at a position, the caret at its end.
    pub fn select_word_at(&mut self, position: usize) {
        let word = words::word_at(&self.text, position);
        self.select(word.start, word.end);
    }

    /// Puts the caret at a position, keeping the anchor when extending the selection.
    pub fn place_caret(&mut self, position: usize, selecting: Selecting) {
        let anchor = match selecting {
            Selecting::Move => position,
            Selecting::Extend => self.anchor,
        };
        self.select(anchor, position);
    }

    /// Moves the caret. Without extending, Left and Right over a selection go to its start or end.
    pub fn move_caret(&mut self, movement: Movement, selecting: Selecting) {
        let selection = self.selection();
        let target = match (movement, selecting) {
            (Movement::Left, Selecting::Move) if !selection.is_empty() => selection.start,
            (Movement::Right, Selecting::Move) if !selection.is_empty() => selection.end,
            (Movement::Left, _) => words::previous_boundary(&self.text, self.caret),
            (Movement::Right, _) => words::next_boundary(&self.text, self.caret),
            (Movement::WordLeft, _) => words::previous_word_start(&self.text, self.caret),
            (Movement::WordRight, _) => words::next_word_start(&self.text, self.caret),
            (Movement::Home, _) => 0,
            (Movement::End, _) => self.text.len(),
        };
        self.place_caret(target, selecting);
    }

    /// Types or pastes text over the selection; line breaks become spaces and other control characters are left
    /// out, and text made of nothing else changes nothing. Returns whether the text changed.
    pub fn insert(&mut self, typed: &str) -> bool {
        let typed = single_line(typed);
        if typed.is_empty() {
            return false;
        }
        // A word typed after a space is a step of its own to undo.
        let starts_word = typed.chars().next().is_some_and(|first| !first.is_whitespace())
            && self
                .text
                .get(..self.caret)
                .and_then(|before| before.chars().next_back())
                .is_some_and(char::is_whitespace);
        let is_continued = self.last_edit == LastEdit::Typing && self.selection().is_empty() && !starts_word;
        if !is_continued {
            self.remember();
        }
        let selection = self.selection();
        self.text.replace_range(selection.clone(), &typed);
        self.caret = selection.start + typed.len();
        self.anchor = self.caret;
        self.last_edit = LastEdit::Typing;
        true
    }

    /// Removes the selection, or what is before the caret: Backspace. Returns whether the text changed.
    pub fn delete_backward(&mut self, extent: Extent) -> bool {
        let start = match extent {
            Extent::Character => words::previous_boundary(&self.text, self.caret),
            Extent::Word => words::previous_word_start(&self.text, self.caret),
        };
        self.remove(start..self.caret)
    }

    /// Removes the selection, or what is after the caret: Delete. Returns whether the text changed.
    pub fn delete_forward(&mut self, extent: Extent) -> bool {
        let end = match extent {
            Extent::Character => words::next_boundary(&self.text, self.caret),
            Extent::Word => words::next_word_start(&self.text, self.caret),
        };
        self.remove(self.caret..end)
    }

    /// Removes the selection and returns it, for Ctrl+X; nothing when nothing is selected.
    pub fn cut(&mut self) -> Option<String> {
        let selected = self.selected_text().to_owned();
        if selected.is_empty() {
            return None;
        }
        let _ = self.remove(self.selection());
        // A cut is a step of its own to undo, not joined with the removals after it.
        self.last_edit = LastEdit::Nothing;
        Some(selected)
    }

    /// Goes back to the text before the last edit. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.redo.push(current);
        self.restore(snapshot);
        true
    }

    /// Goes forward to the text of the last edit undone. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.undo.push(current);
        self.restore(snapshot);
        true
    }

    /// Removes the selection when there is one, or a range otherwise, as one step to undo with the removals right
    /// before it.
    fn remove(&mut self, range: Range<usize>) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() { range } else { selection.clone() };
        if range.is_empty() {
            return false;
        }
        if self.last_edit != LastEdit::Removing || !selection.is_empty() {
            self.remember();
        }
        self.text.replace_range(range.clone(), "");
        self.caret = range.start;
        self.anchor = range.start;
        self.last_edit = LastEdit::Removing;
        true
    }

    /// Keeps the state before an edit, to undo it, and forgets what was undone.
    fn remember(&mut self) {
        let snapshot = self.snapshot();
        self.undo.push(snapshot);
        if self.undo.len() > MOST_UNDO_STEPS {
            let _ = self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// The state now.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        }
    }

    /// Goes back to a state.
    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.caret = snapshot.caret;
        self.anchor = snapshot.anchor;
        self.last_edit = LastEdit::Nothing;
    }
}

/// Text made one line: line breaks and tabs become spaces, other control characters are left out.
fn single_line(text: &str) -> String {
    text.chars()
        .filter_map(|character| match character {
            '\n' | '\t' => Some(' '),
            '\r' => None,
            other if other.is_control() => None,
            other => Some(other),
        })
        .collect()
}

#[cfg(test)]
mod tests;
