//! Which buttons of the heading of the changes stay on it when the width does not hold them all (`changes.pane`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::actions::{RowAction, buttons_kept, staying};
use crate::icons;
use crate::text::changes as words;

/// The buttons of the heading by their words, in the order the heading has them from its right.
fn buttons(said: &[&'static str]) -> Vec<RowAction> {
    said.iter()
        .map(|words| RowAction::shows(words, icons::MORE, None))
        .collect()
}

/// The words of the buttons that stay, of those of the heading, in a width with places for some.
fn kept_in(said: &[&'static str], places: usize) -> Vec<&'static str> {
    let actions = buttons(said);
    staying(&actions, buttons_kept(actions.len(), places))
        .into_iter()
        .filter_map(|index| actions.get(index).map(|action| action.words))
        .collect()
}

/// Every button the heading can have, with a file staged and one not saved.
const ALL: [&str; 6] = [
    words::DISCARD_ALL,
    words::SAVE_ALL,
    words::STASH_ALL,
    words::STASH_STAGED,
    words::OPEN_ALL_CHANGES,
    words::VIEW_AS_TREE,
];

#[test]
fn changes_pane__the_buttons_of_the_heading_all_stay_while_they_fit_and_one_place_is_of_the_menu_once_they_do_not() {
    assert_eq!(buttons_kept(6, 6), 6);
    assert_eq!(buttons_kept(6, 9), 6);
    assert_eq!(buttons_kept(6, 5), 4, "one of the five places is the menu's");
    assert_eq!(buttons_kept(6, 1), 0, "the only place is the menu's");
    assert_eq!(buttons_kept(6, 0), 0);
    assert_eq!(kept_in(&ALL, 6), ALL);
}

#[test]
fn changes_pane__the_buttons_leave_the_heading_in_their_order_and_open_all_changes_is_the_last_to_go() {
    assert_eq!(
        kept_in(&ALL, 5),
        [
            words::DISCARD_ALL,
            words::SAVE_ALL,
            words::STASH_ALL,
            words::OPEN_ALL_CHANGES
        ],
        "the layout and the stash of the staged changes left"
    );
    assert_eq!(
        kept_in(&ALL, 4),
        [words::DISCARD_ALL, words::STASH_ALL, words::OPEN_ALL_CHANGES]
    );
    assert_eq!(kept_in(&ALL, 3), [words::DISCARD_ALL, words::OPEN_ALL_CHANGES]);
    assert_eq!(kept_in(&ALL, 2), [words::OPEN_ALL_CHANGES]);
    assert_eq!(kept_in(&ALL, 1), [""; 0]);
    // **View as list** leaves as **View as tree** does.
    let as_list = [words::DISCARD_ALL, words::OPEN_ALL_CHANGES, words::VIEW_AS_LIST];
    assert_eq!(kept_in(&as_list, 2), [words::OPEN_ALL_CHANGES]);
}
