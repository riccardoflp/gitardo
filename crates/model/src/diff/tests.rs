#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::indexing_slicing, reason = "a test that reads past the end fails")]

use std::collections::BTreeSet;
use std::ops::Range;

use super::{
    AlignInput, AlignedDiff, LineChange, LineChangeKind, ScrollInput, ShownRow, StagedInput, WordMarks, align,
    compare_paths, scroll_together, shown_rows, sorted_paths, staged_changes, unchanged_folds,
};

/// Marks the whole of both lines, so that a test sees which lines were paired.
fn mark_all(left: &str, right: &str) -> WordMarks {
    (
        std::iter::once(0..left.len()).collect(),
        std::iter::once(0..right.len()).collect(),
    )
}

/// The rows of two lists of lines with their changes.
fn aligned(left: &[&str], right: &[&str], changes: &[LineChange]) -> AlignedDiff {
    align(&AlignInput {
        left,
        right,
        changes,
        word_marks: &mark_all,
    })
}

/// A change by its lines on each side.
fn change(left: Range<usize>, right: Range<usize>) -> LineChange {
    LineChange { left, right }
}

#[test]
fn diff_align__two_lines_changed_into_five_give_two_paired_rows_then_three_facing_filler() {
    let left = ["a", "b", "c", "d"];
    let right = ["a", "B1", "B2", "B3", "B4", "B5", "d"];
    let diff = aligned(&left, &right, &[change(1..3, 1..6)]);
    assert_eq!(diff.rows.len(), 7);
    let rows = &diff.rows[1..6];
    assert!(rows.iter().all(|row| row.change == Some(0)));
    for row in &rows[..2] {
        assert_eq!(row.left.as_ref().unwrap().marks.len(), 1);
        assert_eq!(row.right.as_ref().unwrap().marks.len(), 1);
    }
    for row in &rows[2..] {
        assert_eq!(row.left, None);
        assert_eq!(row.right.as_ref().unwrap().marks, Vec::new());
    }
    assert_eq!(diff.changes[0].kind, LineChangeKind::Changed);
    assert_eq!(diff.changes[0].rows, 1..6);
}

#[test]
fn diff_align__four_lines_removed_face_filler_on_the_right_and_the_next_line_faces_itself() {
    let left = ["a", "x1", "x2", "x3", "x4", "b"];
    let right = ["a", "b"];
    let diff = aligned(&left, &right, &[change(1..5, 1..1)]);
    assert_eq!(diff.rows.len(), 6);
    assert!(
        diff.rows[1..5]
            .iter()
            .all(|row| row.right.is_none() && row.left.is_some())
    );
    let next = &diff.rows[5];
    assert_eq!(
        (next.left.as_ref().unwrap().line, next.right.as_ref().unwrap().line),
        (5, 1)
    );
    assert_eq!(next.change, None);
    assert_eq!(diff.changes[0].kind, LineChangeKind::Removed);
}

#[test]
fn diff_align__every_line_of_each_side_is_in_one_row_in_order_and_only_filler_has_no_number() {
    let left = ["a", "b", "c", "d", "e", "f"];
    let right = ["a", "x", "c", "y", "z", "f", "g"];
    let changes = [change(1..2, 1..2), change(3..5, 3..5), change(6..6, 6..7)];
    let diff = aligned(&left, &right, &changes);
    let left_lines: Vec<usize> = diff
        .rows
        .iter()
        .filter_map(|row| row.left.as_ref().map(|line| line.line))
        .collect();
    let right_lines: Vec<usize> = diff
        .rows
        .iter()
        .filter_map(|row| row.right.as_ref().map(|line| line.line))
        .collect();
    assert_eq!(left_lines, (0..left.len()).collect::<Vec<_>>());
    assert_eq!(right_lines, (0..right.len()).collect::<Vec<_>>());
    let filler = diff
        .rows
        .iter()
        .filter(|row| row.left.is_none() || row.right.is_none())
        .count();
    assert_eq!(filler, 1);
    assert_eq!(diff.changes[2].kind, LineChangeKind::Added);
}

/// The rows of a file of lines with one-line changes at some lines, counted from 1.
fn file_with_changes(length: usize, lines_changed: &[usize]) -> AlignedDiff {
    let lines: Vec<String> = (1..=length).map(|line| line.to_string()).collect();
    let left: Vec<&str> = lines.iter().map(String::as_str).collect();
    let changes: Vec<LineChange> = lines_changed
        .iter()
        .map(|line| change(line - 1..*line, line - 1..*line))
        .collect();
    aligned(&left, &left, &changes)
}

#[test]
fn diff_collapse_unchanged__one_change_in_100_lines_shows_lines_47_to_53_and_two_bars() {
    let diff = file_with_changes(100, &[50]);
    let folds = unchanged_folds(&diff);
    assert_eq!(folds, vec![0..46, 53..100]);
    assert_eq!((folds[0].len(), folds[1].len()), (46, 47));
    let shown = shown_rows(diff.rows.len(), &folds, &BTreeSet::new());
    let lines: Vec<usize> = shown
        .iter()
        .filter_map(|row| match row {
            ShownRow::Row(row) => Some(row + 1),
            ShownRow::Folded(_) => None,
        })
        .collect();
    assert_eq!(lines, (47..=53).collect::<Vec<_>>());
    assert_eq!(shown.len(), 9);
}

