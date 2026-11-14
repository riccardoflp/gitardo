//! The cursor of a page in a document: where the caret is, and where the selection started, as bytes of the text.

use std::ops::Range;

use crate::document::Change;

/// The caret and the selection of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Cursor {
    /// Where the selection started; the caret itself when nothing is selected.
    pub anchor: usize,
    /// Where the caret is.
    pub head: usize,
    /// The column, in characters, the arrows up and down keep to across shorter lines.
    pub goal: Option<usize>,
}

impl Cursor {
    /// A caret at a place, with nothing selected.
    #[must_use]
    pub fn at(offset: usize) -> Cursor {
        Cursor {
            anchor: offset,
            head: offset,
            goal: None,
        }
    }

    /// What is selected, from its start to its end.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    /// Whether something is selected.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.anchor != self.head
    }

    /// Puts the caret at a place, with nothing selected.
    pub fn place(&mut self, offset: usize) {
        *self = Cursor::at(offset);
    }

    /// Moves the caret to a place, the selection following when it extends.
    pub fn move_to(&mut self, offset: usize, is_extending: bool) {
        self.head = offset;
        if !is_extending {
            self.anchor = offset;
        }
    }

    /// Keeps the cursor on the same text after a change another page made: after it, it moves with the text; inside
    /// what it took out, it goes to the end of what it put in.
    pub fn follow(&mut self, change: &Change) {
        let shift = |offset: usize| {
            if offset <= change.at {
                offset
            } else if offset >= change.at + change.removed {
                offset - change.removed + change.inserted
            } else {
                change.at + change.inserted
            }
        };
        self.anchor = shift(self.anchor);
        self.head = shift(self.head);
        self.goal = None;
    }
}
