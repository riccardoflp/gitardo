//! The text of a file being edited: its lines, kept with `\n` between them, and the line break each had in the file,
//! so that the file is written back with its own line endings, its byte order mark, and no line break added at its end.

use std::ops::Range;

/// A line break as the file has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Ending {
    /// `\n`.
    #[default]
    Line,
    /// `\r\n`.
    ReturnLine,
}

impl Ending {
    /// Its bytes.
    #[must_use]
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Ending::Line => b"\n",
            Ending::ReturnLine => b"\r\n",
        }
    }
}

/// Why the bytes of a file cannot be edited as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotText {
    /// It has a NUL byte: it is not text.
    Binary,
}

/// The bytes a file starts with when it has a byte order mark in UTF-8.
const BYTE_ORDER_MARK: &[u8] = &[0xef, 0xbb, 0xbf];

/// A place in the text: a line, from zero, and a byte of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Position {
    /// The line, from zero.
    pub line: usize,
    /// The byte in the line, on the boundary of a character.
    pub column: usize,
}

/// What a replacement did to the lines: from which line, how many lines it took away and how many it put in their
/// place, for what colors the lines again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinesChanged {
    /// The first line changed.
    pub start: usize,
    /// How many lines were there.
    pub removed: usize,
    /// How many lines are there now.
    pub added: usize,
}

/// What a replacement took out of the text, to put it back.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Removed {
    /// The text taken out, with `\n` for its line breaks.
    pub text: String,
    /// The line break of the file at each of its `\n`.
    pub endings: Vec<Ending>,
}

/// The text of a file, as lines between `\n`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Buffer {
    /// The text, with `\n` for every line break.
    text: String,
    /// Where each line starts in the text; the first at 0.
    starts: Vec<usize>,
    /// The line break of the file at each `\n` of the text, in order.
    endings: Vec<Ending>,
    /// The line break typed lines take: that of most lines of the file.
    usual: Ending,
    /// Whether the file starts with a byte order mark, which the text leaves out.
    has_byte_order_mark: bool,
}

impl Buffer {
    /// The text of the bytes of a file; what is not UTF-8 is replaced, and a file with a NUL byte is not text.
    ///
    /// # Errors
    ///
    /// When the file has a NUL byte.
    pub fn from_bytes(bytes: &[u8]) -> Result<Buffer, NotText> {
        if bytes.contains(&0) {
            return Err(NotText::Binary);
        }
        let has_byte_order_mark = bytes.starts_with(BYTE_ORDER_MARK);
        let body = bytes
            .get(BYTE_ORDER_MARK.len()..)
            .filter(|_| has_byte_order_mark)
            .unwrap_or(bytes);
        let decoded = String::from_utf8_lossy(body);
        let (text, endings) = without_returns(&decoded);
        let returns = endings.iter().filter(|ending| **ending == Ending::ReturnLine).count();
        let usual = if returns * 2 > endings.len() {
            Ending::ReturnLine
        } else {
            Ending::Line
        };
        let mut buffer = Buffer {
            text,
            starts: Vec::new(),
            endings,
            usual,
            has_byte_order_mark,
        };
        buffer.starts = line_starts(&buffer.text);
        Ok(buffer)
    }

