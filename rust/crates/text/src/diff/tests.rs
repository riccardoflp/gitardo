#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::indexing_slicing, reason = "a test that reads past the end fails")]

use std::ops::Range;

use testkit::Random;

use super::{Change, ChangeKind, LineDiffInput, Span, WordDiffInput, line_diff, lines, word_diff};

/// The changes between two texts written with spaces between their lines, with no time limit.
fn diff_words(left: &str, right: &str) -> Vec<Change> {
    let left: Vec<&str> = left.split_whitespace().collect();
    let right: Vec<&str> = right.split_whitespace().collect();
    diff_lines(&left, &right)
}

/// The changes between two lists of lines, with no time limit.
fn diff_lines(left: &[&str], right: &[&str]) -> Vec<Change> {
    let always = || true;
    line_diff(&LineDiffInput {
        left,
        right,
        keep_going: &always,
    })
    .unwrap()
}

/// The marks of two lines, as the text they cover.
fn marked(left: &str, right: &str) -> (Vec<String>, Vec<String>) {
    let (left_marks, right_marks) = word_diff(&WordDiffInput { left, right });
    let texts = |line: &str, marks: Vec<Range<usize>>| marks.into_iter().map(|mark| line[mark].to_owned()).collect();
    (texts(left, left_marks), texts(right, right_marks))
}

#[test]
fn diff_line_diff__a_line_changed_between_two_is_one_changed_change_of_one_line_each() {
    assert_eq!(
        diff_words("a b c", "a B c"),
        vec![Change {
            kind: ChangeKind::Changed,
            left: Span { start: 2, count: 1 },
            right: Span { start: 2, count: 1 },
        }]
    );
}

#[test]
fn diff_line_diff__an_added_line_has_no_lines_on_the_left_at_the_line_before_it() {
    let changes = diff_words("a b", "a x b");
    assert_eq!(
        changes,
        vec![Change {
            kind: ChangeKind::Added,
            left: Span { start: 1, count: 0 },
            right: Span { start: 2, count: 1 },
        }]
    );
    assert_eq!(changes[0].left.lines(), 1..1);
    assert_eq!(changes[0].right.lines(), 1..2);
    let at_start = diff_words("b", "a b");
    assert_eq!(at_start[0].left, Span { start: 0, count: 0 });
}

#[test]
fn diff_line_diff__a_text_and_the_same_text_with_crlf_have_no_change() {
    let unix = "fn main() {\n    run();\n}\n";
    let windows = "fn main() {\r\n    run();\r\n}\r\n";
    assert_eq!(lines(windows), lines(unix));
    assert_eq!(diff_lines(&lines(unix), &lines(windows)), Vec::new());
}

#[test]
fn diff_line_diff__a_space_at_the_end_of_a_line_is_a_change() {
    let changes = diff_lines(&["a "], &["a"]);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Changed);
}

#[test]
fn diff_line_diff__on_random_texts_no_more_lines_change_than_the_edit_distance() {
    let mut random = Random::from_environment(1986);
    let small = ["a", "b", "c"];
    let large = ["a", "b", "c", "d", "e", "f", "g", "h"];
    for round in 0..600 {
        let alphabet: &[&str] = if round % 3 == 2 { &large } else { &small };
        let text = |random: &mut Random| -> Vec<&str> {
            let length = random.below(51);
            (0..length).map(|_| *random.pick(alphabet)).collect()
        };
        let left = text(&mut random);
        let right = text(&mut random);
        let changes = diff_lines(&left, &right);
        let lines_changed: usize = changes
            .iter()
            .map(|change| change.left.count + change.right.count)
            .sum();
        let distance = edit_distance(&left, &right);
        assert!(
            lines_changed <= distance,
            "seed {}: {lines_changed} lines changed, the edit distance is {distance}, for {left:?} and {right:?}",
            random.seed()
        );
        assert_eq!(apply(&left, &right, &changes), right, "seed {}", random.seed());
    }
}

#[test]
fn diff_line_diff__a_diff_told_to_stop_says_it_is_too_complex() {
    let never = || false;
    let result = line_diff(&LineDiffInput {
        left: &["a", "b"],
        right: &["b", "a"],
        keep_going: &never,
    });
    assert_eq!(result, Err(super::TooComplex));
}

#[test]
fn diff_line_diff__changes_are_never_next_to_each_other_and_texts_with_nothing_in_common_are_one_change() {
    let changes = diff_words("a b c", "x y");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].left, Span { start: 1, count: 3 });
    assert_eq!(changes[0].right, Span { start: 1, count: 2 });
    assert_eq!(diff_words("", ""), Vec::new());
    assert_eq!(diff_words("a", "")[0].kind, ChangeKind::Removed);
}

/// The fewest lines removed and added to turn one list into the other, by dynamic programming.
fn edit_distance(left: &[&str], right: &[&str]) -> usize {
    let mut row: Vec<usize> = (0..=right.len()).collect();
    for (index, line) in left.iter().enumerate() {
        let mut next = vec![index + 1; right.len() + 1];
        for (column, other) in right.iter().enumerate() {
            next[column + 1] = if line == other {
                row[column]
            } else {
                1 + row[column + 1].min(next[column])
            };
        }
        row = next;
    }
    row[right.len()]
}

/// The left lines with the changes applied, taking the added lines from the right.
fn apply<'a>(left: &[&'a str], right: &[&'a str], changes: &[Change]) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut at = 0;
    for change in changes {
        let removed = change.left.lines();
        result.extend_from_slice(&left[at..removed.start]);
        result.extend_from_slice(&right[change.right.lines()]);
        at = removed.end;
    }
    result.extend_from_slice(&left[at..]);
    result
}

#[test]
fn diff_word_diff__one_name_changed_marks_that_name_on_each_side_and_nothing_else() {
    assert_eq!(
        marked("let total = a + b;", "let total = a + c;"),
        (vec!["b".to_owned()], vec!["c".to_owned()])
    );
}

#[test]
fn diff_word_diff__a_word_changed_is_marked_whole_not_letter_by_letter() {
    assert_eq!(
        marked("foo(bar)", "foo(baz)"),
        (vec!["bar".to_owned()], vec!["baz".to_owned()])
    );
}

#[test]
fn diff_word_diff__spaces_added_at_the_start_are_marked_on_the_right_only() {
    assert_eq!(marked("call();", "    call();"), (vec![], vec!["    ".to_owned()]));
}

#[test]
fn diff_word_diff__two_lines_with_nothing_in_common_have_no_marks() {
    assert_eq!(marked("alpha beta", "x = 1;"), (vec![], vec![]));
    assert_eq!(marked("", "something"), (vec![], vec![]));
}

#[test]
fn diff_word_diff__marks_that_touch_are_one_and_letters_of_any_script_make_one_word() {
    assert_eq!(
        marked("città = vecchia", "città = nuova(1)"),
        (vec!["vecchia".to_owned()], vec!["nuova(1)".to_owned()])
    );
}
