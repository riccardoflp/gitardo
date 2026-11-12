//! Words in a line of text, for Ctrl with the arrows, Ctrl with Backspace and Delete, and a double click: a word is a
//! run of letters, digits and underscores, or a run of other marks; spaces are between words.
//!
//! Positions are bytes of the text at the start of a character. Characters are Unicode scalar values: a letter made
//! of several, such as some emoji, is crossed one value at a time.

use std::ops::Range;

/// What a character is, for words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// White space.
    Space,
    /// A letter, a digit or an underscore.
    Word,
    /// Any other mark, such as `/`, `.` or `-`.
    Mark,
}

/// The class of a character.
fn class(character: char) -> Class {
    if character.is_whitespace() {
        Class::Space
    } else if character.is_alphanumeric() || character == '_' {
        Class::Word
    } else {
        Class::Mark
    }
}

/// The start of the character before a position, or the position itself at the start.
pub(crate) fn previous_boundary(text: &str, position: usize) -> usize {
    text.get(..position)
        .and_then(|before| before.char_indices().next_back())
        .map_or(0, |(start, _)| start)
}

/// The end of the character after a position, or the position itself at the end.
pub(crate) fn next_boundary(text: &str, position: usize) -> usize {
    text.get(position..)
        .and_then(|after| after.chars().next())
        .map_or(position, |character| position + character.len_utf8())
}

/// The nearest start of a character at or before a byte, for positions that may fall inside one.
pub(crate) fn floor_boundary(text: &str, position: usize) -> usize {
    let mut position = position.min(text.len());
    while !text.is_char_boundary(position) {
        position -= 1;
    }
    position
}

/// Where Ctrl and the left arrow go: back over spaces, then to the start of the word before them.
pub(crate) fn previous_word_start(text: &str, position: usize) -> usize {
    let before = text.get(..position).unwrap_or_default();
    let mut characters = before.char_indices().rev().peekable();
    while characters
        .next_if(|(_, character)| class(*character) == Class::Space)
        .is_some()
    {}
    let Some(&(mut start, first)) = characters.peek() else {
        return 0;
    };
    let word = class(first);
    while let Some((at, _)) = characters.next_if(|(_, character)| class(*character) == word) {
        start = at;
    }
    start
}

/// Where Ctrl and the right arrow go: past the word the position is in or before, then past the spaces after it, to
/// the start of the next word.
pub(crate) fn next_word_start(text: &str, position: usize) -> usize {
    let after = text.get(position..).unwrap_or_default();
    let mut characters = after.char_indices().peekable();
    if let Some(&(_, first)) = characters.peek()
        && class(first) != Class::Space
    {
        let word = class(first);
        while characters.next_if(|(_, character)| class(*character) == word).is_some() {}
    }
    while characters
        .next_if(|(_, character)| class(*character) == Class::Space)
        .is_some()
    {}
    characters.peek().map_or(text.len(), |(at, _)| position + at)
}

/// The word a double click at a position selects: the run of characters of the same class as the one after the
/// position, or the one before it at the end of the text.
pub(crate) fn word_at(text: &str, position: usize) -> Range<usize> {
    let position = floor_boundary(text, position);
    let anchor = if position >= text.len() {
        previous_boundary(text, position)
    } else {
        position
    };
    let Some(character) = text.get(anchor..).and_then(|after| after.chars().next()) else {
        return position..position;
    };
    let word = class(character);
    let start = text
        .get(..anchor)
        .unwrap_or_default()
        .char_indices()
        .rev()
        .take_while(|(_, character)| class(*character) == word)
        .last()
        .map_or(anchor, |(at, _)| at);
    let end = text
        .get(anchor..)
        .unwrap_or_default()
        .char_indices()
        .find(|(_, character)| class(*character) != word)
        .map_or(text.len(), |(at, _)| anchor + at);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_and_the_arrows_go_from_word_to_word() {
        let text = "git push origin/main  --force";
        assert_eq!(next_word_start(text, 0), 4);
        assert_eq!(next_word_start(text, 4), 9);
        assert_eq!(next_word_start(text, 9), 15);
        assert_eq!(next_word_start(text, 15), 16);
        assert_eq!(next_word_start(text, 16), 22);
        assert_eq!(next_word_start(text, 22), 24);
        assert_eq!(next_word_start(text, 24), text.len());
        assert_eq!(previous_word_start(text, text.len()), 24);
        assert_eq!(previous_word_start(text, 24), 22);
        assert_eq!(previous_word_start(text, 22), 16);
        assert_eq!(previous_word_start(text, 4), 0);
        assert_eq!(previous_word_start(text, 0), 0);
    }

    #[test]
    fn a_double_click_selects_the_word_under_it() {
        let text = "fix the café parser";
        assert_eq!(word_at(text, 9), 8..13);
        assert_eq!(word_at(text, 3), 3..4);
        assert_eq!(word_at(text, text.len()), 14..20);
        assert_eq!(word_at("", 0), 0..0);
        assert_eq!(word_at(text, 12), 8..13);
    }

    #[test]
    fn boundaries_never_split_a_character() {
        let text = "aé";
        assert_eq!(next_boundary(text, 1), 3);
        assert_eq!(previous_boundary(text, 3), 1);
        assert_eq!(floor_boundary(text, 2), 1);
        assert_eq!(next_boundary(text, 3), 3);
    }
}