    /// The bytes of the file: the text with the line break of the file at each line, and its byte order mark.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.text.len() + self.endings.len() + BYTE_ORDER_MARK.len());
        if self.has_byte_order_mark {
            bytes.extend_from_slice(BYTE_ORDER_MARK);
        }
        let mut endings = self.endings.iter();
        for piece in self.text.split_inclusive('\n') {
            match piece.strip_suffix('\n') {
                Some(line) => {
                    bytes.extend_from_slice(line.as_bytes());
                    bytes.extend_from_slice(endings.next().copied().unwrap_or(self.usual).bytes());
                }
                None => bytes.extend_from_slice(piece.as_bytes()),
            }
        }
        bytes
    }

    /// The text, with `\n` for every line break.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many lines it has: one more than its line breaks.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// A line, without its line break; empty past the last.
    #[must_use]
    pub fn line(&self, line: usize) -> &str {
        self.text.get(self.line_range(line)).unwrap_or_default()
    }

    /// Where a line is in the text, without its line break.
    #[must_use]
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let Some(&start) = self.starts.get(line) else {
            return self.text.len()..self.text.len();
        };
        let end = self.starts.get(line + 1).map_or(self.text.len(), |next| next - 1);
        start..end
    }

    /// The place of a byte of the text, a byte past the end being the end of the last line.
    #[must_use]
    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.text.len());
        let line = self.starts.partition_point(|start| *start <= offset).saturating_sub(1);
        let start = self.starts.get(line).copied().unwrap_or(0);
        Position {
            line,
            column: offset - start,
        }
    }

    /// The byte of the text of a place, kept inside its line and on the boundary of a character.
    #[must_use]
    pub fn offset(&self, position: Position) -> usize {
        let line = position.line.min(self.line_count().saturating_sub(1));
        let range = self.line_range(line);
        self.boundary_at_or_before((range.start + position.column).min(range.end))
    }

    /// The boundary of a character at a byte of the text, or the one before it.
    #[must_use]
    pub fn boundary_at_or_before(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    /// The text between two bytes, with `\n` for its line breaks.
    #[must_use]
    pub fn slice(&self, range: Range<usize>) -> &str {
        self.text.get(range).unwrap_or_default()
    }

    /// The text between two bytes as the file has it, with its own line breaks: what is copied.
    #[must_use]
    pub fn slice_as_file(&self, range: Range<usize>) -> String {
        let first = self
            .text
            .get(..range.start)
            .map_or(0, |before| before.matches('\n').count());
        let mut endings = self.endings.iter().skip(first);
        let mut copied = String::new();
        for piece in self.slice(range).split_inclusive('\n') {
            match piece.strip_suffix('\n') {
                Some(line) => {
                    copied.push_str(line);
                    copied.push_str(match endings.next().copied().unwrap_or(self.usual) {
                        Ending::Line => "\n",
                        Ending::ReturnLine => "\r\n",
                    });
                }
                None => copied.push_str(piece),
            }
        }
        copied
    }

    /// The line break typed lines take.
    #[must_use]
    pub fn usual_ending(&self) -> Ending {
        self.usual
    }

    /// Replaces the text between two bytes by another, whose line breaks, `\n`, `\r\n` or `\r`, become `\n`. Its line
    /// breaks are those given, as an undo gives back those it took; the line break of the file, otherwise. Says what it
    /// took out and which lines changed.
    pub fn replace(
        &mut self,
        range: Range<usize>,
        inserted: &str,
        endings: Option<&[Ending]>,
    ) -> (Removed, LinesChanged) {
        let start = self.boundary_at_or_before(range.start);
        let end = self.boundary_at_or_before(range.end.max(start));
        let (inserted, typed_endings) = without_returns(inserted);
        let first_break = self.text.get(..start).map_or(0, |before| before.matches('\n').count());
        let removed_text = self.slice(start..end).to_owned();
        let removed_breaks = removed_text.matches('\n').count();
        let added_breaks = inserted.matches('\n').count();
        let usual = self.usual;
        let added_endings: Vec<Ending> = match endings {
            Some(given) => (0..added_breaks)
                .map(|at| given.get(at).copied().unwrap_or(usual))
                .collect(),
            None => vec![usual; typed_endings.len()],
        };
        let removed_endings: Vec<Ending> = self
            .endings
            .splice(first_break..first_break + removed_breaks, added_endings)
            .collect();
        self.text.replace_range(start..end, &inserted);
        self.starts = line_starts(&self.text);
        let changed = LinesChanged {
            start: first_break,
            removed: removed_breaks + 1,
            added: added_breaks + 1,
        };
        let removed = Removed {
            text: removed_text,
            endings: removed_endings,
        };
        (removed, changed)
    }
}

/// A text with its line breaks made `\n`, and the line break each was.
fn without_returns(text: &str) -> (String, Vec<Ending>) {
    let mut plain = String::with_capacity(text.len());
    let mut endings = Vec::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' if characters.peek() == Some(&'\n') => {
                let _ = characters.next();
                plain.push('\n');
                endings.push(Ending::ReturnLine);
            }
            '\r' | '\n' => {
                plain.push('\n');
                endings.push(Ending::Line);
            }
            other => plain.push(other),
        }
    }
    (plain, endings)
}

/// Where each line of a text starts.
fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}
