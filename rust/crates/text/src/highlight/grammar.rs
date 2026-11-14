//! What the lexer of code needs of a language: its comments, its strings, its words and the traits it has.

use super::TokenKind;

/// A kind of string of a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Quote {
    /// What opens it.
    pub(crate) open: &'static str,
    /// What closes it.
    pub(crate) close: &'static str,
    /// The character that makes the one after it part of the string, if any.
    pub(crate) escape: Option<u8>,
    /// Whether it goes on to the next line when it is not closed on its own.
    pub(crate) spans_lines: bool,
}

impl Quote {
    /// A string on one line, with backslash escapes.
    pub(crate) const fn line(open: &'static str, close: &'static str) -> Quote {
        Quote {
            open,
            close,
            escape: Some(b'\\'),
            spans_lines: false,
        }
    }

    /// A string that may go over several lines, with backslash escapes.
    pub(crate) const fn lines(open: &'static str, close: &'static str) -> Quote {
        Quote {
            spans_lines: true,
            ..Quote::line(open, close)
        }
    }

    /// A string on one line, with no escapes.
    pub(crate) const fn raw(open: &'static str, close: &'static str) -> Quote {
        Quote {
            escape: None,
            ..Quote::line(open, close)
        }
    }

    /// A string that may go over several lines, with no escapes.
    pub(crate) const fn raw_lines(open: &'static str, close: &'static str) -> Quote {
        Quote {
            escape: None,
            ..Quote::lines(open, close)
        }
    }
}

/// What the lexer of code needs of a language. The lists of words are words between single spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a trait of languages of its own, which some have"
)]
pub(crate) struct Grammar {
    /// What starts a comment to the end of the line.
    pub(crate) line_comments: &'static [&'static str],
    /// What opens and what closes a block comment.
    pub(crate) block_comments: &'static [(&'static str, &'static str)],
    /// The kinds of strings, those whose opening starts another's first.
    pub(crate) quotes: &'static [Quote],
    /// The keywords.
    pub(crate) keywords: &'static str,
    /// The names of types that do not start with a capital letter.
    pub(crate) types: &'static str,
    /// The names of constants.
    pub(crate) constants: &'static str,
    /// The characters that make the name after them a token of a kind: `$name`, `@decorator`, `#[attribute]`.
    pub(crate) marks: &'static [(u8, TokenKind)],
    /// The names that are part of a string when one opens right after them: `r"…"`, `f"…"`.
    pub(crate) string_prefixes: &'static [&'static str],
    /// Whether the keywords are the same in capitals and small letters, as in SQL.
    pub(crate) is_case_blind: bool,
    /// Whether a name that starts with a capital letter is a type, and one all in capitals a constant.
    pub(crate) capital_types: bool,
    /// Whether a name or a string before a colon is a property: the keys of JSON, YAML and CSS.
    pub(crate) keys_before_colons: bool,
    /// Whether a name at the start of a line before `=` is a property: the keys of TOML and INI.
    pub(crate) keys_before_equals: bool,
    /// Whether a line that starts with `[` is the header of a section, as in TOML and INI.
    pub(crate) has_sections: bool,
    /// Whether names have dashes inside them, as in CSS and YAML.
    pub(crate) has_dashes: bool,
    /// Whether `'name` is a lifetime, as in Rust, rather than the start of a character.
    pub(crate) has_lifetimes: bool,
    /// Whether `name!` is a macro called, as in Rust.
    pub(crate) has_macros: bool,
    /// Whether a name like `Get-ChildItem` is a command, as in PowerShell.
    pub(crate) has_cmdlets: bool,
    /// What a name is that nothing else tells.
    pub(crate) bare: TokenKind,
}

/// A grammar with nothing in it, whose names are variables: the start of the others.
pub(crate) const NAME: Grammar = Grammar {
    line_comments: &[],
    block_comments: &[],
    quotes: &[],
    keywords: "",
    types: "",
    constants: "",
    marks: &[],
    string_prefixes: &[],
    is_case_blind: false,
    capital_types: true,
    keys_before_colons: false,
    keys_before_equals: false,
    has_sections: false,
    has_dashes: false,
    has_lifetimes: false,
    has_macros: false,
    has_cmdlets: false,
    bare: TokenKind::Variable,
};

/// Whether a word is among words between single spaces.
pub(crate) fn is_among(words: &str, word: &str, is_case_blind: bool) -> bool {
    if is_case_blind {
        words.split(' ').any(|known| known.eq_ignore_ascii_case(word))
    } else {
        words.split(' ').any(|known| known == word)
    }
}
