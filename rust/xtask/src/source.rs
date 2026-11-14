//! Reading Rust source the way the checks need it: the code without its comments and literals, so that a word is
//! found only where it is code. Written from the lexical structure of the Rust Reference.

/// The source with its comments, strings and characters blanked out with spaces. Line breaks are kept, so every
/// line keeps its number and every word its column.
pub fn code_only(source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut code = String::with_capacity(source.len());
    let mut position = 0;
    while let Some(&character) = characters.get(position) {
        let length = literal_or_comment_length(&characters, position);
        if length == 0 {
            code.push(character);
            position += 1;
            continue;
        }
        for blanked in characters.iter().skip(position).take(length) {
            code.push(if *blanked == '\n' { '\n' } else { ' ' });
        }
        position += length;
    }
    code
}

/// How many characters, from `position`, are a comment or a literal; 0 when code starts there.
fn literal_or_comment_length(characters: &[char], position: usize) -> usize {
    let at = |offset: usize| characters.get(position + offset).copied();
    let follows_identifier = position
        .checked_sub(1)
        .and_then(|previous| characters.get(previous))
        .is_some_and(|previous| previous.is_alphanumeric() || *previous == '_');
    match (at(0), at(1)) {
        (Some('/'), Some('/')) => line_comment_length(characters, position),
        (Some('/'), Some('*')) => block_comment_length(characters, position),
        (Some('"'), _) => quoted_length(characters, position, '"'),
        (Some('\''), _) => character_length(characters, position),
        (Some('b' | 'c'), Some('"' | '\'' | 'r')) if !follows_identifier => {
            let rest = literal_or_comment_length(characters, position + 1);
            if rest == 0 { 0 } else { rest + 1 }
        }
        (Some('r'), Some('"' | '#')) if !follows_identifier => raw_string_length(characters, position),
        _ => 0,
    }
}

fn line_comment_length(characters: &[char], position: usize) -> usize {
    characters
        .iter()
        .skip(position)
        .take_while(|character| **character != '\n')
        .count()
}

/// Block comments nest in Rust: `/* a /* b */ c */` is one comment.
fn block_comment_length(characters: &[char], position: usize) -> usize {
    let mut depth = 0usize;
    let mut offset = 0;
    while let Some(&character) = characters.get(position + offset) {
        let next = characters.get(position + offset + 1).copied();
        match (character, next) {
            ('/', Some('*')) => {
                depth += 1;
                offset += 2;
            }
            ('*', Some('/')) => {
                depth -= 1;
                offset += 2;
                if depth == 0 {
                    return offset;
                }
            }
            _ => offset += 1,
        }
    }
    offset
}

/// A string or character between two `delimiter`s, with backslash escapes.
fn quoted_length(characters: &[char], position: usize, delimiter: char) -> usize {
    let mut offset = 1;
    while let Some(&character) = characters.get(position + offset) {
        offset += if character == '\\' { 2 } else { 1 };
        if character == delimiter {
            return offset;
        }
    }
    offset
}

/// A character literal, or 0 for a lifetime or a label such as `'a`.
fn character_length(characters: &[char], position: usize) -> usize {
    match (characters.get(position + 1), characters.get(position + 2)) {
        (Some('\\'), _) => quoted_length(characters, position, '\''),
        (Some(_), Some('\'')) => 3,
        _ => 0,
    }
}

/// `r"…"`, `r#"…"#` and so on: no escapes, closed by a quote and as many `#` as opened it.
fn raw_string_length(characters: &[char], position: usize) -> usize {
    let hashes = characters
        .iter()
        .skip(position + 1)
        .take_while(|character| **character == '#')
        .count();
    if characters.get(position + 1 + hashes) != Some(&'"') {
        return 0;
    }
    let mut offset = hashes + 2;
    while let Some(&character) = characters.get(position + offset) {
        offset += 1;
        let closing = characters
            .iter()
            .skip(position + offset)
            .take(hashes)
            .filter(|next| **next == '#')
            .count();
        if character == '"' && closing == hashes {
            return offset + hashes;
        }
    }
    offset
}

/// The words of the code, with the line each starts on, counting from 1. A word is what an identifier or a keyword
/// is made of.
pub fn words(code: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let is_word_character = |character: char| character.is_alphanumeric() || character == '_';
        for word in line.split(|character: char| !is_word_character(character)) {
            if !word.is_empty() {
                found.push((index + 1, word));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{code_only, words};

    #[test]
    fn blanks_comments_strings_and_characters_and_keeps_the_lines() {
        let cases = [
            ("let a = 1; // unsafe\nb", "let a = 1;          \nb"),
            (
                "x /* unsafe /* nested */ still */ y",
                "x                                 y",
            ),
            ("f(\"unsafe \\\" quoted\")", "f(                  )"),
            ("r#\"un\"safe\"# z", "             z"),
            ("b\"unsafe\" c'x' '\\n'", "                   "),
            ("fn a<'a>(x: &'a str)", "fn a<'a>(x: &'a str)"),
            ("ab\"c\"", "ab   "),
            ("/* a\nb */c", "    \n    c"),
        ];
        for (source, expected) in cases {
            assert_eq!(code_only(source), expected, "{source:?}");
        }
    }

    #[test]
    fn finds_the_words_and_their_lines() {
        let found = words("use std::fs;\n  unsafe {}");
        assert_eq!(found, vec![(1, "use"), (1, "std"), (1, "fs"), (2, "unsafe")]);
    }
}
