//! The lexers of markup: HTML, Vue, Svelte and XML, whose tags, attributes, values, entities and comments are cut out
//! of the text between them; and Markdown, whose headings, quotes, lists, code and links are.

use super::lexer::{is_name_byte, is_name_start, push};
use super::{Inside, Token, TokenKind};

/// What opens a comment of markup.
const COMMENT_OPEN: &str = "<!--";
/// What closes it.
const COMMENT_CLOSE: &str = "-->";

/// Cuts one line of HTML or XML into its tokens, starting inside the tag, the value or the comment the line before
/// left open.
pub(crate) fn markup_line(line: &str, start: Inside, tokens: &mut Vec<Token>) -> Inside {
    let mut cutter = Cutter { line, at: 0, tokens };
    let mut inside = match start {
        Inside::Tag | Inside::Value(_) | Inside::Comment(_) => start,
        Inside::Code | Inside::Quote(_) | Inside::Fence => Inside::Code,
    };
    while cutter.at < line.len() {
        inside = match inside {
            Inside::Comment(_) => cutter.comment(),
            Inside::Value(quote) => cutter.value(quote),
            Inside::Tag => cutter.tag(),
            Inside::Code | Inside::Quote(_) | Inside::Fence => cutter.text(),
        };
    }
    inside
}

/// The cutter of one line of markup.
struct Cutter<'a> {
    /// The line.
    line: &'a str,
    /// Where the next token starts.
    at: usize,
    /// The tokens so far.
    tokens: &'a mut Vec<Token>,
}

impl Cutter<'_> {
    /// The byte at a place.
    fn byte(&self, at: usize) -> Option<u8> {
        self.line.as_bytes().get(at).copied()
    }

    /// Adds a token and goes past it.
    fn take(&mut self, end: usize, kind: TokenKind) {
        push(self.tokens, self.at..end, kind);
        self.at = end;
    }

    /// The end of what goes on from a place while its bytes are so.
    fn end_while(&self, from: usize, goes_on: impl Fn(u8) -> bool) -> usize {
        let mut end = from;
        while self.byte(end).is_some_and(&goes_on) {
            end += 1;
        }
        end
    }

    /// Goes on in a comment, to its end or the end of the line.
    fn comment(&mut self) -> Inside {
        if let Some(offset) = self.line.get(self.at..).and_then(|rest| rest.find(COMMENT_CLOSE)) {
            self.take(self.at + offset + COMMENT_CLOSE.len(), TokenKind::Comment);
            Inside::Code
        } else {
            self.take(self.line.len(), TokenKind::Comment);
            Inside::Comment(0)
        }
    }

    /// Goes on in the value of an attribute, to its closing quote or the end of the line.
    fn value(&mut self, quote: u8) -> Inside {
        let rest = self.line.as_bytes().get(self.at..).unwrap_or_default();
        if let Some(offset) = rest.iter().position(|byte| *byte == quote) {
            self.take(self.at + offset + 1, TokenKind::String);
            Inside::Tag
        } else {
            self.take(self.line.len(), TokenKind::String);
            Inside::Value(quote)
        }
    }

    /// Inside a tag: its attributes, their values, and the `>` that ends it.
    fn tag(&mut self) -> Inside {
        let Some(byte) = self.byte(self.at) else {
            return Inside::Tag;
        };
        match byte {
            b'>' => {
                self.take(self.at + 1, TokenKind::Punctuation);
                Inside::Code
            }
            b'/' | b'?' if self.byte(self.at + 1) == Some(b'>') => {
                self.take(self.at + 2, TokenKind::Punctuation);
                Inside::Code
            }
            b'"' | b'\'' => {
                self.take(self.at + 1, TokenKind::String);
                self.value(byte)
            }
            b'=' => {
                self.take(self.at + 1, TokenKind::Operator);
                Inside::Tag
            }
            _ if is_attribute_byte(byte) => {
                let end = self.end_while(self.at, is_attribute_byte);
                self.take(end, TokenKind::Attribute);
                Inside::Tag
            }
            _ => {
                self.at += 1;
                Inside::Tag
            }
        }
    }

    /// The text between tags, up to the next tag, comment or entity.
    fn text(&mut self) -> Inside {
        let rest = self.line.as_bytes().get(self.at..).unwrap_or_default();
        let Some(offset) = rest.iter().position(|byte| *byte == b'<' || *byte == b'&') else {
            self.at = self.line.len();
            return Inside::Code;
        };
        self.at += offset;
        if self
            .line
            .get(self.at..)
            .is_some_and(|rest| rest.starts_with(COMMENT_OPEN))
        {
            self.take(self.at + COMMENT_OPEN.len(), TokenKind::Comment);
            return self.comment();
        }
        if self.byte(self.at) == Some(b'&') {
            let end = self.end_while(self.at + 1, |byte| is_name_byte(byte) || byte == b'#');
            if self.byte(end) == Some(b';') && end > self.at + 1 {
                self.take(end + 1, TokenKind::Constant);
            } else {
                self.at += 1;
            }
            return Inside::Code;
        }
        let mut name = self.at + 1;
        if self.byte(name).is_some_and(|byte| b"/!?".contains(&byte)) {
            name += 1;
        }
        if !self.byte(name).is_some_and(is_name_start) {
            self.at += 1;
            return Inside::Code;
        }
        self.take(name, TokenKind::Punctuation);
        let end = self.end_while(name, |byte| is_name_byte(byte) || b"-:.".contains(&byte));
        self.take(end, TokenKind::Tag);
        Inside::Tag
    }
}

