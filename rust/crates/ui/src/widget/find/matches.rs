//! The matches of a page in reading order: those of the text drawn, and those of the lists that search all their rows,
//! each list taking the place of its rows; and where the match of a number is.

use std::collections::HashMap;

use render::{Fonts, Rect};

use super::search::{DrawnText, MatchCase, Recording, RowMatches, occurrences};
use crate::id::WidgetId;
use crate::layer::Layer;

/// Where a match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// In a text drawn, by its place in the recording and the number of the match in it.
    Text {
        /// The text, by its place in the recording.
        text: usize,
        /// Which of its matches.
        occurrence: usize,
    },
    /// In a row of a list that searches its rows, in sight or not.
    Row {
        /// The list.
        list: WidgetId,
        /// The row.
        row: usize,
        /// Which of the row's matches.
        occurrence: usize,
    },
}

/// A group of matches in reading order: a text drawn, or a list.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Group {
    /// A text, by its place in the recording.
    Text(usize),
    /// A list.
    List(WidgetId),
}

/// The matches of a page, in reading order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PageMatches {
    /// Each group with the number of its first match and how many it has.
    groups: Vec<(Group, usize, usize)>,
    /// How many matches there are.
    pub(crate) total: usize,
}

/// What a page is searched for, and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageSearch<'query> {
    /// The page.
    pub(crate) page: Rect,
    /// The query.
    pub(crate) query: &'query str,
    /// Whether case matters.
    pub(crate) case: MatchCase,
}

/// Whether a text drawn is part of a page and counted with it: drawn on the base layer, in sight on the page, and not
/// in a row of a list that counts its own rows.
fn is_counted(text: &DrawnText, recording: &Recording, page: Rect) -> bool {
    let is_in_searched_row = text
        .owner
        .is_some_and(|owner| recording.lists.iter().any(|list| list.list == owner.list));
    text.layer == Layer::Base && !is_in_searched_row && !text.clip.intersection(page).is_empty()
}

/// The matches of a page, from what was drawn on it and the lists that searched their rows.
pub(crate) fn page_matches(
    recording: &Recording,
    lists: &HashMap<WidgetId, RowMatches>,
    search: PageSearch<'_>,
) -> PageMatches {
    let mut anchored: Vec<((f32, f32), Group, usize)> = Vec::new();
    for (index, text) in recording.texts.iter().enumerate() {
        if !is_counted(text, recording, search.page) {
            continue;
        }
        let count = occurrences(&text.text, search.query, search.case).len();
        if count > 0 {
            anchored.push(((text.origin.y, text.origin.x), Group::Text(index), count));
        }
    }
    for list in &recording.lists {
        let count = lists.get(&list.list).map_or(0, |matches| matches.total);
        if count > 0 && !list.rect.intersection(search.page).is_empty() {
            anchored.push(((list.rect.y, list.rect.x), Group::List(list.list), count));
        }
    }
    anchored.sort_by(|(first, ..), (second, ..)| first.0.total_cmp(&second.0).then(first.1.total_cmp(&second.1)));
    let mut total = 0;
    let groups = anchored
        .into_iter()
        .map(|(_, group, count)| {
            let entry = (group, total, count);
            total += count;
            entry
        })
        .collect();
    PageMatches { groups, total }
}

impl PageMatches {
    /// Where the match of a number is.
    pub(crate) fn place(&self, index: usize, lists: &HashMap<WidgetId, RowMatches>) -> Option<Place> {
        let (group, first, _) = self
            .groups
            .iter()
            .find(|(_, first, count)| index >= *first && index < first + count)?;
        let within = index - first;
        match group {
            Group::Text(text) => Some(Place::Text {
                text: *text,
                occurrence: within,
            }),
            Group::List(list) => {
                let (row, occurrence) = lists.get(list)?.locate(within)?;
                Some(Place::Row {
                    list: *list,
                    row,
                    occurrence,
                })
            }
        }
    }
}

/// Where each match of a text drawn is, in logical pixels, or nothing for a match cut off by the ellipsis.
pub(crate) fn match_rects(fonts: &Fonts, text: &DrawnText, query: &str, case: MatchCase) -> Vec<Option<Rect>> {
    let shown = text.max_width.map_or(text.text.len(), |width| {
        fonts.truncate(&text.text, text.style, width).shown
    });
    let height = fonts.line_metrics(text.style).height;
    let width_to = |end: usize| fonts.measure(text.text.get(..end).unwrap_or_default(), text.style);
    occurrences(&text.text, query, case)
        .into_iter()
        .map(|range| {
            if range.start >= shown {
                return None;
            }
            let start = width_to(range.start);
            let end = width_to(range.end.min(shown));
            Some(Rect::new(text.origin.x + start, text.origin.y, end - start, height))
        })
        .collect()
}
