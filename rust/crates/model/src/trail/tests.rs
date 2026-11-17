#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{WayChange, folded, page_trail, way_to};

#[test]
fn ui_page_trail__a_page_opened_from_the_page_shown_is_added_after_it() {
    let cases: [(&[char], char, &[char]); 3] = [
        (&[], 'a', &['a']),
        (&['a'], 'b', &['a', 'b']),
        (&['a', 'b'], 'c', &['a', 'b', 'c']),
    ];
    for (way, page, expected) in cases {
        assert_eq!(page_trail(way, WayChange::Opened(page)), expected, "{way:?} {page}");
    }
}

#[test]
fn ui_page_trail__a_page_on_the_way_already_is_gone_back_to_rather_than_added_again() {
    assert_eq!(page_trail(&['a', 'b', 'c'], WayChange::Opened('a')), ['a']);
    assert_eq!(page_trail(&['a', 'b', 'c'], WayChange::Opened('b')), ['a', 'b']);
    assert_eq!(page_trail(&['a', 'b', 'c'], WayChange::Opened('c')), ['a', 'b', 'c']);
}

#[test]
fn ui_page_trail__a_page_opened_from_the_bar_of_views_quick_search_the_side_column_or_a_key_starts_the_way_again() {
    assert_eq!(page_trail(&['a', 'b'], WayChange::StartedAgain('c')), ['c']);
    assert_eq!(page_trail(&['a', 'b'], WayChange::StartedAgain('a')), ['a']);
    assert_eq!(page_trail(&['a', 'b'], WayChange::Timeline), [] as [char; 0]);
}

#[test]
fn ui_page_trail__going_back_to_a_name_drops_what_came_after_it() {
    assert_eq!(page_trail(&['a', 'b', 'c', 'd'], WayChange::Back(1)), ['a', 'b']);
    assert_eq!(page_trail(&['a', 'b', 'c', 'd'], WayChange::Back(0)), ['a']);
    assert_eq!(
        page_trail(&['a', 'b'], WayChange::Back(5)),
        ['a', 'b'],
        "past its end: as it was"
    );
}

#[test]
fn ui_page_trail__a_page_closed_is_left_out_and_a_way_that_does_not_end_at_the_page_shown_is_that_page_alone() {
    assert_eq!(way_to(&['a', 'b', 'c'], &['a', 'c', 'x'], &'c'), ['a', 'c']);
    assert_eq!(way_to(&['a', 'b'], &['a', 'b', 'x'], &'x'), ['x']);
    assert_eq!(way_to(&[], &['x'], &'x'), ['x']);
}

#[test]
fn ui_page_trail__at_most_four_names_are_shown_those_in_between_folded() {
    assert_eq!(folded(2), None);
    assert_eq!(folded(4), None);
    assert_eq!(folded(5), Some(1..2));
    assert_eq!(folded(7), Some(1..4));
}
