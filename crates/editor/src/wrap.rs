//! Long lines wrapped into the rows they take on screen, in a font whose characters are all one column wide: broken
//! after the spaces between words where a row can end, and inside a word only when it is longer than a row.

use std::ops::Range;

/// The room a line is wrapped in: how many columns a row has, and how many columns a tab goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WrapWidth {
    /// How many columns a row has; a row always takes at least one character, even when it is narrower.
    pub columns: usize,
    /// How many columns a tab goes to.
    pub tab_width: usize,
}

/// How many columns a character takes after so many columns of its row: a tab goes to the next stop, any other
/// character takes one.
#[must_use]
pub fn columns_of(character: char, before: usize, tab_width: usize) -> usize {
    if character == '\t' {
        let width = tab_width.max(1);
        width - before % width
    } else {
        1
    }
}

/// Whether a row can end after a character: after a space or a tab, which stay at the end of the row they close even
/// past its width, as text with its spaces kept does on the web (`white-space: pre-wrap`).
fn is_break_after(character: char) -> bool {
    character == ' ' || character == '\t'
}

/// The rows a line takes in a width, as ranges of its bytes, one after the other from its start to its end: an empty
/// line takes one empty row. A row ends after the last space that lets what follows fit; a word longer than a row is
/// cut where it overflows. Each row counts its columns from its own start, so a tab goes to a stop of its row.
#[must_use]
pub fn wrap_line(line: &str, width: WrapWidth) -> Vec<Range<usize>> {
    let columns = width.columns.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut used = 0;
    // Where the row may end: after its latest space.
    let mut last_break: Option<usize> = None;
    for (index, character) in line.char_indices() {
        // Twice at most: once at the latest space, and once more before this character when the word after that space
        // still leaves it no room.
        while used + columns_of(character, used, width.tab_width) > columns
            && index > start
            && !is_break_after(character)
        {
            let end = last_break.filter(|end| *end > start).unwrap_or(index);
            rows.push(start..end);
            start = end;
            last_break = None;
            // What came after the last space goes to the new row, its tabs to the stops of that row.
            used = columns_in(line.get(start..index).unwrap_or_default(), width.tab_width);
        }
        used += columns_of(character, used, width.tab_width);
        if is_break_after(character) {
            last_break = Some(index + character.len_utf8());
        }
    }
    rows.push(start..line.len());
    rows
}

/// How many columns a piece of a row takes from the row's start.
fn columns_in(piece: &str, tab_width: usize) -> usize {
    piece
        .chars()
        .fold(0, |used, character| used + columns_of(character, used, tab_width))
}

/// How many rows a line takes in a width: one, without wrapping it, when it is shorter than a row.
#[must_use]
pub fn row_count(line: &str, width: WrapWidth) -> usize {
    if line.len() <= width.columns.max(1) && !line.contains('\t') {
        return 1;
    }
    wrap_line(line, width).len()
}

/// Which of the rows of a line a byte of it is drawn on: the row it starts, or the last row for the end of the line.
#[must_use]
pub fn row_of(rows: &[Range<usize>], offset: usize) -> usize {
    rows.iter().rposition(|row| row.start <= offset).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows of a line, as their texts.
    fn rows(line: &str, columns: usize) -> Vec<&str> {
        let width = WrapWidth { columns, tab_width: 4 };
        wrap_line(line, width)
            .into_iter()
            .map(|row| line.get(row).unwrap_or_default())
            .collect()
    }

    #[test]
    fn a_line_is_broken_after_the_spaces_between_its_words() {
        let cases: [(&str, usize, &[&str]); 7] = [
            ("", 10, &[""]),
            ("short", 10, &["short"]),
            ("the quick brown fox", 10, &["the quick ", "brown fox"]),
            ("the quick brown fox", 8, &["the ", "quick ", "brown ", "fox"]),
            ("abcdefghij", 4, &["abcd", "efgh", "ij"]),
            ("ab abcdefgh", 4, &["ab ", "abcd", "efgh"]),
            ("word      next", 6, &["word      ", "next"]),
        ];
        for (line, columns, expected) in cases {
            assert_eq!(rows(line, columns), expected, "{line:?} in {columns}");
        }
    }

    #[test]
    fn the_rows_follow_each_other_and_a_tab_goes_to_a_stop_of_its_row() {
        let line = "\tlet total = first + second;";
        let width = WrapWidth {
            columns: 12,
            tab_width: 4,
        };
        let wrapped = wrap_line(line, width);
        assert_eq!(wrapped.first().map(|row| row.start), Some(0));
        assert_eq!(wrapped.last().map(|row| row.end), Some(line.len()));
        let follow = wrapped
            .windows(2)
            .all(|pair| pair.first().map(|row| row.end) == pair.get(1).map(|row| row.start));
        assert!(follow, "{wrapped:?}");
        assert_eq!(rows(line, 12), ["\tlet ", "total = ", "first + ", "second;"]);
        assert_eq!(row_count(line, width), 4);
        assert_eq!(row_count("short", width), 1);
    }

    #[test]
    fn a_byte_is_on_the_row_it_starts_and_the_end_on_the_last() {
        let wrapped = wrap_line(
            "one two three",
            WrapWidth {
                columns: 5,
                tab_width: 4,
            },
        );
        assert_eq!(wrapped, [0..4, 4..8, 8..13]);
        assert_eq!(row_of(&wrapped, 0), 0);
        assert_eq!(row_of(&wrapped, 3), 0);
        assert_eq!(row_of(&wrapped, 4), 1);
        assert_eq!(row_of(&wrapped, 13), 2);
    }

    #[test]
    fn characters_of_several_bytes_are_never_cut() {
        let line = "àèìòùàèìòù";
        let wrapped = wrap_line(
            line,
            WrapWidth {
                columns: 3,
                tab_width: 4,
            },
        );
        assert_eq!(wrapped.len(), 4);
        assert!(wrapped.iter().all(|row| line.get(row.clone()).is_some()));
    }
}
