//! What can go wrong when reading or editing JSON, said in words.

use std::fmt;

use crate::value::ValueKind;

/// A place in a text, as an editor shows it: both numbers start at 1, and a column counts characters, not bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// The line, from 1.
    pub line: usize,
    /// The character in the line, from 1.
    pub column: usize,
}

impl Position {
    /// The line and column of a byte offset in a text. An offset past the end, or inside a character, counts as far
    /// as the whole characters before it.
    pub(crate) fn of(text: &str, offset: usize) -> Position {
        let mut line = 1;
        let mut column = 1;
        let mut characters = text.char_indices().peekable();
        while let Some((index, character)) = characters.next() {
            if index >= offset {
                break;
            }
            // A line ends with a line feed, a carriage return and a line feed, or a carriage return alone.
            let next = characters.peek().map(|(_, next)| *next);
            let ends_line = character == '\n' || (character == '\r' && next != Some('\n'));
            if ends_line {
                line += 1;
                column = 1;
            } else if character != '\r' {
                column += 1;
            }
        }
        Position { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}, column {}", self.line, self.column)
    }
}

/// What is wrong in a text that is not valid JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The text ends before the value is complete.
    UnexpectedEnd,
    /// Something that is not a value stands where a value should be.
    ExpectedValue,
    /// A member of an object does not start with its name in double quotes.
    ExpectedMemberName,
    /// The name of a member is not followed by a colon.
    ExpectedColon,
    /// A member of an object is followed by something other than a comma or the closing brace.
    ExpectedCommaOrClosingBrace,
    /// An item of an array is followed by something other than a comma or the closing bracket.
    ExpectedCommaOrClosingBracket,
    /// A comma comes right before a closing brace or bracket, which only JSON with comments allows.
    TrailingComma,
    /// A comment, which only JSON with comments allows.
    Comment,
    /// A comment that starts with `/*` and never ends.
    UnterminatedComment,
    /// A string that never ends with its closing quote.
    UnterminatedString,
    /// A control character, such as a line break or a tab, written as itself inside a string.
    ControlCharacterInString,
    /// A backslash in a string followed by a character that is not an escape.
    InvalidEscape,
    /// A `\u` escape not followed by four hexadecimal digits.
    InvalidUnicodeEscape,
    /// Half of a surrogate pair written without the other half.
    LoneSurrogate,
    /// A number that starts with a zero followed by more digits.
    LeadingZero,
    /// A number that is missing digits, after its minus sign, its decimal point or its exponent.
    InvalidNumber,
    /// Arrays and objects nested deeper than the limit.
    TooDeep {
        /// How many levels are allowed.
        limit: usize,
    },
    /// More text after the value, where only white space or comments may follow.
    TextAfterValue,
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words = match self {
            Problem::UnexpectedEnd => "the text ends before the value is complete",
            Problem::ExpectedValue => "a value was expected here",
            Problem::ExpectedMemberName => "a member name in double quotes was expected",
            Problem::ExpectedColon => "a colon was expected after the member name",
            Problem::ExpectedCommaOrClosingBrace => "a comma or a closing brace was expected",
            Problem::ExpectedCommaOrClosingBracket => "a comma or a closing bracket was expected",
            Problem::TrailingComma => "a comma cannot come right before a closing brace or bracket",
            Problem::Comment => "comments are not allowed in strict JSON",
            Problem::UnterminatedComment => "the comment is never closed with */",
            Problem::UnterminatedString => "the string is never closed with a double quote",
            Problem::ControlCharacterInString => "a control character must be escaped inside a string",
            Problem::InvalidEscape => "this is not a valid escape after a backslash",
            Problem::InvalidUnicodeEscape => "\\u must be followed by four hexadecimal digits",
            Problem::LoneSurrogate => "half of a surrogate pair is written without the other half",
            Problem::LeadingZero => "a number cannot start with a zero followed by more digits",
            Problem::InvalidNumber => "the number is missing digits",
            Problem::TooDeep { limit } => {
                return write!(formatter, "arrays and objects are nested more than {limit} levels deep");
            }
            Problem::TextAfterValue => "there is more text after the value",
        };
        formatter.write_str(words)
    }
}

