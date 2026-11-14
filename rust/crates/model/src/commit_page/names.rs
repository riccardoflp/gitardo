//! The names of files as the header of a diff writes them.

use std::fmt::Write;

/// A name as git writes it in the header of a diff: as it is, or, when it holds a double quote, a backslash or a control
/// character, between double quotes with those characters escaped as in C. Characters beyond ASCII stay as they are,
/// since every read runs with `core.quotepath=false` (git's documentation of `core.quotePath`).
#[must_use]
pub(crate) fn quoted_name(name: &str) -> String {
    if !name.chars().any(needs_escape) {
        return name.to_owned();
    }
    let mut quoted = String::with_capacity(name.len() + 2);
    quoted.push('"');
    for character in name.chars() {
        match character {
            '\u{7}' => quoted.push_str("\\a"),
            '\u{8}' => quoted.push_str("\\b"),
            '\t' => quoted.push_str("\\t"),
            '\n' => quoted.push_str("\\n"),
            '\u{b}' => quoted.push_str("\\v"),
            '\u{c}' => quoted.push_str("\\f"),
            '\r' => quoted.push_str("\\r"),
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            other if needs_escape(other) => {
                // Writing to a string cannot fail.
                let _ = write!(quoted, "\\{:03o}", u32::from(other));
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// Whether git escapes a character of a name whatever `core.quotePath` says.
fn needs_escape(character: char) -> bool {
    character == '"' || character == '\\' || character.is_ascii_control()
}

#[cfg(test)]
mod tests {
    use super::quoted_name;

    #[test]
    fn quotes_only_names_that_need_it() {
        let cases = [
            ("plain.txt", "plain.txt"),
            ("with space.txt", "with space.txt"),
            ("accentué.txt", "accentué.txt"),
            ("a/tab\tname.txt", "\"a/tab\\tname.txt\""),
            ("a/q\"uote.txt", "\"a/q\\\"uote.txt\""),
            ("back\\slash", "\"back\\\\slash\""),
            ("bell\u{7}\u{1}", "\"bell\\a\\001\""),
        ];
        for (name, expected) in cases {
            assert_eq!(quoted_name(name), expected, "{name}");
        }
    }
}
