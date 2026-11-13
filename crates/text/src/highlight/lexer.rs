//! The lexer of code: one line at a time, with the block comment or the string still open at its end carried to the
//! next, it cuts comments, strings, numbers, names, operators and punctuation, and tells what each name is from the
//! grammar of its language and from what is around it.

use super::grammar::{Grammar, is_among};
use super::{Inside, Token, TokenKind};

/// The characters of operators.
const OPERATORS: &[u8] = b"+-*/%=<>!&|^~?:";

/// Cuts one line of code into its tokens, starting inside what the line before left open, and says what is open at
/// its end. A string that does not go over lines ends with its line when it is not closed.
pub(crate) fn code_line(grammar: &Grammar, line: &str, start: Inside, tokens: &mut Vec<Token>) -> Inside {
    let mut lexer = Lexer {
        grammar,
        line,
        at: 0,
        tokens,
    };
    let mut inside = match start {
        Inside::Comment(_) | Inside::Quote(_) => start,
        Inside::Code | Inside::Tag | Inside::Value(_) | Inside::Fence => Inside::Code,
    };
    if grammar.has_sections && inside == Inside::Code {
        lexer.section();
    }
    while lexer.at < line.len() {
        inside = match inside {
            Inside::Comment(index) => lexer.in_comment(index),
            Inside::Quote(index) => lexer.in_quote(index),
            Inside::Code | Inside::Tag | Inside::Value(_) | Inside::Fence => lexer.code(),
        };
    }
    match inside {
        Inside::Quote(index) if grammar.quotes.get(index).is_none_or(|quote| !quote.spans_lines) => Inside::Code,
        other => other,
    }
}

/// Adds a token after the others, or makes the last one longer when it is of the same kind and ends where this starts.
pub(crate) fn push(tokens: &mut Vec<Token>, range: std::ops::Range<usize>, kind: TokenKind) {
    if range.is_empty() || kind == TokenKind::Plain {
        return;
    }
    if let Some(last) = tokens.last_mut()
        && last.kind == kind
        && last.range.end == range.start
    {
        last.range.end = range.end;
        return;
    }
    tokens.push(Token { range, kind });
}

/// Whether a byte starts a name: a letter, `_`, or a byte of a character that is not ASCII.
pub(crate) fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

/// Whether a byte goes on a name.
pub(crate) fn is_name_byte(byte: u8) -> bool {
    is_name_start(byte) || byte.is_ascii_digit()
}

/// The lexer of one line of code.
struct Lexer<'a> {
    /// The grammar of the language.
    grammar: &'a Grammar,
    /// The line.
    line: &'a str,
    /// Where the next token starts, in bytes.
    at: usize,
    /// The tokens so far.
    tokens: &'a mut Vec<Token>,
}