/// One step of a path into a JSON document: the member of an object with that name, or the item of an array at that
/// index, from 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// The member of an object with this name.
    Member(String),
    /// The item of an array at this index, from 0.
    Index(usize),
}

/// Why a text could not be read as JSON, or a value could not be set or removed in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The text is not valid JSON, or not valid JSON with comments.
    Syntax {
        /// Where the problem is.
        position: Position,
        /// What the problem is.
        problem: Problem,
    },
    /// The path goes through a value that is not the object or array its next key needs.
    NotAContainer {
        /// The path to the value in the way, empty for the whole document.
        path: Vec<Key>,
        /// What that value is.
        found: ValueKind,
        /// The key that could not be followed into it.
        key: Key,
    },
    /// The path names an item past the end of an array, beyond the one place where a new item can be added.
    IndexOutOfRange {
        /// The path to the array, empty for the whole document.
        path: Vec<Key>,
        /// The index asked for.
        index: usize,
        /// How many items the array has.
        length: usize,
    },
    /// Removing was asked for the whole document, which only a value inside it can be.
    EmptyPath,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax { position, problem } => write!(formatter, "At {position}, {problem}."),
            Error::NotAContainer { path, found, key } => {
                let (needed, item) = match key {
                    Key::Member(name) => ("an object", format!("a member named \"{name}\"")),
                    Key::Index(index) => ("an array", format!("an item at index {index}")),
                };
                let place = describe_place(path);
                write!(formatter, "{place} is {found}, not {needed}, so it cannot have {item}.")
            }
            Error::IndexOutOfRange { path, index, length } => {
                let place = describe_place(path);
                write!(
                    formatter,
                    "{place} has {length} items, so there is no item at index {index}."
                )
            }
            Error::EmptyPath => formatter.write_str("The whole document cannot be removed, only a value inside it."),
        }
    }
}

impl std::error::Error for Error {}

/// The value a path leads to, in words that can start a sentence.
fn describe_place(path: &[Key]) -> String {
    if path.is_empty() {
        return "The document".to_string();
    }
    let mut written = String::new();
    for key in path {
        match key {
            Key::Member(name) if written.is_empty() => written.push_str(name),
            Key::Member(name) => {
                written.push('.');
                written.push_str(name);
            }
            Key::Index(index) => {
                written.push('[');
                written.push_str(&index.to_string());
                written.push(']');
            }
        }
    }
    format!("The value at {written}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_lines_and_characters_from_one() {
        let cases: [(&str, usize, usize, usize); 7] = [
            ("abc", 0, 1, 1),
            ("abc", 2, 1, 3),
            ("a\nbc", 3, 2, 2),
            ("a\r\nbc", 4, 2, 2),
            ("a\rbc", 3, 2, 2),
            ("é\nxé!", 6, 2, 3),
            ("ab\r", 3, 2, 1),
        ];
        for (text, offset, line, column) in cases {
            assert_eq!(
                Position::of(text, offset),
                Position { line, column },
                "{text:?} at {offset}"
            );
        }
    }

    #[test]
    fn errors_say_where_and_what_in_words() {
        let cases = [
            (
                Error::Syntax {
                    position: Position { line: 3, column: 7 },
                    problem: Problem::ExpectedColon,
                },
                "At line 3, column 7, a colon was expected after the member name.",
            ),
            (
                Error::NotAContainer {
                    path: vec![Key::Member("editor".into()), Key::Index(2)],
                    found: ValueKind::String,
                    key: Key::Member("size".into()),
                },
                "The value at editor[2] is a string, not an object, so it cannot have a member named \"size\".",
            ),
            (
                Error::IndexOutOfRange {
                    path: vec![],
                    index: 4,
                    length: 2,
                },
                "The document has 2 items, so there is no item at index 4.",
            ),
            (
                Error::Syntax {
                    position: Position { line: 1, column: 1 },
                    problem: Problem::TooDeep { limit: 512 },
                },
                "At line 1, column 1, arrays and objects are nested more than 512 levels deep.",
            ),
        ];
        for (error, words) in cases {
            assert_eq!(error.to_string(), words);
        }
    }
}
