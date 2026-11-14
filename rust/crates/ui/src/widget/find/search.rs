//! What find in page searches: the text drawn in a frame, recorded as it is drawn, and the rows of the lists that
//! search all their rows, in and out of sight; and where each match is.

use std::ops::Range;

use render::{Point, Rect, TextRun, TextStyle};

use crate::id::WidgetId;
use crate::layer::Layer;

/// Whether the case of letters matters when finding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchCase {
    /// `Login` finds `login` and `LOGIN`.
    #[default]
    Ignore,
    /// `Login` finds `Login` only.
    Exact,
}

/// The places of a text where a query is found, as ranges of its bytes, one after the other with no overlap.
pub(crate) fn occurrences(text: &str, query: &str, case: MatchCase) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    if query.is_empty() {
        return found;
    }
    let mut from = 0;
    while let Some(rest) = text.get(from..) {
        let Some(range) = first_occurrence(rest, query, case) else {
            break;
        };
        found.push(from + range.start..from + range.end);
        from += range.end.max(range.start + 1);
        while !text.is_char_boundary(from) && from < text.len() {
            from += 1;
        }
    }
    found
}

/// How many times a query is found in a text.
pub(crate) fn count_occurrences(text: &str, query: &str, case: MatchCase) -> usize {
    occurrences(text, query, case).len()
}

/// The first place of a text where a query is found.
fn first_occurrence(text: &str, query: &str, case: MatchCase) -> Option<Range<usize>> {
    if case == MatchCase::Exact {
        return text.find(query).map(|start| start..start + query.len());
    }
    if text.is_ascii() && query.is_ascii() {
        let (haystack, needle) = (text.as_bytes(), query.as_bytes());
        return haystack
            .windows(needle.len())
            .position(|window| window.eq_ignore_ascii_case(needle))
            .map(|start| start..start + needle.len());
    }
    text.char_indices()
        .find_map(|(start, _)| matched_length(text.get(start..)?, query).map(|length| start..start + length))
}

/// The length in bytes of the start of a text that matches a query in any case, if it does.
fn matched_length(text: &str, query: &str) -> Option<usize> {
    let mut characters = text.chars();
    let mut length = 0;
    for wanted in query.chars() {
        let found = characters.next()?;
        if !found.to_lowercase().eq(wanted.to_lowercase()) {
            return None;
        }
        length += found.len_utf8();
    }
    Some(length)
}

/// The row of a list a text was drawn in, for a list that searches its rows itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RowOwner {
    /// The list.
    pub(crate) list: WidgetId,
    /// The row.
    pub(crate) row: usize,
}

/// A line of text drawn in a frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawnText {
    /// The text.
    pub(crate) text: String,
    /// Its typeface and size.
    pub(crate) style: TextStyle,
    /// The top left corner of the line.
    pub(crate) origin: Point,
    /// The widest it may be before it is cut with an ellipsis.
    pub(crate) max_width: Option<f32>,
    /// The clip it was drawn in.
    pub(crate) clip: Rect,
    /// The layer it was drawn on.
    pub(crate) layer: Layer,
    /// The row of a searching list it was drawn in.
    pub(crate) owner: Option<RowOwner>,
}

/// A list that searched all its rows in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SearchedList {
    /// The list.
    pub(crate) list: WidgetId,
    /// Where it is.
    pub(crate) rect: Rect,
    /// The height of its rows.
    pub(crate) row_height: f32,
}

/// The text drawn in a frame and the lists that searched their rows, while find in page is open.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Recording {
    /// Whether text is recorded in this frame.
    is_on: bool,
    /// Whether recording is held, while the find bar draws itself.
    is_held: bool,
    /// The row of a searching list being drawn.
    owner: Option<RowOwner>,
    /// The text drawn, in order.
    pub(crate) texts: Vec<DrawnText>,
    /// The lists that searched their rows.
    pub(crate) lists: Vec<SearchedList>,
}

impl Recording {
    /// A recording for a frame, on while there is something to find.
    pub(crate) fn new(is_on: bool) -> Recording {
        Recording {
            is_on,
            ..Recording::default()
        }
    }

    /// Whether text drawn now is recorded.
    pub(crate) fn is_on(&self) -> bool {
        self.is_on && !self.is_held
    }