impl Lexer<'_> {
    /// The byte at a place, none past the end.
    fn byte(&self, at: usize) -> Option<u8> {
        self.line.as_bytes().get(at).copied()
    }

    /// Whether the line has a text at a place.
    fn has_at(&self, at: usize, text: &str) -> bool {
        self.line.get(at..).is_some_and(|rest| rest.starts_with(text))
    }

    /// The first byte at or after a place that is not a space.
    fn next_visible(&self, from: usize) -> Option<u8> {
        self.line
            .as_bytes()
            .get(from..)?
            .iter()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
    }

    /// The last byte before a place that is not a space.
    fn previous_visible(&self, before: usize) -> Option<u8> {
        self.line
            .as_bytes()
            .get(..before)?
            .iter()
            .rev()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
    }

    /// Adds a token.
    fn push(&mut self, range: std::ops::Range<usize>, kind: TokenKind) {
        push(self.tokens, range, kind);
    }

    /// The header of a section, `[server]`, at the start of the line.
    fn section(&mut self) {
        let start = self.line.len() - self.line.trim_start().len();
        if self.byte(start) != Some(b'[') {
            return;
        }
        let end = self
            .line
            .get(start..)
            .and_then(|rest| rest.find(']'))
            .map_or(self.line.len(), |close| start + close + 1);
        self.push(start..end, TokenKind::Type);
        self.at = end;
    }

    /// Goes on in a block comment, to its end or to the end of the line.
    fn in_comment(&mut self, index: usize) -> Inside {
        let close = self.grammar.block_comments.get(index).map_or("", |(_, close)| *close);
        let found = self.line.get(self.at..).and_then(|rest| rest.find(close));
        if let Some(offset) = found.filter(|_| !close.is_empty()) {
            let end = self.at + offset + close.len();
            self.push(self.at..end, TokenKind::Comment);
            self.at = end;
            Inside::Code
        } else {
            self.push(self.at..self.line.len(), TokenKind::Comment);
            self.at = self.line.len();
            Inside::Comment(index)
        }
    }

    /// Goes on in a string, to its end or to the end of the line.
    fn in_quote(&mut self, index: usize) -> Inside {
        let Some(quote) = self.grammar.quotes.get(index) else {
            self.at = self.line.len();
            return Inside::Code;
        };
        let start = self.at;
        let mut at = start;
        let bytes = self.line.as_bytes();
        while at < bytes.len() {
            if quote.escape.is_some() && self.byte(at) == quote.escape {
                at = (at + 1 + self.char_length(at + 1)).min(bytes.len());
                continue;
            }
            if self.has_at(at, quote.close) {
                let end = at + quote.close.len();
                self.push(start..end, TokenKind::String);
                self.at = end;
                self.key_before_colon();
                return Inside::Code;
            }
            at += self.char_length(at);
        }
        self.push(start..bytes.len(), TokenKind::String);
        self.at = bytes.len();
        Inside::Quote(index)
    }

    /// The length in bytes of the character at a place, 1 past the end.
    fn char_length(&self, at: usize) -> usize {
        self.line
            .get(at..)
            .and_then(|rest| rest.chars().next())
            .map_or(1, char::len_utf8)
    }

    /// A string just closed before a colon is a key, when the language has keys before colons.
    fn key_before_colon(&mut self) {
        if self.grammar.keys_before_colons
            && self.next_visible(self.at) == Some(b':')
            && let Some(last) = self.tokens.last_mut()
        {
            last.kind = TokenKind::Property;
        }
    }

    /// The token at the place in code, and what it opens.
    fn code(&mut self) -> Inside {
        let start = self.at;
        let Some(byte) = self.byte(start) else {
            return Inside::Code;
        };
        if byte.is_ascii_whitespace() {
            self.at += 1;
            return Inside::Code;
        }
        if self.lifetime() {
            return Inside::Code;
        }
        if let Some(opened) = self.comment_or_quote() {
            return opened;
        }
        if self.number() || self.mark() {
            return Inside::Code;
        }
        if is_name_start(byte) {
            return self.name();
        }
        let kind = if OPERATORS.contains(&byte) {
            TokenKind::Operator
        } else if byte.is_ascii_punctuation() && !b"\\#`$'\"".contains(&byte) {
            TokenKind::Punctuation
        } else {
            TokenKind::Plain
        };
        self.push(start..start + 1, kind);
        self.at = start + 1;
        Inside::Code
    }

    /// A comment or a string that starts at the place: a block comment and a string go on until they close.
    fn comment_or_quote(&mut self) -> Option<Inside> {
        let start = self.at;
        let grammar = self.grammar;
        if let Some(index) = grammar
            .block_comments
            .iter()
            .position(|(open, _)| self.has_at(start, open))
        {
            let open = grammar.block_comments.get(index).map_or(0, |(open, _)| open.len());
            self.push(start..start + open, TokenKind::Comment);
            self.at = start + open;
            return Some(self.in_comment(index));
        }
        if grammar.line_comments.iter().any(|open| self.has_at(start, open)) {
            self.push(start..self.line.len(), TokenKind::Comment);
            self.at = self.line.len();
            return Some(Inside::Code);
        }
        self.quote_at(start)
    }

    /// A string that opens at a place.
    fn quote_at(&mut self, start: usize) -> Option<Inside> {
        let index = self
            .grammar
            .quotes
            .iter()
            .position(|quote| self.has_at(start, quote.open))?;
        let open = self.grammar.quotes.get(index).map_or(0, |quote| quote.open.len());
        self.push(start..start + open, TokenKind::String);
        self.at = start + open;
        Some(self.in_quote(index))
    }

    /// A number at the place: digits, or a dot before digits, with the letters, digits, `_` and dots before digits
    /// after them: `42`, `0x1F`, `1_000`, `2.5e3`, `10u32`.
    fn number(&mut self) -> bool {
        let start = self.at;
        let is_digit = |byte: Option<u8>| byte.is_some_and(|byte| byte.is_ascii_digit());
        let starts = is_digit(self.byte(start))
            || (self.byte(start) == Some(b'.')
                && is_digit(self.byte(start + 1))
                && !self.previous_visible(start).is_some_and(is_name_byte));
        if !starts {
            return false;
        }
        let mut end = start + 1;
        while let Some(byte) = self.byte(end) {
            let goes_on =
                byte.is_ascii_alphanumeric() || byte == b'_' || (byte == b'.' && is_digit(self.byte(end + 1)));
            if !goes_on {
                break;
            }
            end += 1;
        }
        self.push(start..end, TokenKind::Number);
        self.at = end;
        true
    }

    /// A lifetime of Rust, `'a`, which is not a character `'a'`.
    fn lifetime(&mut self) -> bool {
        let start = self.at;
        if !self.grammar.has_lifetimes || self.byte(start) != Some(b'\'') {
            return false;
        }
        if !self.byte(start + 1).is_some_and(is_name_start) {
            return false;
        }
        let after_first = start + 1 + self.char_length(start + 1);
        if self.byte(after_first) == Some(b'\'') {
            return false;
        }
        let end = self.name_end(start + 1);
        self.push(start..end, TokenKind::Type);
        self.at = end;
        true
    }

    /// A name after a mark of the language, `$HOME`, `@Override`, `#[derive(Debug)]`, `${name}`, as the mark's kind,
    /// or as a constant when the language names it so: `$true`.
    fn mark(&mut self) -> bool {
        let start = self.at;
        let Some(&(_, kind)) = self
            .grammar
            .marks
            .iter()
            .find(|(mark, _)| self.byte(start) == Some(*mark))
        else {
            return false;
        };
        let mut next = start + 1;
        if self.byte(next) == Some(b'!') && self.byte(next + 1) == Some(b'[') {
            next += 1;
        }
        let end = match self.byte(next) {
            Some(b'[') => self.closing(next, b']'),
            Some(b'{') => self.closing(next, b'}'),
            Some(byte) if is_name_byte(byte) => self.name_end(next),
            _ => return false,
        };
        let text = self.line.get(start..end).unwrap_or_default();
        let grammar = self.grammar;
        let kind = if is_among(grammar.constants, text, grammar.is_case_blind) {
            TokenKind::Constant
        } else {
            kind
        };
        self.push(start..end, kind);
        self.at = end;
        true
    }

    /// The end of a bracket opened at a place: after the first closing one, or the end of the line.
    fn closing(&self, open: usize, close: u8) -> usize {
        self.line
            .as_bytes()
            .get(open..)
            .and_then(|rest| rest.iter().position(|byte| *byte == close))
            .map_or(self.line.len(), |offset| open + offset + 1)
    }

    /// The end of a name that starts at a place.
    fn name_end(&self, start: usize) -> usize {
        let mut end = start;
        while let Some(byte) = self.byte(end) {
            let is_dash = self.grammar.has_dashes && byte == b'-' && self.byte(end + 1).is_some_and(is_name_byte);
            if !is_name_byte(byte) && !is_dash {
                break;
            }
            end += 1;
        }
        end
    }

    /// A name at the place, of the kind the grammar and what is around it tell; a string when it is a prefix of one.
    fn name(&mut self) -> Inside {
        let start = self.at;
        let end = self.name_end(start);
        let text = self.line.get(start..end).unwrap_or_default();
        self.at = end;
        if self.grammar.string_prefixes.contains(&text) {
            let tokens = self.tokens.len();
            if let Some(opened) = self.quote_at(end) {
                if let Some(token) = self.tokens.get_mut(tokens) {
                    token.range.start = start;
                }
                return opened;
            }
        }
        let kind = self.name_kind(text, (start, end));
        self.push(start..end, kind);
        Inside::Code
    }

    /// What a name is: a keyword, a constant or a type the grammar names, a command, a function called, a key, a
    /// property after a dot, a type or a constant by its capitals, or what the grammar gives the others.
    fn name_kind(&self, text: &str, (start, end): (usize, usize)) -> TokenKind {
        let grammar = self.grammar;
        let after = self.next_visible(end);
        let is_line_start = self.line.get(..start).is_some_and(|before| before.trim().is_empty());
        if is_among(grammar.keywords, text, grammar.is_case_blind) {
            TokenKind::Keyword
        } else if is_among(grammar.constants, text, grammar.is_case_blind) {
            TokenKind::Constant
        } else if is_among(grammar.types, text, grammar.is_case_blind) {
            TokenKind::Type
        } else if self.is_function(text, end) {
            TokenKind::Function
        } else if (grammar.keys_before_colons && after == Some(b':') && !self.has_at_visible(end, "::"))
            || (grammar.keys_before_equals && is_line_start && after == Some(b'='))
            || (self.previous_visible(start) == Some(b'.') && !self.has_before(start, ".."))
        {
            TokenKind::Property
        } else if grammar.capital_types && text.starts_with(|first: char| first.is_uppercase()) {
            if text.len() > 1 && !text.chars().any(char::is_lowercase) {
                TokenKind::Constant
            } else {
                TokenKind::Type
            }
        } else {
            grammar.bare
        }
    }

    /// Whether a name that ends at a place is a function: a command of PowerShell, a name called, a macro of Rust.
    fn is_function(&self, text: &str, end: usize) -> bool {
        let grammar = self.grammar;
        (grammar.has_cmdlets && text.contains('-') && text.starts_with(|first: char| first.is_uppercase()))
            || self.next_visible(end) == Some(b'(')
            || (grammar.has_macros && self.byte(end) == Some(b'!') && self.byte(end + 1) != Some(b'='))
    }

    /// Whether the first text after a place, past the spaces, starts so.
    fn has_at_visible(&self, from: usize, text: &str) -> bool {
        self.line
            .get(from..)
            .is_some_and(|rest| rest.trim_start().starts_with(text))
    }

    /// Whether the text before a place, past the spaces, ends so.
    fn has_before(&self, before: usize, text: &str) -> bool {
        self.line
            .get(..before)
            .is_some_and(|rest| rest.trim_end().ends_with(text))
    }
}
