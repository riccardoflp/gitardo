//! A file open in the editor: its text, what was done to it in steps that undo and redo, and whether it differs from
//! what was saved. Every page of a file shows the same document, each with its own cursor.

use std::ops::Range;

use crate::buffer::{Buffer, Ending, LinesChanged, NotText, Removed};
use crate::cursor::Cursor;

/// One replacement of a step, with what it took out, to be undone.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Replacement {
    /// Where it starts in the text.
    at: usize,
    /// What it took out.
    removed: Removed,
    /// What it put in.
    inserted: Removed,
}

/// What a step was, which decides whether typing goes on in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StepKind {
    /// Characters typed: the spaces before a word, and the word.
    Typing,
    /// Anything else: a paste, a deletion, a line break, a change of the whole text by a button of a page.
    Other,
}

/// What Undo takes back at once.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Step {
    /// Its number, unique in its document.
    id: u64,
    /// What it was.
    kind: StepKind,
    /// The replacements, in the order they were made.
    replacements: Vec<Replacement>,
    /// The cursor before it.
    before: Cursor,
    /// The cursor after it.
    after: Cursor,
}

/// A change of the text, for the others who show it: the cursors of the other pages, the colors of the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Change {
    /// Where it starts in the text.
    pub at: usize,
    /// How many bytes it took out.
    pub removed: usize,
    /// How many bytes it put in.
    pub inserted: usize,
    /// The lines it changed.
    pub lines: LinesChanged,
}

/// A file open in the editor.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Document {
    /// Its text.
    buffer: Buffer,
    /// The steps Undo takes back, the last last.
    undo: Vec<Step>,
    /// The steps Redo makes again, the next last.
    redo: Vec<Step>,
    /// The number of the next step.
    next: u64,
    /// The step last done when the text was saved or read; none for the text before any step.
    saved: Option<u64>,
    /// Whether typing goes on in the last step: nothing else was done since.
    is_typing: bool,
}

impl Document {
    /// The document of the bytes of a file.
    ///
    /// # Errors
    ///
    /// When the file is not text.
    pub fn from_bytes(bytes: &[u8]) -> Result<Document, NotText> {
        Ok(Document {
            buffer: Buffer::from_bytes(bytes)?,
            ..Document::default()
        })
    }

    /// Its text.
    #[must_use]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Whether it differs from what was saved: Undo back to what was saved makes it the same again.
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        self.undo.last().map(|step| step.id) != self.saved
    }

    /// Takes the text as it is as what is saved.
    pub fn mark_saved(&mut self) {
        self.saved = self.undo.last().map(|step| step.id);
        self.is_typing = false;
    }

    /// Takes the bytes of the file as its text, as read again from the disk, keeping the cursor where it can: the steps
    /// done before are forgotten.
    ///
    /// # Errors
    ///
    /// When the file is no longer text.
    pub fn reload(&mut self, bytes: &[u8]) -> Result<(), NotText> {
        *self = Document::from_bytes(bytes)?;
        Ok(())
    }

    /// Whether there is a step to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is a step to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Replaces the text of the selection of a cursor by another, as one step: typing goes on in the last step when
    /// it is typing too, it follows the last thing typed, and it is not a space after a word.
    pub(crate) fn replace_selection(&mut self, cursor: &mut Cursor, inserted: &str, is_typed: bool) -> Change {
        let range = cursor.range();
        let goes_on = is_typed && self.is_typing && self.continues_typing(range.start, inserted);
        let before = *cursor;
        let (removed, lines) = self.buffer.replace(range.clone(), inserted, None);
        let inserted_text = self
            .buffer
            .slice(range.start..range.start + inserted_length(inserted))
            .to_owned();
        let inserted_endings = vec![self.buffer.usual_ending(); inserted_text.matches('\n').count()];
        let change = Change {
            at: range.start,
            removed: removed.text.len(),
            inserted: inserted_text.len(),
            lines,
        };
        cursor.place(range.start + inserted_text.len());
        let replacement = Replacement {
            at: range.start,
            removed,
            inserted: Removed {
                text: inserted_text,
                endings: inserted_endings,
            },
        };
        self.redo.clear();
        match self.undo.last_mut() {
            Some(step) if goes_on => {
                step.replacements.push(replacement);
                step.after = *cursor;
            }
            _ => {
                let id = self.next;
                self.next += 1;
                self.undo.push(Step {
                    id,
                    kind: if is_typed { StepKind::Typing } else { StepKind::Other },
                    replacements: vec![replacement],
                    before,
                    after: *cursor,
                });
            }
        }
        self.is_typing = is_typed;
        change
    }

    /// Whether text typed at a place goes on in the last step: it follows what the step typed last, and it is not a
    /// space after a word.
    fn continues_typing(&self, at: usize, typed: &str) -> bool {
        let Some(last) = self.undo.last().filter(|step| step.kind == StepKind::Typing) else {
            return false;
        };
        let Some(replacement) = last.replacements.last() else {
            return false;
        };
        let follows = replacement.at + replacement.inserted.text.len() == at;
        let after_word = replacement
            .inserted
            .text
            .ends_with(|character: char| !character.is_whitespace());
        let starts_space = typed.starts_with(char::is_whitespace);
        follows && !(after_word && starts_space)
    }

    /// Says that something other than typing was done, so that what is typed next is a step of its own.
    pub(crate) fn stop_typing(&mut self) {
        self.is_typing = false;
    }

    /// Takes back the last step, and puts the cursor where it was before it.
    pub(crate) fn undo(&mut self, cursor: &mut Cursor) -> Vec<Change> {
        let Some(step) = self.undo.pop() else {
            return Vec::new();
        };
        let changes = step
            .replacements
            .iter()
            .rev()
            .map(|replacement| {
                let range = replacement.at..replacement.at + replacement.inserted.text.len();
                self.apply(range, &replacement.removed)
            })
            .collect();
        *cursor = step.before;
        self.redo.push(step);
        self.is_typing = false;
        changes
    }

    /// Makes the last step taken back again, and puts the cursor where it was after it.
    pub(crate) fn redo(&mut self, cursor: &mut Cursor) -> Vec<Change> {
        let Some(step) = self.redo.pop() else {
            return Vec::new();
        };
        let changes = step
            .replacements
            .iter()
            .map(|replacement| {
                let range = replacement.at..replacement.at + replacement.removed.text.len();
                self.apply(range, &replacement.inserted)
            })
            .collect();
        *cursor = step.after;
        self.undo.push(step);
        self.is_typing = false;
        changes
    }

    /// Puts a text with its line breaks in place of a range.
    fn apply(&mut self, range: Range<usize>, text: &Removed) -> Change {
        let endings: &[Ending] = &text.endings;
        let (removed, lines) = self.buffer.replace(range.clone(), &text.text, Some(endings));
        Change {
            at: range.start,
            removed: removed.text.len(),
            inserted: text.text.len(),
            lines,
        }
    }
}

/// The length of a text once its line breaks are `\n`.
fn inserted_length(text: &str) -> usize {
    text.len() - text.matches("\r\n").count()
}