    /// Holds the recording, or lets it go on.
    pub(crate) fn hold(&mut self, is_held: bool) {
        self.is_held = is_held;
    }

    /// Says which row of a searching list is being drawn, or that none is.
    pub(crate) fn set_owner(&mut self, owner: Option<RowOwner>) {
        self.owner = owner;
    }

    /// Records a line of text drawn.
    pub(crate) fn record(&mut self, run: &TextRun<'_>, (clip, layer): (Rect, Layer)) {
        self.texts.push(DrawnText {
            text: run.text.to_owned(),
            style: run.style,
            origin: run.origin,
            max_width: run.max_width,
            clip,
            layer,
            owner: self.owner,
        });
    }
}

/// The rows of a list where a query is found, kept between frames so that a million rows are searched once per query.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RowMatches {
    /// The query searched.
    pub(crate) query: String,
    /// Whether case mattered.
    pub(crate) case: MatchCase,
    /// The version of the rows the view gave.
    pub(crate) generation: u64,
    /// How many rows there were.
    pub(crate) row_count: usize,
    /// Each row with a match, and the number of the first of its matches among all the list's.
    pub(crate) rows: Vec<(usize, usize)>,
    /// How many matches there are.
    pub(crate) total: usize,
}

/// What the rows of a list are searched for: what [`RowMatches`] was made from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowSearch<'query> {
    /// The query.
    pub(crate) query: &'query str,
    /// Whether case matters.
    pub(crate) case: MatchCase,
    /// The version of the rows.
    pub(crate) generation: u64,
    /// How many rows there are.
    pub(crate) row_count: usize,
}

impl RowMatches {
    /// Whether these matches were made for a search.
    pub(crate) fn is_for(&self, search: &RowSearch<'_>) -> bool {
        self.query == search.query
            && self.case == search.case
            && self.generation == search.generation
            && self.row_count == search.row_count
    }

    /// Searches every row of a list.
    pub(crate) fn search(search: &RowSearch<'_>, row_text: &dyn Fn(usize) -> String) -> RowMatches {
        let mut rows = Vec::new();
        let mut total = 0;
        for row in 0..search.row_count {
            let count = count_occurrences(&row_text(row), search.query, search.case);
            if count > 0 {
                rows.push((row, total));
                total += count;
            }
        }
        RowMatches {
            query: search.query.to_owned(),
            case: search.case,
            generation: search.generation,
            row_count: search.row_count,
            rows,
            total,
        }
    }

    /// The row of a match, by its number among the list's, and its number among the row's.
    pub(crate) fn locate(&self, index: usize) -> Option<(usize, usize)> {
        if index >= self.total {
            return None;
        }
        let after = self.rows.partition_point(|(_, first)| *first <= index);
        let (row, first) = self.rows.get(after.checked_sub(1)?)?;
        Some((*row, index - first))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occurrences_are_found_in_any_case_or_exactly_without_overlap() {
        assert_eq!(
            occurrences("Add the login page, LOGIN", "login", MatchCase::Ignore),
            vec![8..13, 20..25]
        );
        assert_eq!(
            occurrences("Add the login page, LOGIN", "login", MatchCase::Exact),
            vec![8..13]
        );
        assert_eq!(occurrences("aaaa", "aa", MatchCase::Ignore), vec![0..2, 2..4]);
        assert_eq!(
            occurrences("Ärger über Äpfel", "äpfel", MatchCase::Ignore),
            vec![13..19]
        );
        assert_eq!(
            occurrences("anything", "", MatchCase::Ignore),
            Vec::<Range<usize>>::new()
        );
    }

    #[test]
    fn the_matches_of_a_list_are_counted_once_and_located_by_number() {
        let texts = ["fix the login", "add a page", "login, login", "readme"];
        let search = RowSearch {
            query: "login",
            case: MatchCase::Ignore,
            generation: 1,
            row_count: texts.len(),
        };
        let matches = RowMatches::search(&search, &|row| texts[row].to_owned());
        assert!(matches.is_for(&search));
        assert_eq!(matches.total, 3);
        assert_eq!(matches.locate(0), Some((0, 0)));
        assert_eq!(matches.locate(2), Some((2, 1)));
        assert_eq!(matches.locate(3), None);
    }
}
