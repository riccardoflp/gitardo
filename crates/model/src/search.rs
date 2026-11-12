//! Searching the history: the ways of searching it, what a search found, finding among the loaded commits, and stepping
//! through what was found.

use crate::CommitId;
use crate::graph::Row;

#[cfg(test)]
mod tests;

/// How a search of the whole history looks at the commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchMode {
    /// The text is in the message, ignoring case.
    Message,
    /// The text is in the name or the address of the author, ignoring case.
    Author,
    /// The commit adds or removes the text.
    Changes,
    /// A line the commit changed matches the text, a POSIX extended regular expression.
    ChangesPattern,
}

/// What the search box searches, chosen from the list before it (`search.box`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoxMode {
    /// The commits loaded, as the text is typed (`search.loaded`); the mode when the window opens.
    #[default]
    Loaded,
    /// The messages of the whole history, on `Enter`.
    Message,
    /// The authors of the whole history, on `Enter`.
    Author,
    /// The text added or removed by the commits of the whole history, on `Enter`.
    Changes,
    /// The lines changed by the commits of the whole history, by a regular expression, on `Enter`.
    ChangesPattern,
}

impl BoxMode {
    /// Every mode, in the order of the list.
    pub const ALL: [BoxMode; 5] = [
        BoxMode::Loaded,
        BoxMode::Message,
        BoxMode::Author,
        BoxMode::Changes,
        BoxMode::ChangesPattern,
    ];

    /// How git searches the whole history in this mode; none for the commits loaded.
    #[must_use]
    pub fn history(self) -> Option<SearchMode> {
        match self {
            BoxMode::Loaded => None,
            BoxMode::Message => Some(SearchMode::Message),
            BoxMode::Author => Some(SearchMode::Author),
            BoxMode::Changes => Some(SearchMode::Changes),
            BoxMode::ChangesPattern => Some(SearchMode::ChangesPattern),
        }
    }
}

/// The commits a search of the whole history found, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Matches {
    /// The commits found, at most as many as a search keeps.
    pub commits: Vec<CommitId>,
    /// Whether the search found more than those kept.
    pub has_more: bool,
}

/// Which way to step through the matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// To the match after the one selected: `Enter`, `F3`, the arrow down.
    Next,
    /// To the match before it: `Shift+Enter`, `Shift+F3`, the arrow up.
    Previous,
}

/// The fewest characters of a hash that are looked for as one.
const SHORTEST_HASH: usize = 4;
/// The most characters a hash has.
const LONGEST_HASH: usize = 64;

/// The commits of the rows loaded whose subject or author holds a text, without regard to case and with the spaces at
/// its ends taken away; and, for text of four hexadecimal digits or more, those whose hash starts with it. The draft
/// of the uncommitted changes is no commit to find; the stashes are found like commits. Empty text finds nothing. The
/// result is in the order of the rows.
///
/// Feature: search.loaded
#[must_use]
pub fn loaded(rows: &[Row], text: &str) -> Vec<CommitId> {
    let wanted = text.trim();
    if wanted.is_empty() {
        return Vec::new();
    }
    let lower = wanted.to_lowercase();
    let prefix = (wanted.len() >= SHORTEST_HASH && is_hexadecimal(wanted)).then_some(lower.as_str());
    rows.iter()
        .filter_map(Row::commit)
        .filter(|commit| {
            prefix.is_some_and(|prefix| commit.id.as_str().starts_with(prefix))
                || holds(&commit.subject, &lower)
                || holds(&commit.author_name, &lower)
        })
        .map(|commit| commit.id.clone())
        .collect()
}

/// The text typed as the start of a hash, in lower case: 4 to 64 hexadecimal digits once its spaces are taken away
/// (`search.go-to-hash`); none otherwise.
#[must_use]
pub fn hash_prefix(text: &str) -> Option<String> {
    let wanted = text.trim();
    let is_hash = (SHORTEST_HASH..=LONGEST_HASH).contains(&wanted.len()) && is_hexadecimal(wanted);
    is_hash.then(|| wanted.to_ascii_lowercase())
}

/// Whether every character of a text is a hexadecimal digit, in any case.
fn is_hexadecimal(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Whether a text holds another, given in lower case, without regard to case.
fn holds(text: &str, lower: &str) -> bool {
    if text.is_ascii() && lower.is_ascii() {
        let wanted = lower.as_bytes();
        return wanted.is_empty()
            || text
                .as_bytes()
                .windows(wanted.len())
                .any(|window| window.eq_ignore_ascii_case(wanted));
    }
    text.to_lowercase().contains(lower)
}

/// The match to select next, or before, going round at the ends. With the selected commit not among the matches, the
/// next is the first and the previous the last; with no match there is none.
///
/// Feature: search.step
#[must_use]
pub fn step(matches: &[CommitId], selected: Option<&CommitId>, by: Direction) -> Option<CommitId> {
    let count = matches.len();
    if count == 0 {
        return None;
    }
    let position = selected.and_then(|selected| matches.iter().position(|commit| commit == selected));
    let chosen = match (position, by) {
        (Some(position), Direction::Next) => (position + 1) % count,
        (Some(position), Direction::Previous) => (position + count - 1) % count,
        (None, Direction::Next) => 0,
        (None, Direction::Previous) => count - 1,
    };
    matches.get(chosen).cloned()
}
