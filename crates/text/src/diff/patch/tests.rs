#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{ChangeLines, ChangePatchInput, PatchDirection, change_patch};

/// A text of so many lines, `line 1` to `line N`, each ended by `ending`.
fn numbered(count: usize, ending: &str) -> String {
    let lines: Vec<String> = (1..=count).map(|line| format!("line {line}{ending}")).collect();
    lines.concat()
}

/// The text with lines put in after a line, counted from 1.
fn inserted(text: &str, after: usize, lines: &[&str]) -> String {
    let mut all: Vec<String> = text.lines().map(|line| format!("{line}\n")).collect();
    for (offset, line) in lines.iter().enumerate() {
        all.insert(after + offset, format!("{line}\n"));
    }
    all.concat()
}

/// The patch that stages the change at some lines of the working tree.
fn staging(index: &str, working_tree: &str, lines: std::ops::Range<usize>) -> Option<String> {
    change_patch(&ChangePatchInput {
        path: "src/app.rs",
        index,
        other: working_tree,
        direction: PatchDirection::Stage,
        change: ChangeLines::Other(lines),
    })
}

/// The lines of a patch after its three lines of names.
fn body(patch: &str) -> Vec<&str> {
    patch.split_inclusive('\n').skip(3).collect()
}

#[test]
fn diff_change_patch__staging_a_two_line_insertion_after_line_12_gives_its_header_and_the_two_lines() {
    let index = numbered(20, "\n");
    let working_tree = inserted(&index, 12, &["first new", "second new"]);
    let patch = staging(&index, &working_tree, 12..14).unwrap();
    assert!(patch.starts_with("diff --git a/src/app.rs b/src/app.rs\n--- a/src/app.rs\n+++ b/src/app.rs\n"));
    assert_eq!(body(&patch), ["@@ -12,0 +13,2 @@\n", "+first new\n", "+second new\n"]);
}

#[test]
fn diff_change_patch__with_an_earlier_change_staged_the_old_start_is_that_of_the_index() {
    // Two lines were put in after line 3 and staged; the working tree also has a line put in after line 10 of `HEAD`,
    // which is line 12 of the working tree and of the index.
    let head = numbered(20, "\n");
    let index = inserted(&head, 3, &["staged one", "staged two"]);
    let working_tree = inserted(&index, 12, &["later"]);
    let patch = staging(&index, &working_tree, 12..13).unwrap();
    assert_eq!(body(&patch), ["@@ -12,0 +13,1 @@\n", "+later\n"]);
}

#[test]
fn diff_change_patch__a_change_at_the_last_line_of_a_file_with_no_final_line_break_carries_the_mark() {
    let index = "one\ntwo\nthree";
    let working_tree = "one\ntwo\nTHREE";
    let patch = staging(index, working_tree, 2..3).unwrap();
    assert_eq!(
        body(&patch),
        [
            "@@ -3,1 +3,1 @@\n",
            "-three\n",
            "\\ No newline at end of file\n",
            "+THREE\n",
            "\\ No newline at end of file\n"
        ]
    );
}

#[test]
fn diff_change_patch__staging_into_an_index_stored_with_crlf_writes_crlf_in_the_added_lines() {
    let index = numbered(4, "\r\n");
    let working_tree = "line 1\nline 2 changed\nline 3\nline 4\n";
    let patch = staging(&index, working_tree, 1..2).unwrap();
    assert_eq!(
        body(&patch),
        ["@@ -2,1 +2,1 @@\n", "-line 2\r\n", "+line 2 changed\r\n"]
    );
}

#[test]
fn a_change_staged_already_has_no_patch() {
    let index = numbered(5, "\n");
    assert_eq!(staging(&index, &index, 2..3), None);
}

#[test]
fn only_the_differences_that_touch_the_change_are_in_the_patch_and_later_ones_count_the_earlier() {
    let index = numbered(10, "\n");
    let working_tree =
        "line 1\nline 2 changed\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8 changed\nline 9\nline 10\n";
    let patch = staging(&index, working_tree, 7..8).unwrap();
    assert_eq!(body(&patch), ["@@ -8,1 +8,1 @@\n", "-line 8\n", "+line 8 changed\n"]);
}

#[test]
fn a_line_added_after_a_last_line_with_no_break_changes_that_line_too() {
    let patch = staging("one\ntwo", "one\ntwo\nthree\n", 2..3).unwrap();
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
fn unstaging_goes_from_the_index_towards_head_keeping_the_line_endings_of_head() {
    let head = "a\r\nb\r\nc\r\n";
    let index = "a\nB\nc\n";
    let patch = change_patch(&ChangePatchInput {
        path: "notes.txt",
        index,
        other: head,
        direction: PatchDirection::Unstage,
        change: ChangeLines::Index(1..2),
    })
    .unwrap();
    assert_eq!(body(&patch), ["@@ -2,1 +2,1 @@\n", "-B\n", "+b\r\n"]);
    let in_head = change_patch(&ChangePatchInput {
        path: "notes.txt",
        index,
        other: head,
        direction: PatchDirection::Unstage,
        change: ChangeLines::Other(1..2),
    });
    assert_eq!(in_head, Some(patch));
}

#[test]
fn a_removal_lands_after_the_line_before_it() {
    let index = numbered(6, "\n");
    let working_tree = "line 1\nline 2\nline 5\nline 6\n";
    let patch = staging(&index, working_tree, 2..2).unwrap();
    assert_eq!(body(&patch), ["@@ -3,2 +2,0 @@\n", "-line 3\n", "-line 4\n"]);
}

#[test]
fn a_path_with_a_quote_is_written_in_quotes() {
    let patch = change_patch(&ChangePatchInput {
        path: "say \"hi\".txt",
        index: "",
        other: "hello\n",
        direction: PatchDirection::Stage,
        change: ChangeLines::Other(0..1),
    })
    .unwrap();
    assert!(patch.starts_with("diff --git \"a/say \\\"hi\\\".txt\" \"b/say \\\"hi\\\".txt\"\n"));
    assert_eq!(body(&patch), ["@@ -0,0 +1,1 @@\n", "+hello\n"]);
}
