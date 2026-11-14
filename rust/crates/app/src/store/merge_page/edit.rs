//! The result of a Merge page changed by lines: the lines of some stretches put in place of others, each line ending as
//! the lines of the file do, and the file ending without a line ending when it did.

use std::ops::Range;

use text::conflicts::split_lines;

/// Lines of the result to put in place of a stretch of its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    /// The lines replaced, by their places in the result.
    pub lines: Range<usize>,
    /// What goes in their place, line by line, without what ends each line.
    pub text: Vec<String>,
}

/// The result with each stretch replaced by its lines; the stretches do not overlap.
#[must_use]
pub fn replaced(result: &str, mut replacements: Vec<Replacement>) -> String {
    let split = split_lines(result);
    let ending = split
        .iter()
        .map(|line| line.ending)
        .find(|ending| !ending.is_empty())
        .unwrap_or("\n");
    let ends_without_ending = split.last().is_some_and(|line| line.ending.is_empty());
    let mut lines: Vec<(String, &str)> = split.iter().map(|line| (line.text.to_owned(), line.ending)).collect();
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.lines.start));
    for replacement in replacements {
        let end = replacement.lines.end.min(lines.len());
        let start = replacement.lines.start.min(end);
        let new = replacement.text.into_iter().map(|line| (line, ending));
        let _removed: Vec<(String, &str)> = lines.splice(start..end, new).collect();
    }
    let count = lines.len();
    let mut text = String::with_capacity(result.len());
    for (place, (line, line_ending)) in lines.iter().enumerate() {
        text.push_str(line);
        let is_last = place + 1 == count;
        if !(is_last && ends_without_ending) {
            text.push_str(if line_ending.is_empty() { ending } else { line_ending });
        }
    }
    text
}

/// The lines of a text, without what ends each.
#[must_use]
pub fn lines_of(text: &str) -> Vec<String> {
    split_lines(text).iter().map(|line| line.text.to_owned()).collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Replacement, replaced};

    fn replacement(lines: std::ops::Range<usize>, text: &[&str]) -> Replacement {
        Replacement {
            lines,
            text: text.iter().map(|line| (*line).to_owned()).collect(),
        }
    }

    #[test]
    fn merge_view_take_side__lines_are_replaced_with_the_ending_of_the_file() {
        let result = "a\r\nb\r\nc\r\n";
        assert_eq!(
            replaced(result, vec![replacement(1..2, &["x", "y"])]),
            "a\r\nx\r\ny\r\nc\r\n"
        );
    }

    #[test]
    fn merge_view_take_side__a_file_without_a_final_ending_keeps_none() {
        assert_eq!(replaced("a\nb", vec![replacement(1..2, &["x"])]), "a\nx");
        assert_eq!(replaced("a\nb", vec![replacement(2..2, &["x"])]), "a\nb\nx");
    }

    #[test]
    fn merge_view_accept_simple__several_stretches_are_replaced_at_once() {
        let result = "a\nb\nc\nd\n";
        let edits = vec![replacement(0..1, &["A"]), replacement(2..4, &[])];
        assert_eq!(replaced(result, edits), "A\nb\n");
    }
}
