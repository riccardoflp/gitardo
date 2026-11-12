//! Searching the history: the ways of searching it, what a search found, and stepping through it.

use crate::CommitId;

/// How a search of the whole history looks at the commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// The commits a search of the whole history found, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Matches {
    /// The commits found, at most as many as a search keeps.
    pub commits: Vec<CommitId>,
    /// Whether the search found more than those kept.
    pub has_more: bool,
}

/// Which way to step through the matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// To the match after the one selected: `Enter`, `F3`, the arrow down.
    Next,
    /// To the match before it: `Shift+Enter`, `Shift+F3`, the arrow up.
    Previous,
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

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Direction, step};
    use crate::CommitId;

    /// Fourteen matches, and a commit that is none of them.
    fn matches() -> (Vec<CommitId>, CommitId) {
        let commit = |number: usize| CommitId::parse(&format!("{number:040x}")).unwrap();
        ((1..=14).map(commit).collect(), commit(99))
    }

    #[test]
    fn search_step__goes_from_the_last_to_the_first() {
        let (matches, _) = matches();
        assert_eq!(
            step(&matches, matches.get(13), Direction::Next),
            matches.first().cloned()
        );
        assert_eq!(step(&matches, matches.get(4), Direction::Next), matches.get(5).cloned());
    }

    #[test]
    fn search_step__goes_from_the_first_to_the_last() {
        let (matches, _) = matches();
        assert_eq!(
            step(&matches, matches.first(), Direction::Previous),
            matches.get(13).cloned()
        );
    }

    #[test]
    fn search_step__starts_at_an_end_when_the_selected_commit_is_no_match() {
        let (matches, other) = matches();
        assert_eq!(
            step(&matches, Some(&other), Direction::Previous),
            matches.get(13).cloned()
        );
        assert_eq!(step(&matches, Some(&other), Direction::Next), matches.first().cloned());
        assert_eq!(step(&matches, None, Direction::Next), matches.first().cloned());
        assert_eq!(step(&[], Some(&other), Direction::Next), None);
    }
}
