//! Reading JSON text, strict as RFC 8259 says or with the comments and trailing commas of settings files, into values
//! that remember where in the text they were written.

use crate::error::{Error, Position, Problem};
use crate::value::{Member, Value};

mod scalar;

/// How deep arrays and objects may be nested, so that a hostile text gives an error instead of exhausting the stack.
const NESTING_LIMIT: usize = 512;

/// The byte order mark some editors write at the start of a file, which RFC 8259 lets a reader ignore.
pub(crate) const BYTE_ORDER_MARK: &str = "\u{feff}";

/// Reads a text that must be strict JSON, as RFC 8259 defines it, such as an answer from a web service.
///
/// # Errors
///
/// [`Error::Syntax`] with the line, the column and the problem, when the text is not valid JSON: comments and
/// trailing commas included.
pub fn parse(text: &str) -> Result<Value, Error> {
    Ok(parse_document(text, Dialect::Strict)?.into_value())
}

/// Reads a text of JSON with comments, as settings files are written: JSON where `//` and `/* */` comments and a
/// comma after the last item of an array or object are allowed.
///
/// # Errors
///
/// [`Error::Syntax`] with the line, the column and the problem, when the text is not valid JSON with comments.
pub fn parse_with_comments(text: &str) -> Result<Value, Error> {
    Ok(parse_document(text, Dialect::WithComments)?.into_value())
}

/// Which JSON a text is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// RFC 8259, nothing more.
    Strict,
    /// RFC 8259 with comments and trailing commas.
    WithComments,
}

/// A value read from a text, with the bytes it was written in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    /// What the value is.
    pub(crate) kind: NodeKind,
    /// The offset of its first byte.
    pub(crate) start: usize,
    /// The offset just after its last byte: after the closing brace or bracket of an object or array.
    pub(crate) end: usize,
}

/// What a value read from a text is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// Anything but an array or an object.
    Scalar(Value),
    /// An array, its items as entries with no name.
    Array(Vec<Entry>),
    /// An object, its members as entries with their names.
    Object(Vec<Entry>),
}

/// One item of an array or member of an object, as written in a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The name of a member, or nothing for an item of an array.
    pub(crate) name: Option<String>,
    /// The offset of its first byte: the quote of the name of a member, the value of an item.
    pub(crate) start: usize,
    /// Its value.
    pub(crate) value: Node,
    /// The offset of the comma after it, when it has one.
    pub(crate) comma: Option<usize>,
}

impl Node {
    /// The plain value, without where it was written.
    pub(crate) fn into_value(self) -> Value {
        match self.kind {
            NodeKind::Scalar(value) => value,
            NodeKind::Array(entries) => {
                Value::Array(entries.into_iter().map(|entry| entry.value.into_value()).collect())
            }
            NodeKind::Object(entries) => Value::Object(
                entries
                    .into_iter()
                    .map(|entry| Member {
                        name: entry.name.unwrap_or_default(),
                        value: entry.value.into_value(),
                    })
                    .collect(),
            ),
        }
    }
}

/// Reads a whole text: one value, with only white space, and comments if the dialect allows them, around it.
pub(crate) fn parse_document(text: &str, dialect: Dialect) -> Result<Node, Error> {
    let start = if text.starts_with(BYTE_ORDER_MARK) {
        BYTE_ORDER_MARK.len()
    } else {
        0
    };
    let mut parser = Parser {
        text,
        position: start,
        dialect,
    };
    parser.skip_trivia()?;
    let node = parser.parse_value()?;
    parser.skip_trivia()?;
    if parser.position < text.len() {
        return Err(parser.error(Problem::TextAfterValue));
    }
    Ok(node)
}

/// Whether a text of JSON with comments holds no value at all, only white space and comments.
pub(crate) fn holds_no_value(text: &str) -> Result<bool, Error> {
    let start = if text.starts_with(BYTE_ORDER_MARK) {
        BYTE_ORDER_MARK.len()
    } else {
        0
    };
    let mut parser = Parser {
        text,
        position: start,
        dialect: Dialect::WithComments,
    };
    parser.skip_trivia()?;
    Ok(parser.position == text.len())
}

/// Whether a text holds only white space, and so no value at all.
pub(crate) fn is_blank(text: &str) -> bool {
    text.strip_prefix(BYTE_ORDER_MARK)
        .unwrap_or(text)
        .bytes()
        .all(is_white_space)
}

