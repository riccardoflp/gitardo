//! Find in page over several frames: Ctrl+F opens the bar, what is typed is counted over the text drawn and over every
//! row of a list that can be searched, Enter and Shift+Enter go through the matches, a match out of sight is scrolled
//! into it, and Escape closes the bar.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::{Rect, ThemeName};
use support::{Window, key, shortcut, typed};
use ui::{FindResponse, Label, List, ListSearch, Modifiers, NamedKey, SelectionMode, Ui};

/// How many rows the list has.
const ROWS: usize = 1000;
/// The one row of the list with "needle" in it, far out of sight.
const NEEDLE_ROW: usize = 900;

/// The text of a row of the list.
fn row_text(row: usize) -> String {
    if row == NEEDLE_ROW {
        String::from("a needle in a haystack")
    } else {
        format!("row {row} of hay")
    }
}

/// A page: two labels, one with "needle" in it, over a list of a thousand rows, with find in page over all of it.
/// Returns what find in page said and the first row of the list in sight.
fn page(ui: &mut Ui<'_>) -> (FindResponse, usize) {
    let window = ui.window();
    let _ = ui.label(
        Rect::new(10.0, 10.0, 300.0, 22.0),
        &Label::new("The needle is somewhere"),
    );
    let _ = ui.label(Rect::new(10.0, 40.0, 300.0, 22.0), &Label::new("Nothing here"));
    let list = List {
        row_count: ROWS,
        row_height: 22.0,
        selection_mode: SelectionMode::Single,
    };
    let search = ListSearch {
        row_text: &row_text,
        generation: 1,
    };
    let rect = Rect::new(0.0, 80.0, window.width, window.height - 80.0);
    let answer = ui.searchable_list("rows", (rect, &list), &search, |ui, row| {
        let _ = ui.label(row.rect, &Label::new(&row_text(row.index)));
    });
    let found = ui.find_in_page("find", window);
    (found, answer.visible_rows.start)
}

/// A frame of the page with some input, the time moved on after it.
fn step(window: &mut Window, events: &[ui::InputEvent]) -> (FindResponse, usize) {
    let (answer, _) = window.frame(events, page);
    window.wait(16);
    answer
}

#[test]
fn ctrl_f_opens_the_bar_and_matches_are_counted_in_sight_and_out_of_it() {
    let mut window = Window::new(ThemeName::Dark, (600, 400), 1);
    let _ = step(&mut window, &[]);
    let (found, _) = step(&mut window, &[shortcut('f', Modifiers::CONTROL)]);
    assert!(found.is_open);
    assert_eq!(found.match_count, 0, "nothing is typed yet");
    let _ = step(&mut window, &[typed("needle")]);
    let (found, _) = step(&mut window, &[]);
    assert_eq!(found.match_count, 2, "the label and the row out of sight");
    assert_eq!(found.current, Some(0));
    let (found, _) = step(&mut window, &[key(NamedKey::Enter, Modifiers::NONE)]);
    assert_eq!(found.current, Some(1));
    let (_, first) = step(&mut window, &[]);
    let in_sight = 320 / 22;
    // The row comes into sight at the bottom of the list, and the first row in sight is cut by the top.
    assert!(
        first <= NEEDLE_ROW && NEEDLE_ROW <= first + in_sight,
        "the row is scrolled into sight: {first}"
    );
    let (found, _) = step(&mut window, &[key(NamedKey::Enter, Modifiers::SHIFT)]);
    assert_eq!(found.current, Some(0));
    let (found, _) = step(&mut window, &[key(NamedKey::Enter, Modifiers::SHIFT)]);
    assert_eq!(found.current, Some(1), "Shift+Enter goes round to the last match");
    let (found, _) = step(&mut window, &[key(NamedKey::Escape, Modifiers::NONE)]);
    assert!(!found.is_open);
}

/// Where the switch for the case of letters is in the bar, at the top right of the page.
const CASE_BUTTON: (f32, f32) = (600.0 - 114.0, 21.0);

#[test]
fn the_case_of_letters_does_not_matter_until_it_is_asked_to() {
    let mut window = Window::new(ThemeName::Dark, (600, 400), 1);
    let _ = step(&mut window, &[]);
    let _ = step(&mut window, &[shortcut('f', Modifiers::CONTROL)]);
    let _ = step(&mut window, &[typed("NEEDLE")]);
    let (found, _) = step(&mut window, &[]);
    assert_eq!(found.match_count, 2);
    let _ = step(&mut window, &support::click(CASE_BUTTON.0, CASE_BUTTON.1));
    let (found, _) = step(&mut window, &[]);
    assert_eq!(found.match_count, 0, "with the case matched, NEEDLE is nowhere");
}
