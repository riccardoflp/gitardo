//! The colors of the code of a File page (`file-view.highlight`): the tokens of each line, cut by the language of the
//! file, and the parts of each line in the kind the editor colors, kept up to date as the text is edited.

use text::highlight::{HighlightInput, Language, LineTokens, LinesReplaced, TokenKind, highlight, highlight_again};
use ui::editor::{Buffer, Change};
use ui::{CodeKind, CodeSpan};

/// The colors of a text.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Colors {
    /// The language, told by the name of the file; none for plain text.
    language: Option<Language>,
    /// The tokens of each line, with what is open at its end.
    tokens: Vec<LineTokens>,
    /// The parts of each line the editor colors.
    pub spans: Vec<Vec<CodeSpan>>,
}

impl Colors {
    /// The colors of the whole text of a file.
    #[must_use]
    pub fn of(path: &str, buffer: &Buffer) -> Colors {
        let tokens = highlight(&HighlightInput {
            path,
            text: buffer.text(),
        });
        let spans = tokens.iter().map(spans_of).collect();
        Colors {
            language: Language::of(path),
            tokens,
            spans,
        }
    }

    /// Colors again the lines a change of the text touched, and those after them as far as it matters.
    pub fn follow(&mut self, buffer: &Buffer, change: &Change) {
        let replaced = LinesReplaced {
            start: change.lines.start,
            removed: change.lines.removed,
            added: change.lines.added,
        };
        let line_of = |line: usize| buffer.line(line).to_owned();
        let recolored = highlight_again((self.language, &line_of), &mut self.tokens, replaced);
        let start = replaced.start.min(self.spans.len());
        let end = (replaced.start + replaced.removed).min(self.spans.len());
        let _replaced: Vec<Vec<CodeSpan>> = self
            .spans
            .splice(start..end, std::iter::repeat_n(Vec::new(), replaced.added))
            .collect();
        for line in recolored {
            if let (Some(spans), Some(tokens)) = (self.spans.get_mut(line), self.tokens.get(line)) {
                *spans = spans_of(tokens);
            }
        }
    }
}

/// The colors of the code of each line of a whole text, as a side of a Diff page is colored: never a line out of its
/// context.
#[must_use]
pub fn colors_of(path: &str, text: &str) -> Vec<Vec<CodeSpan>> {
    highlight(&HighlightInput { path, text }).iter().map(spans_of).collect()
}

/// The parts of a line in the kinds the editor colors.
fn spans_of(line: &LineTokens) -> Vec<CodeSpan> {
    line.tokens
        .iter()
        .map(|token| CodeSpan {
            range: token.range.clone(),
            kind: kind_of(token.kind),
        })
        .collect()
}

/// The kind of a token, as the theme colors it.
#[must_use]
pub fn kind_of(kind: TokenKind) -> CodeKind {
    match kind {
        TokenKind::Comment => CodeKind::Comment,
        TokenKind::String => CodeKind::String,
        TokenKind::Number => CodeKind::Number,
        TokenKind::Keyword => CodeKind::Keyword,
        TokenKind::Operator => CodeKind::Operator,
        TokenKind::Punctuation => CodeKind::Punctuation,
        TokenKind::Type => CodeKind::Type,
        TokenKind::Function => CodeKind::Function,
        TokenKind::Variable => CodeKind::Variable,
        TokenKind::Property => CodeKind::Property,
        TokenKind::Constant => CodeKind::Constant,
        TokenKind::Tag => CodeKind::Tag,
        TokenKind::Attribute => CodeKind::Attribute,
        TokenKind::Plain => CodeKind::Plain,
    }
}
