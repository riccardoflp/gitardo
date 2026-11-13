//! The cards of the review: what they show of a proposal and its warnings, the sides, the columns, the files deleted
//! on one side, the result before it is written, and the write with its report.

use app::store::{Message, State};
use model::review::{Origin, ProposalWarning};
use platform::accesskit::Role;
use testkit::Repository;
use views::Intent;
use views::page::Page;
use views::review::{Filter, Layout, ReviewIntent};

use super::compare::until_ended;
use super::review::{NEXT_TO_EACH_OTHER, SAME_LINE, cards, drawn, file, key, merged, review, reviewed};
use super::support::{OperationsDriver, has, says, says_in};

/// A file whose rules proposal keeps `x` set twice, beside a file the rules resolve without a warning.
fn warned() -> Repository {
    merged(&[
        file("settings.txt", ("x = 1\nx = 2\n", "x = 3\nx = 2\n", "x = 1\nx = 4\n")),
        file("two.txt", NEXT_TO_EACH_OTHER),
    ])
}

#[test]
fn review_proposal_warnings__accept_all_and_accept_the_file_leave_out_a_proposal_with_a_warning() {
    let repository = warned();
    let mut state = reviewed(&repository);
    let card = cards(&state)
        .into_iter()
        .find(|card| card.key.path == "settings.txt")
        .unwrap();
    let warnings = card.proposal.unwrap().warnings;
    assert!(
        matches!(warnings.as_slice(), [ProposalWarning::SetTwice { name, .. }] if name == "x"),
        "{warnings:?}"
    );
    let head = "1 proposal has a warning: look at it and accept one by one.";
    assert!(says(&drawn(&state), head));
    review(&mut state, ReviewIntent::AcceptAll);
    review(&mut state, ReviewIntent::AcceptFile(String::from("settings.txt")));
    let accepted: Vec<(String, bool)> = cards(&state)
        .into_iter()
        .map(|card| (card.key.path, card.is_accepted))
        .collect();
    assert_eq!(
        accepted,
        [(String::from("settings.txt"), false), (String::from("two.txt"), true)]
    );
    assert!(says(&drawn(&state), head));
    let key = key(&state, "settings.txt", 0);
    review(&mut state, ReviewIntent::Accept(key));
    assert!(!says(&drawn(&state), head));
}

#[test]
fn review_accept_file__a_proposal_with_a_warning_is_left_out() {
    let repository = merged(&[file(
        "settings.txt",
        ("x = 1\nx = 2\n", "x = 3\nx = 2\n", "x = 1\nx = 4\n"),
    )]);
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptFile(String::from("settings.txt")));
    assert!(cards(&state).iter().all(|card| !card.is_accepted));
    assert!(!has(&drawn(&state), Role::Button, "Accept the file"));
}

#[test]
fn review_card__lines_are_numbered_as_the_file_would_number_them_with_the_proposal_in_place() {
    let base = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nb\nc\nd\ne\nf\n";
    let main = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nB\nc\nd\ne\nf\n";
    let feature = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nb\nC\nd\ne\nf\n";
    let repository = merged(&[file("one.txt", (base, main, feature))]);
    let state = reviewed(&repository);
    let card = cards(&state).remove(0);
    assert_eq!(card.line, 11);
    let proposal = card.proposal.unwrap();
    let numbers: Vec<usize> = proposal.lines.iter().map(|line| line.line.number).collect();
    assert_eq!(numbers, [11, 12]);
    let before: Vec<usize> = card.before.iter().map(|line| line.number).collect();
    let after: Vec<(usize, &str)> = card
        .after
        .iter()
        .map(|line| (line.number, line.text.as_str()))
        .collect();
    assert_eq!(before, [8, 9, 10]);
    assert_eq!(after, [(13, "d"), (14, "e"), (15, "f")]);
}

#[test]
fn review_card__compare_with_the_sides_names_them_with_their_subjects() {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the map", &[("map.txt", "a\nb\nc\n")]);
    repository.branch("feature/map-view");
    let _main = repository.commit("Name the map on main", &[("map.txt", "a\nB1\nc\n")]);
    repository.checkout("feature/map-view");
    let _feature = repository.commit("Name the map in the view", &[("map.txt", "a\nB2\nc\n")]);
    repository.checkout("main");
    let _status = repository
        .command("git")
        .args(["merge", "--no-edit", "feature/map-view"])
        .output()
        .unwrap();
    let state = reviewed(&repository);
    let tree = drawn(&state);
    assert!(says(&tree, "Current \u{b7} main \u{2014} Name the map on main"));
    assert!(says(
        &tree,
        "Incoming \u{b7} feature/map-view \u{2014} Name the map in the view"
    ));
    let card = cards(&state).remove(0);
    let added = card
        .side_diffs
        .0
        .rows
        .iter()
        .any(|row| matches!(row, model::review::SideRow::Added { text, .. } if text == "B1"));
    assert!(added, "{:?}", card.side_diffs.0.rows);
}

