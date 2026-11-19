#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::ops::Range;

use super::{LinesPatchInput, lines_patch};
use crate::diff::PatchDirection;

/// A text of so many lines, `line 1` to `line N`.
fn numbered(count: usize) -> String {
    let lines: Vec<String> = (1..=count).map(|line| format!("line {line}\n")).collect();
    lines.concat()
}

/// The patch that stages the lines of some rows, given by their lines in the index and in the working tree.
fn staging(
    index: &str,
    working_tree: &str,
    (index_lines, other_lines): (Range<usize>, Range<usize>),
) -> Option<String> {
    lines_patch(&LinesPatchInput {
        path: "src/app.rs",
        index,
        other: working_tree,
        direction: PatchDirection::Stage,
        index_lines,
        other_lines,
        new_file: None,
    })
}

/// The lines of a patch after its three lines of names.
fn body(patch: &str) -> Vec<&str> {
    patch.split_inclusive('\n').skip(3).collect()
}

/// The text with lines put in after a line, counted from 1.
fn inserted(text: &str, after: usize, lines: &[&str]) -> String {
    let mut all: Vec<String> = text.lines().map(|line| format!("{line}\n")).collect();
    for (offset, line) in lines.iter().enumerate() {
        all.insert(after + offset, format!("{line}\n"));
    }
    all.concat()
}

#[test]
fn diff_lines_patch__the_second_and_third_of_five_lines_added_after_line_12_give_their_hunk() {
    let index = numbered(20);
    let working_tree = inserted(&index, 12, &["one", "two", "three", "four", "five"]);
    let patch = staging(&index, &working_tree, (12..12, 13..15)).unwrap();
    assert_eq!(body(&patch), ["@@ -12,0 +13,2 @@\n", "+two\n", "+three\n"]);
}

#[test]
fn diff_lines_patch__the_row_of_line_5_of_a_change_of_lines_4_to_6_gives_its_old_line_and_its_new_one() {
    let index = numbered(10);
    let working_tree = index
        .replace("line 4\n", "new 4\n")
        .replace("line 5\n", "new 5\n")
        .replace("line 6\n", "new 6\n");
    let patch = staging(&index, &working_tree, (4..5, 4..5)).unwrap();
    assert_eq!(body(&patch), ["@@ -5,1 +5,1 @@\n", "-line 5\n", "+new 5\n"]);
}

#[test]
fn diff_lines_patch__rows_across_two_changes_give_two_hunks_the_second_where_the_first_left_the_index() {
    let index = numbered(10);
    let working_tree = inserted(&inserted(&index, 2, &["a", "b"]), 8, &["c"]);
    // Rows from the second line put in after line 2 to the one put in after line 6 of the index.
    let patch = staging(&index, &working_tree, (2..6, 3..9)).unwrap();
    assert_eq!(body(&patch), ["@@ -2,0 +3,1 @@\n", "+b\n", "@@ -6,0 +8,1 @@\n", "+c\n"]);
}

#[test]
fn diff_lines_patch__the_first_and_third_of_three_lines_removed_give_two_hunks_and_the_second_stays() {
    let index = numbered(10);
    let working_tree = index.replace("line 4\nline 5\nline 6\n", "");
    let first = staging(&index, &working_tree, (3..4, 3..3)).unwrap();
    assert_eq!(body(&first), ["@@ -4,1 +3,0 @@\n", "-line 4\n"]);
    let both = lines_patch(&LinesPatchInput {
        path: "src/app.rs",
        index: &index,
        other: &working_tree,
        direction: PatchDirection::Stage,
        index_lines: 3..6,
        other_lines: 3..3,
        new_file: None,
    })
    .unwrap();
    assert_eq!(
        body(&both),
        ["@@ -4,3 +3,0 @@\n", "-line 4\n", "-line 5\n", "-line 6\n"]
    );
    // The first and the third alone: two selections, two patches applied one after the other in the page; as one, the
    // rows of the second are not selected.
    let third = staging(&index, &working_tree, (5..6, 3..3)).unwrap();
    assert_eq!(body(&third), ["@@ -6,1 +5,0 @@\n", "-line 6\n"]);
}

#[test]
fn diff_lines_patch__lines_added_after_a_last_line_with_no_line_break_take_its_change_too() {
    let index = "one\ntwo";
    let working_tree = "one\ntwo\nthree\n";
    let patch = staging(index, working_tree, (2..2, 2..3)).unwrap();
    assert_eq!(
        body(&patch),
        [
            "@@ -2,1 +2,2 @@\n",
            "-two\n",
            "\\ No newline at end of file\n",
            "+two\n",
            "+three\n"
        ]
    );
}

#[test]
fn diff_lines_patch__two_of_the_five_lines_of_an_untracked_file_create_it_with_those_two() {
    let working_tree = "one\ntwo\nthree\nfour\nfive\n";
    let patch = lines_patch(&LinesPatchInput {
        path: "notes.txt",
        index: "",
        other: working_tree,
        direction: PatchDirection::Stage,
        index_lines: 0..0,
        other_lines: 1..3,
        new_file: Some("100644"),
    })
    .unwrap();
    assert_eq!(
        patch,
        "diff --git a/notes.txt b/notes.txt\nnew file mode 100644\n--- /dev/null\n+++ b/notes.txt\n@@ -0,0 +1,2 @@\n+two\n+three\n"
    );
}

#[test]
fn diff_lines_patch__rows_with_no_changed_line_give_none() {
    let index = numbered(10);
    let working_tree = inserted(&index, 8, &["late"]);
    assert_eq!(staging(&index, &working_tree, (1..4, 1..4)), None);
}

#[test]
fn diff_lines_patch__lines_staged_take_the_crlf_of_the_index() {
    let index = "one\r\ntwo\r\n";
    let working_tree = "one\ntwo\nthree\n";
    let patch = staging(index, working_tree, (2..2, 2..3)).unwrap();
    assert_eq!(body(&patch), ["@@ -2,0 +3,1 @@\n", "+three\r\n"]);
}
