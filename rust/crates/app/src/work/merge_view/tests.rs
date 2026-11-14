//! The regions of files in conflict computed from their texts, with the diffs of the program
//! (`merge-view.three-way`), and the line of the result a line of a side stands for (`merge-view.open-at-line`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

use model::merge_view::{Column, RegionKind, RegionState, result_line};
use views::merge_view::MergeText;

use super::{MergeSidesText, compute};

/// The texts of a file in conflict, each line ended by `\n`.
fn sides(base: &[&str], current: &[&str], incoming: &[&str]) -> MergeSidesText {
    MergeSidesText {
        base: text(base),
        current: text(current),
        incoming: text(incoming),
        operation: None,
    }
}

/// Lines, each ended by `\n`.
fn text(lines: &[&str]) -> String {
    lines.iter().flat_map(|line| [*line, "\n"]).collect()
}

/// The regions of a result against the sides, without proposals.
fn regions(sides: &MergeSidesText, result: &str) -> MergeText {
    compute(sides, result.to_owned(), (&super::Proposing::default(), &|| true)).unwrap()
}

/// What git writes for the conflict of `B` and `c` against `b` and `C` after `a`.
const MARKED: &str = "a\n<<<<<<< HEAD\nB\nc\n=======\nb\nC\n>>>>>>> feature\n";

#[test]
fn merge_view_three_way__a_change_of_the_current_side_alone_is_one_current_region_whose_result_is_that_side() {
    let file = sides(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "c"]);
    let computed = regions(&file, "a\nB\nc\n");
    assert_eq!(computed.regions.len(), 1);
    let region = &computed.regions[0];
    assert_eq!(region.kind, RegionKind::Current);
    assert_eq!(
        (region.base.clone(), region.current.clone(), region.result.clone()),
        (1..2, 1..2, 1..2)
    );
    assert_eq!(region.state, RegionState::Current);
}

#[test]
fn merge_view_three_way__changes_of_the_two_sides_that_touch_are_one_conflict() {
    let file = sides(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    let computed = regions(&file, MARKED);
    assert_eq!(computed.regions.len(), 1);
    let region = &computed.regions[0];
    assert_eq!(region.kind, RegionKind::Conflict);
    assert_eq!(region.base, 1..3);
}

#[test]
fn merge_view_three_way__a_marked_conflict_covers_its_block_until_it_is_replaced_and_then_is_edited() {
    let file = sides(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    let marked = regions(&file, MARKED);
    let region = &marked.regions[0];
    assert_eq!(region.result, 1..8);
    assert_eq!(region.state, RegionState::Unresolved);
    let block = region.marked.as_ref().unwrap();
    assert_eq!((block.current.clone(), block.incoming.clone()), (2..4, 5..7));
    let resolved = regions(&file, "a\nB\nC\n");
    assert_eq!(resolved.regions.len(), 1);
    assert_eq!(resolved.regions[0].result, 1..3);
    assert_eq!(resolved.regions[0].state, RegionState::Edited);
    assert!(!resolved.regions[0].is_unresolved());
}

#[test]
fn merge_view_three_way__lines_typed_above_a_region_move_it_down_by_as_many_lines() {
    let base = ["a", "b", "c", "d", "e"];
    let current = ["a", "b", "c", "D", "e"];
    let file = sides(&base, &current, &base);
    let before = regions(&file, &text(&current));
    assert_eq!(before.regions[0].result, 3..4);
    let typed = regions(&file, &format!("one\ntwo\n{}", text(&current)));
    assert_eq!(typed.regions.len(), 1);
    assert_eq!(typed.regions[0].result, 5..6);
    assert_eq!(typed.regions[0].state, RegionState::Current);
}

#[test]
fn merge_view_three_way__sides_with_crlf_give_the_same_regions_as_with_lf() {
    let file = sides(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    let with_lf = regions(&file, MARKED);
    let crlf = |text: &str| text.replace('\n', "\r\n");
    let file_crlf = MergeSidesText {
        base: crlf(&file.base),
        current: crlf(&file.current),
        incoming: crlf(&file.incoming),
        operation: None,
    };
    let with_crlf = regions(&file_crlf, &crlf(MARKED));
    assert_eq!(with_crlf.regions, with_lf.regions);
}

#[test]
fn merge_view_open_at_line__a_line_below_a_region_two_lines_longer_in_the_result_moves_down_by_two() {
    let base: Vec<String> = (1..=50).map(|line| format!("line {line}")).collect();
    let base: Vec<&str> = base.iter().map(String::as_str).collect();
    let mut incoming = base.clone();
    let _replaced: Vec<&str> = incoming.splice(9..10, ["line 10", "added 1", "added 2"]).collect();
    let file = sides(&base, &base, &incoming);
    let computed = regions(&file, &text(&incoming));
    assert_eq!(computed.regions.len(), 1);
    assert_eq!(result_line(&computed.regions, (Column::Current, 39)) + 1, 42);
}

#[test]
fn merge_view_open_at_line__a_line_of_the_incoming_side_of_a_marked_conflict_goes_after_the_separator() {
    let file = sides(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    let computed = regions(&file, MARKED);
    let separator = MARKED.lines().position(|line| line == "=======").unwrap();
    assert_eq!(result_line(&computed.regions, (Column::Incoming, 2)), separator + 1);
}