/// Whether a byte is white space for JSON: a space, a tab, a line feed or a carriage return.
pub(crate) fn is_white_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

/// The reader of one text, at one place in it.
struct Parser<'a> {
    text: &'a str,
    position: usize,
    dialect: Dialect,
}

/// An array or object whose opening brace or bracket has been read, and not yet its closing one.
struct OpenContainer {
    /// The byte that closes it: a brace or a bracket.
    close: u8,
    /// The offset of its opening byte.
    start: usize,
    /// The entries read so far.
    entries: Vec<Entry>,
    /// Where the entry being read starts.
    entry_start: usize,
    /// The name of the member being read, when this is an object.
    name: Option<String>,
}

impl OpenContainer {
    /// The value it is, now that it closes just before `end`.
    fn into_node(self, end: usize) -> Node {
        let kind = if self.close == b'}' {
            NodeKind::Object(self.entries)
        } else {
            NodeKind::Array(self.entries)
        };
        Node {
            kind,
            start: self.start,
            end,
        }
    }
}

/// An array or object whose opening has been read.
enum Opened {
    /// It was empty, and it is read whole.
    Empty(Node),
    /// It has a first entry, whose value is next.
    Open(OpenContainer),
}

/// What follows an item or member.
enum Separator {
    /// A comma at an offset, and more entries after it.
    Comma(usize),
    /// The end of the array or object.
    Close,
    /// A comma at an offset, then the end of the array or object. The comma is kept on the last entry, so that an
    /// edit can tell that the file writes trailing commas.
    CommaAndClose(usize),
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.position).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<u8> {
        self.text.as_bytes().get(self.position + ahead).copied()
    }

    fn error(&self, problem: Problem) -> Error {
        self.error_at(self.position, problem)
    }

    fn error_at(&self, offset: usize, problem: Problem) -> Error {
        Error::Syntax {
            position: Position::of(self.text, offset),
            problem,
        }
    }

    /// The error for something other than what was expected: the end of the text, or the expected problem.
    fn unexpected(&self, problem: Problem) -> Error {
        if self.peek().is_none() {
            self.error(Problem::UnexpectedEnd)
        } else {
            self.error(problem)
        }
    }

    /// Moves past white space and comments.
    fn skip_trivia(&mut self) -> Result<(), Error> {
        loop {
            while self.peek().is_some_and(is_white_space) {
                self.position += 1;
            }
            match (self.peek(), self.peek_at(1)) {
                (Some(b'/'), Some(b'/' | b'*')) if self.dialect == Dialect::Strict => {
                    return Err(self.error(Problem::Comment));
                }
                (Some(b'/'), Some(b'/')) => self.skip_line_comment(),
                (Some(b'/'), Some(b'*')) => self.skip_block_comment()?,
                _ => return Ok(()),
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while self.peek().is_some_and(|byte| byte != b'\n' && byte != b'\r') {
            self.position += 1;
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), Error> {
        let start = self.position;
        let rest = self.text.get(start + 2..).unwrap_or_default();
        match rest.find("*/") {
            Some(length) => {
                self.position = start + 2 + length + 2;
                Ok(())
            }
            None => Err(self.error_at(start, Problem::UnterminatedComment)),
        }
    }

    /// Reads the value that starts here, white space before it already skipped. Arrays and objects are read with a
    /// stack of their own rather than by recursion, so that the depth of a text never depends on the stack of the
    /// thread reading it.
    fn parse_value(&mut self) -> Result<Node, Error> {
        let mut open: Vec<OpenContainer> = Vec::new();
        loop {
            let node = match self.peek() {
                Some(byte @ (b'{' | b'[')) => match self.open_container(byte, open.len())? {
                    Opened::Empty(node) => node,
                    Opened::Open(container) => {
                        open.push(container);
                        continue;
                    }
                },
                _ => self.parse_scalar()?,
            };
            if let Some(root) = self.finish_value(&mut open, node)? {
                return Ok(root);
            }
        }
    }

    /// Reads a value that is neither an array nor an object.
    fn parse_scalar(&mut self) -> Result<Node, Error> {
        let start = self.position;
        let value = match self.peek() {
            Some(b'"') => Value::String(self.parse_string()?),
            Some(b'-' | b'0'..=b'9') => Value::Number(self.parse_number()?),
            Some(b't') => self.parse_literal("true", Value::Bool(true))?,
            Some(b'f') => self.parse_literal("false", Value::Bool(false))?,
            Some(b'n') => self.parse_literal("null", Value::Null)?,
            _ => return Err(self.unexpected(Problem::ExpectedValue)),
        };
        Ok(Node {
            kind: NodeKind::Scalar(value),
            start,
            end: self.position,
        })
    }

    fn parse_literal(&mut self, word: &str, value: Value) -> Result<Value, Error> {
        if self
            .text
            .get(self.position..)
            .is_some_and(|rest| rest.starts_with(word))
        {
            self.position += word.len();
            Ok(value)
        } else {
            Err(self.error(Problem::ExpectedValue))
        }
    }

    /// Reads the opening brace or bracket here, inside `depth` arrays and objects, and what follows it up to its
    /// first value, or to its end when it is empty.
    fn open_container(&mut self, opening: u8, depth: usize) -> Result<Opened, Error> {
        if depth >= NESTING_LIMIT {
            return Err(self.error(Problem::TooDeep { limit: NESTING_LIMIT }));
        }
        let start = self.position;
        let close = if opening == b'{' { b'}' } else { b']' };
        self.position += 1;
        self.skip_trivia()?;
        let mut container = OpenContainer {
            close,
            start,
            entries: Vec::new(),
            entry_start: start,
            name: None,
        };
        if self.peek() == Some(close) {
            self.position += 1;
            return Ok(Opened::Empty(container.into_node(self.position)));
        }
        self.begin_entry(&mut container)?;
        Ok(Opened::Open(container))
    }

    /// Reads where an entry starts, and its name and colon when it is the member of an object.
    fn begin_entry(&mut self, container: &mut OpenContainer) -> Result<(), Error> {
        container.entry_start = self.position;
        if container.close == b'}' {
            container.name = Some(self.parse_member_name()?);
        }
        Ok(())
    }

    /// Gives a value just read to the containers waiting for it, closing those it completes. Gives back the value of
    /// the whole text when the last one closes, or nothing when a container goes on with another entry.
    fn finish_value(&mut self, open: &mut Vec<OpenContainer>, value: Node) -> Result<Option<Node>, Error> {
        let mut node = value;
        loop {
            let Some(mut container) = open.pop() else {
                return Ok(Some(node));
            };
            self.skip_trivia()?;
            let separator = self.parse_separator(container.close)?;
            let comma = match separator {
                Separator::Comma(offset) | Separator::CommaAndClose(offset) => Some(offset),
                Separator::Close => None,
            };
            let name = container.name.take();
            container.entries.push(Entry {
                name,
                start: container.entry_start,
                value: node,
                comma,
            });
            if matches!(separator, Separator::Comma(_)) {
                self.begin_entry(&mut container)?;
                open.push(container);
                return Ok(None);
            }
            node = container.into_node(self.position);
        }
    }

    fn parse_member_name(&mut self) -> Result<String, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.unexpected(Problem::ExpectedMemberName));
        }
        let name = self.parse_string()?;
        self.skip_trivia()?;
        if self.peek() != Some(b':') {
            return Err(self.unexpected(Problem::ExpectedColon));
        }
        self.position += 1;
        self.skip_trivia()?;
        Ok(name)
    }

    /// Reads what follows an entry: a comma, with a closing byte after it when trailing commas are allowed, or the
    /// closing byte.
    fn parse_separator(&mut self, close: u8) -> Result<Separator, Error> {
        if self.peek() == Some(close) {
            self.position += 1;
            return Ok(Separator::Close);
        }
        if self.peek() != Some(b',') {
            let problem = if close == b'}' {
                Problem::ExpectedCommaOrClosingBrace
            } else {
                Problem::ExpectedCommaOrClosingBracket
            };
            return Err(self.unexpected(problem));
        }
        let comma = self.position;
        self.position += 1;
        self.skip_trivia()?;
        if self.peek() != Some(close) {
            return Ok(Separator::Comma(comma));
        }
        if self.dialect == Dialect::Strict {
            return Err(self.error_at(comma, Problem::TrailingComma));
        }
        self.position += 1;
        Ok(Separator::CommaAndClose(comma))
    }
}

#[cfg(test)]
mod tests;