#[test]
fn review_card__accept_folds_the_card_to_the_first_line_of_its_resolution() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    let key = key(&state, "one.txt", 0);
    review(&mut state, ReviewIntent::Accept(key));
    review(&mut state, ReviewIntent::FoldFile(String::from("one.txt"), false));
    let card = cards(&state).remove(0);
    assert!(card.shown.is_folded);
    let first = card.proposal.unwrap().lines.first().unwrap().line.text.clone();
    let tree = drawn(&state);
    assert!(says_in(&tree, &format!("{}  \u{b7} Rules", first.trim())));
    assert!(!says(&tree, "Conflict at line 2"));
}

#[test]
fn review_columns__bands_join_each_line_proposed_to_the_side_it_comes_from() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let state = reviewed(&repository);
    let card = cards(&state).remove(0);
    let proposal = card.proposal.unwrap();
    let texts: Vec<&str> = proposal.lines.iter().map(|line| line.line.text.as_str()).collect();
    let place = |text: &str| texts.iter().position(|line| *line == text).unwrap();
    let current = |index: Option<usize>| {
        index
            .and_then(|index| card.current.lines.get(index))
            .map(|line| line.text.as_str())
    };
    let incoming = |index: Option<usize>| {
        index
            .and_then(|index| card.incoming.lines.get(index))
            .map(|line| line.text.as_str())
    };
    assert_eq!(
        current(proposal.from_current.get(place("B")).copied().flatten()),
        Some("B")
    );
    assert_eq!(
        incoming(proposal.to_incoming.get(place("C")).copied().flatten()),
        Some("C")
    );
    let joined_current: Vec<&str> = proposal
        .from_current
        .iter()
        .map(|index| current(*index).unwrap_or_default())
        .collect();
    let joined_incoming: Vec<&str> = proposal
        .to_incoming
        .iter()
        .map(|index| incoming(*index).unwrap_or_default())
        .collect();
    assert!(!joined_current.contains(&"c"), "the current side's c is faded");
    assert!(!joined_incoming.contains(&"b"), "the incoming side's b is faded");
    let origins: Vec<Origin> = proposal.lines.iter().map(|line| line.origin).collect();
    assert!(origins.contains(&Origin::Current) && origins.contains(&Origin::Incoming));
}

#[test]
fn review_columns__the_widths_are_the_same_for_every_conflict_and_kept_when_the_page_opens_again() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::Layout(Layout::Columns));
    review(&mut state, ReviewIntent::Widths(Some((400, 250))));
    let _effects = super::support::run(&mut state, Message::Intent(Intent::ClosePage));
    assert_eq!(state.reviews.page, None);
    super::review::open(&mut state);
    let page = app::store::review::page_state(&state).unwrap();
    assert_eq!((page.layout, page.widths), (Layout::Columns, (400, 250)));
}

#[test]
fn review_show_result__one_diff_page_shows_every_file_beside_its_result() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    review(&mut state, ReviewIntent::ShowResult);
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    let tree = drawn(&state);
    assert!(says(&tree, "Conflicts resolved as accepted"));
    assert!(says_in(&tree, "one.txt") && says_in(&tree, "two.txt"));
}

#[test]
fn review_show_result__no_file_on_disk_changes() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let before = std::fs::read(repository.path().join("one.txt")).unwrap();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    review(&mut state, ReviewIntent::ShowResult);
    assert_eq!(std::fs::read(repository.path().join("one.txt")).unwrap(), before);
}

/// A file deleted on `feature` and changed on `main`, beside a file the rules resolve.
fn deleted() -> Repository {
    merged(&[
        ("gone.txt", "one\ntwo\nthree\n", Some("one\nTWO\nthree\n"), None),
        file("one.txt", NEXT_TO_EACH_OTHER),
    ])
}

#[test]
fn review_deleted_card__says_which_side_deleted_the_file_and_shows_the_changes_of_the_other() {
    let repository = deleted();
    let state = reviewed(&repository);
    let tree = drawn(&state);
    assert!(says(
        &tree,
        "The branch being merged deleted it, and the branch checked out changed it."
    ));
    assert!(says(&tree, "gone.txt"));
}

#[test]
fn review_deleted_card__is_not_shown_under_from_the_ai() {
    let repository = deleted();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::Filter(Filter::FromTheAi));
    let page = app::store::review::page_state(&state).unwrap();
    let file = page.deleted.first().unwrap();
    assert!(!page.is_deleted_shown(file));
    review(&mut state, ReviewIntent::Filter(Filter::NothingProposed));
    let page = app::store::review::page_state(&state).unwrap();
    assert!(page.is_deleted_shown(page.deleted.first().unwrap()));
}

/// Writes what is accepted from the page, confirming what is asked.
fn written(state: &mut State) {
    let driver = OperationsDriver::new();
    driver.intent(state, Intent::Review(ReviewIntent::Write));
    until_ended(&driver, state);
}

#[test]
fn review_write__the_page_writes_what_was_accepted_and_reports_it() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    written(&mut state);
    let tree = drawn(&state);
    assert!(
        says(&tree, "Wrote the resolution of 1 conflict in 1 file."),
        "{:?}",
        state.operations.said
    );
    assert!(says(&tree, "1 file was left with no conflict, and staged."));
    assert!(says(&tree, "1 file is still in conflict."));
    assert!(!has(&tree, Role::Button, "Continue the merge"));
    let on_disk = std::fs::read_to_string(repository.path().join("one.txt")).unwrap();
    assert!(!on_disk.contains("<<<<<<<"), "{on_disk}");
}
