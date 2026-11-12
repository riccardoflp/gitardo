//! The title of a message: its subject, or the first sentence of a subject too long to be a title.

use super::unwrap::unwrap_message;

/// The longest subject, in characters, that is the title as it is.
const LONGEST_TITLE: usize = 100;
/// How far from the start of a long subject, in characters, its first sentence must end at least.
const SHORTEST_SENTENCE: usize = 20;
/// How far from the start of a long subject, in characters, its first sentence must end at most.
const LONGEST_SENTENCE: usize = 120;
/// Before which character a long subject with no sentence to take is cut at a space.
const CUT_BEFORE: usize = 80;
/// What a title cut at a space ends with, and the text after it starts with.
const ELLIPSIS: char = '\u{2026}';

/// The message of a commit as it was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMessage<'a> {
    /// Its text: the subject, an empty line and the body.
    pub text: &'a str,
}

/// A message as the inspector and the page of a commit show it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ShownMessage {
    /// The first paragraph, as far as its first line goes once it is unwrapped.
    pub subject: String,
    /// Everything after the subject, unwrapped and trimmed.
    pub body: String,
    /// What is shown in bold.
    pub title: String,
    /// What is shown under the title.
    pub text: String,
}

/// The subject and the body of a message, and from them its title and the text under it: a subject of at most 100
/// characters is the title; a longer one is cut after its first sentence ending between 20 and 120 characters from its
/// start, or else at the last space before the 80th character with `…`, the rest of it starting the text before the
/// body. The commit is not changed: this is how its message is shown.
///
/// Feature: details.message-title
#[must_use]
pub fn message_title(input: &RawMessage<'_>) -> ShownMessage {
    let unwrapped = unwrap_message(input.text.trim());
    let (subject, body) = unwrapped.split_once('\n').unwrap_or((&unwrapped, ""));
    let subject = subject.trim().to_owned();
    let body = body.trim().to_owned();
    let (title, rest) = cut_subject(&subject);
    let text = match (rest.is_empty(), body.is_empty()) {
        (true, _) => body.clone(),
        (false, true) => rest,
        (false, false) => format!("{rest}\n\n{body}"),
    };
    ShownMessage {
        subject,
        body,
        title,
        text,
    }
}

/// The title a subject gives, and what of it goes to the text.
fn cut_subject(subject: &str) -> (String, String) {
    let characters: Vec<(usize, char)> = subject.char_indices().collect();
    if characters.len() <= LONGEST_TITLE {
        return (subject.to_owned(), String::new());
    }
    if let Some(end) = first_sentence_end(&characters)
        && let Some((byte, _)) = characters.get(end)
    {
        let (title, rest) = subject.split_at(*byte);
        return (title.trim_end().to_owned(), rest.trim_start().to_owned());
    }
    let space = characters
        .iter()
        .take(CUT_BEFORE)
        .rposition(|(_, character)| *character == ' ');
    match space.and_then(|index| characters.get(index)) {
        Some((byte, _)) => {
            let (title, rest) = subject.split_at(*byte);
            (
                format!("{}{ELLIPSIS}", title.trim_end()),
                format!("{ELLIPSIS}{}", rest.trim_start()),
            )
        }
        None => (subject.to_owned(), String::new()),
    }
}

/// Where the first sentence of a subject ends, in characters, when it ends at least 20 and at most 120 characters from
/// the start: a `.`, `!` or `?`, maybe a closing quote, parenthesis or bracket, then white space and a capital letter or
/// a digit.
fn first_sentence_end(characters: &[(usize, char)]) -> Option<usize> {
    let ends = characters.iter().enumerate().filter_map(|(index, (_, character))| {
        if !matches!(character, '.' | '!' | '?') {
            return None;
        }
        let mut end = index + 1;
        if characters
            .get(end)
            .is_some_and(|(_, next)| matches!(next, '"' | '\'' | ')' | ']' | '\u{201d}' | '\u{2019}'))
        {
            end += 1;
        }
        let spaces = characters
            .get(end..)?
            .iter()
            .take_while(|(_, next)| next.is_whitespace())
            .count();
        let starts_sentence = characters
            .get(end + spaces)
            .is_some_and(|(_, next)| next.is_uppercase() || next.is_ascii_digit());
        (spaces > 0 && starts_sentence).then_some(end)
    });
    ends.into_iter()
        .find(|end| *end >= SHORTEST_SENTENCE)
        .filter(|end| *end <= LONGEST_SENTENCE)
}
