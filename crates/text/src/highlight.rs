//! The colors of the code (`file-view.highlight`): the text of a file cut into tokens of its language, each of a kind
//! the theme gives a color, for the File, Diff and Merge pages.
//!
//! The text is cut line by line, with what is still open at the end of a line (a block comment, a string over several
//! lines, a tag of markup) carried to the next, so that a line inside a block comment is colored as one. The languages
//! are those of the card, told by the name of the file; each is a table of its comments, strings, keywords, types and
//! constants given to one lexer of code, or one of the two of markup (HTML and XML, and Markdown). They give the kinds
//! the published grammars of these languages give the same text, coarser: a name is a function when it is called, a
//! property after a dot or before a colon, a type when it starts with a capital letter.

mod grammar;
mod grammars;
mod lexer;
mod markup;

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;

use std::ops::Range;

pub use grammars::Language;

/// What a token is, which the theme gives a color and a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A comment.
    Comment,
    /// A string, or a character.
    String,
    /// A number.
    Number,
    /// A keyword of the language: `fn`, `if`, `return`.
    Keyword,
    /// An operator: `+`, `==`, `->`.
    Operator,
    /// Punctuation: brackets, commas, semicolons, the dot.
    Punctuation,
    /// The name of a type.
    Type,
    /// The name of a function, where it is called or defined.
    Function,
    /// The name of a variable.
    Variable,
    /// The name of a property, a field or a key.
    Property,
    /// A constant: `true`, `null`, a name all in capitals.
    Constant,
    /// The name of a tag of markup, or a heading of Markdown.
    Tag,
    /// The name of an attribute of markup, an attribute or a decorator of code.
    Attribute,
    /// Anything else: drawn in the color of text.
    Plain,
}

/// A token of a line: where it is in the line, in bytes, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Where it is in its line, in bytes, on boundaries of characters.
    pub range: Range<usize>,
    /// What it is.
    pub kind: TokenKind,
}

/// What is still open at the end of a line, which the next line starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LineState(pub(crate) Inside);

/// What a line starts inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Inside {
    /// Code, or the text of markup.
    #[default]
    Code,
    /// A block comment, by its place among those of the language.
    Comment(usize),
    /// A string, by its place among those of the language.
    Quote(usize),
    /// A tag of markup, between its name and its `>`.
    Tag,
    /// A value of an attribute of markup, in these quotes.
    Value(u8),
    /// A block of code of Markdown, between its fences.
    Fence,
}

/// The tokens of a line, in order, without the plain text between them, and what is open at its end.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineTokens {
    /// The tokens, in order, none overlapping another.
    pub tokens: Vec<Token>,
    /// What is still open at its end.
    pub end: LineState,
}

/// A text to color, and the name of its file, which tells its language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightInput<'a> {
    /// The path or the name of the file: only its name is looked at.
    pub path: &'a str,
    /// Its text.
    pub text: &'a str,
}

/// The tokens of each line of a text, colored as the language its name tells; a file of no language of the card has
/// no tokens. A line is what is between two `\n`, without the `\r` of a `\r\n`; the text has one line more than it has
/// `\n`.
///
/// Feature: file-view.highlight
#[must_use]
pub fn highlight(input: &HighlightInput<'_>) -> Vec<LineTokens> {
    let language = Language::of(input.path);
    let mut state = LineState::default();
    input
        .text
        .split('\n')
        .map(|line| {
            let tokens = highlight_line(language, line, state);
            state = tokens.end;
            tokens
        })
        .collect()
}

/// The tokens of one line of a language, starting in what the line before left open; a line of no language has none.
#[must_use]
pub fn highlight_line(language: Option<Language>, line: &str, start: LineState) -> LineTokens {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let Some(language) = language else {
        return LineTokens::default();
    };
    let mut tokens = Vec::new();
    let end = match language.grammar() {
        grammars::Lexer::Code(grammar) => lexer::code_line(grammar, line, start.0, &mut tokens),
        grammars::Lexer::Markup => markup::markup_line(line, start.0, &mut tokens),
        grammars::Lexer::Markdown => markup::markdown_line(line, start.0, &mut tokens),
    };
    LineTokens {
        tokens,
        end: LineState(end),
    }
}

/// Some lines of a text replaced by others, as an edit does: the lines from `start`, `removed` of them before, `added`
/// in their place after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinesReplaced {
    /// The first line changed.
    pub start: usize,
    /// How many lines it replaced.
    pub removed: usize,
    /// How many lines are in their place.
    pub added: usize,
}

/// Colors a text again after some of its lines were replaced, from the first line changed and only as far as what is
/// open at the end of a line comes back to what it was: typing in a line colors that line again, opening a block
/// comment colors the lines after it. Gives the lines colored again, by their places in the text as it is now.
pub fn highlight_again(
    (language, line_of): (Option<Language>, &dyn Fn(usize) -> String),
    tokens: &mut Vec<LineTokens>,
    replaced: LinesReplaced,
) -> Range<usize> {
    let removed_end = (replaced.start + replaced.removed).min(tokens.len());
    let start = replaced.start.min(removed_end);
    let mut old_start = removed_end
        .checked_sub(1)
        .and_then(|before| tokens.get(before))
        .map(|line| line.end)
        .unwrap_or_default();
    let after: Vec<LineTokens> = tokens.split_off(removed_end);
    tokens.truncate(start);
    let mut state = tokens.last().map(|line| line.end).unwrap_or_default();
    for line in start..start + replaced.added {
        let colored = highlight_line(language, &line_of(line), state);
        state = colored.end;
        tokens.push(colored);
    }
    let mut after = after.into_iter();
    let mut line = start + replaced.added;
    while let Some(old) = after.next() {
        if state == old_start {
            tokens.push(old);
            tokens.extend(after);
            return start..line;
        }
        old_start = old.end;
        let colored = highlight_line(language, &line_of(line), state);
        state = colored.end;
        tokens.push(colored);
        line += 1;
    }
    start..line
}
