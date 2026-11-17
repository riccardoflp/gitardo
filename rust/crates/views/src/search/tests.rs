#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::search::BoxMode;
use ui::MenuEntry;

use super::{HistorySearch, MatchCount, mode_entries};
use crate::icons;
use crate::text::search as words;

#[test]
fn search_box__lists_the_five_modes_in_order_with_loaded_chosen_when_the_window_opens() {
    let entries = mode_entries(BoxMode::default());
    let shown: Vec<(&str, bool)> = entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some((item.label, item.icon == Some(icons::CHOSEN))),
            _ => None,
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("Loaded", true),
            ("Message", false),
            ("Author", false),
            ("Changes", false),
            ("Changes (regex)", false)
        ]
    );
    assert_eq!(words::placeholder(BoxMode::Loaded), "Find by message, author or hash");
}

#[test]
fn search_box__counts_the_matches_and_the_place_of_the_one_selected() {
    let mut found = HistorySearch {
        is_looking: false,
        is_listing: false,
        count: Some(MatchCount {
            selected: None,
            total: 14,
            has_more: false,
        }),
    };
    assert_eq!(found.count_words().as_deref(), Some("14"));
    found.count = found.count.map(|count| MatchCount {
        selected: Some(2),
        ..count
    });
    assert_eq!(found.count_words().as_deref(), Some("2/14"));
    found.count = Some(MatchCount {
        selected: None,
        total: 5_000,
        has_more: true,
    });
    assert_eq!(found.count_words().as_deref(), Some("5000+"));
    found.is_looking = true;
    assert_eq!(found.count_words(), None);
}

#[test]
fn search_history__says_the_first_line_of_what_git_refused() {
    assert_eq!(
        words::search_failed("fatal: command line, '(': Unmatched ( or \\(\nmore\n"),
        r"Gitardo: the search failed. fatal: command line, '(': Unmatched ( or \("
    );
}