/// Whether a byte goes in the name of an attribute: the names of Vue and Svelte too, `v-on:click`, `@click`, `:value`.
fn is_attribute_byte(byte: u8) -> bool {
    is_name_byte(byte) || b"-:.@#".contains(&byte)
}

/// Cuts one line of Markdown into its tokens: a fence opens or closes a block of code, whose lines are strings; a
/// heading is a tag, a quote a comment, the mark of an item of a list punctuation; in the text, code between backticks
/// is a string, and a link has its text as an attribute and its address as a string.
pub(crate) fn markdown_line(line: &str, start: Inside, tokens: &mut Vec<Token>) -> Inside {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = line.get(indent..).unwrap_or_default();
    if indent <= 3 && (rest.starts_with("```") || rest.starts_with("~~~")) {
        push(tokens, 0..line.len(), TokenKind::Punctuation);
        return if start == Inside::Fence {
            Inside::Code
        } else {
            Inside::Fence
        };
    }
    if start == Inside::Fence {
        push(tokens, 0..line.len(), TokenKind::String);
        return Inside::Fence;
    }
    let hashes = rest.len() - rest.trim_start_matches('#').len();
    if (1..=6).contains(&hashes)
        && rest
            .get(hashes..)
            .is_none_or(|after| after.is_empty() || after.starts_with(' '))
    {
        push(tokens, indent..line.len(), TokenKind::Tag);
        return Inside::Code;
    }
    if rest.starts_with('>') {
        push(tokens, indent..line.len(), TokenKind::Comment);
        return Inside::Code;
    }
    let digits = rest.len()
        - rest
            .trim_start_matches(|character: char| character.is_ascii_digit())
            .len();
    let marker = match rest.get(digits..) {
        Some(after) if digits == 0 && ["- ", "* ", "+ "].iter().any(|mark| after.starts_with(mark)) => 1,
        Some(after) if digits > 0 && (after.starts_with(". ") || after.starts_with(") ")) => digits + 1,
        _ => 0,
    };
    push(tokens, indent..indent + marker, TokenKind::Punctuation);
    inline(line, indent + marker, tokens);
    Inside::Code
}

/// The code and the links in the text of a line of Markdown.
fn inline(line: &str, from: usize, tokens: &mut Vec<Token>) {
    let bytes = line.as_bytes();
    let find = |byte: u8, after: usize| {
        bytes
            .get(after..)
            .and_then(|rest| rest.iter().position(|found| *found == byte))
            .map(|offset| after + offset)
    };
    let mut at = from;
    while at < bytes.len() {
        match bytes.get(at) {
            Some(b'`') => {
                let end = find(b'`', at + 1).map_or(bytes.len(), |close| close + 1);
                push(tokens, at..end, TokenKind::String);
                at = end;
            }
            Some(b'[') => {
                let Some(close) = find(b']', at + 1) else {
                    at += 1;
                    continue;
                };
                push(tokens, at..close + 1, TokenKind::Attribute);
                at = close + 1;
                if bytes.get(at) == Some(&b'(')
                    && let Some(end) = find(b')', at)
                {
                    push(tokens, at..end + 1, TokenKind::String);
                    at = end + 1;
                }
            }
            _ => at += 1,
        }
    }
}