#[test]
fn diff_collapse_unchanged__nine_lines_between_two_changes_all_stay_in_sight() {
    let diff = file_with_changes(30, &[10, 20]);
    let folds = unchanged_folds(&diff);
    assert!(folds.iter().all(|fold| fold.end <= 6 || fold.start >= 23));
    assert_eq!(unchanged_folds(&file_with_changes(30, &[])), Vec::new());
}

#[test]
fn diff_collapse_unchanged__a_fold_unfolded_shows_its_rows_on_both_sides() {
    let diff = file_with_changes(100, &[50]);
    let folds = unchanged_folds(&diff);
    let unfolded = BTreeSet::from([0]);
    let shown = shown_rows(diff.rows.len(), &folds, &unfolded);
    assert_eq!(shown.first(), Some(&ShownRow::Row(0)));
    assert_eq!(shown.last(), Some(&ShownRow::Folded(53..100)));
    assert_eq!(shown.len(), 53 + 1);
}

/// Where the other side is scrolled to, for a side scrolled so far, with rows of 10 pixels in a view of 100.
fn other_top(changes: &[(Range<f32>, Range<f32>)], heights: (f32, f32), top: f32) -> f32 {
    scroll_together(&ScrollInput {
        changes,
        heights,
        top,
        view: 100.0,
    })
}

#[test]
fn diff_scroll_together__through_unchanged_lines_ten_lines_scroll_ten_lines() {
    let changes = [(500.0..520.0, 500.0..510.0)];
    let before = other_top(&changes, (2000.0, 1990.0), 100.0);
    let after = other_top(&changes, (2000.0, 1990.0), 200.0);
    assert!((after - before - 100.0).abs() < 0.01);
    let later = other_top(&changes, (2000.0, 1990.0), 900.0) - other_top(&changes, (2000.0, 1990.0), 800.0);
    assert!((later - 100.0).abs() < 0.01);
}

#[test]
fn diff_scroll_together__through_a_long_change_the_single_line_stays_in_the_middle() {
    let changes = [(1000.0..3000.0, 1000.0..1010.0)];
    let heights = (5000.0, 3010.0);
    for top in [960.0, 1200.0, 2000.0, 2800.0, 2940.0] {
        let other = other_top(&changes, heights, top);
        let line_on_screen = 1000.0 - other;
        assert!(
            (40.0..=60.0).contains(&line_on_screen),
            "scrolled to {top}, the line is {line_on_screen} pixels down the view"
        );
    }
    let past = other_top(&changes, heights, 3100.0);
    assert!(
        1010.0 - past < 0.0,
        "past the change the line has gone up out of the middle"
    );
}

#[test]
fn diff_scroll_together__the_end_of_one_side_brings_the_other_to_its_end() {
    let changes = [(1000.0..3000.0, 1000.0..1010.0), (4990.0..5000.0, 3000.0..3010.0)];
    assert!((other_top(&changes, (5000.0, 3010.0), 4900.0) - 2910.0).abs() < 0.01);
    assert!((other_top(&changes, (3010.0, 5000.0), 2910.0) - 4900.0).abs() < 0.01);
}

#[test]
fn diff_staged_changes__a_change_staged_is_staged_and_the_other_is_not() {
    let changes = [change(2..3, 2..3), change(10..10, 10..12)];
    let unstaged = [change(10..10, 10..12)];
    let input = StagedInput {
        changes: &changes,
        unstaged: Some(&unstaged),
    };
    assert_eq!(staged_changes(&input), vec![true, false]);
}

#[test]
fn diff_staged_changes__a_change_staged_in_part_is_not_staged() {
    let changes = [change(2..6, 2..6)];
    let unstaged = [change(4..5, 4..5)];
    let input = StagedInput {
        changes: &changes,
        unstaged: Some(&unstaged),
    };
    assert_eq!(staged_changes(&input), vec![false]);
    let insertion_before = [change(2..2, 2..3)];
    let touching = StagedInput {
        changes: &[change(2..2, 2..2)],
        unstaged: Some(&insertion_before),
    };
    assert_eq!(staged_changes(&touching), vec![false]);
}

#[test]
fn diff_staged_changes__an_untracked_file_has_no_change_staged() {
    let changes = [change(0..0, 0..10)];
    let input = StagedInput {
        changes: &changes,
        unstaged: None,
    };
    assert_eq!(staged_changes(&input), vec![false]);
}

#[test]
fn changed_files_go_without_regard_to_case_and_numbers_by_their_value() {
    assert_eq!(
        sorted_paths(["b10.rs", "B9.rs", "a.rs", "b9.rs"]),
        vec!["a.rs", "B9.rs", "b9.rs", "b10.rs"]
    );
    assert_eq!(compare_paths("src/item2", "src/item10"), std::cmp::Ordering::Less);
    assert_eq!(compare_paths("Zeta", "alpha"), std::cmp::Ordering::Greater);
}
