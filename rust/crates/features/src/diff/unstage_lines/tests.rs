use super::unstage_lines;
use crate::diff::testing::{change, on, refusal, versions};
use crate::testing::{lines, plan};
use model::contract::{Plan, Step};
use model::diff::DiffSide;

/// The patch given to git on its standard input by the only step of a plan.
fn patch(plan: &Plan) -> String {
    match plan.steps.as_slice() {
        [Step::Git { input: Some(input), .. }] => String::from_utf8(input.clone()).unwrap(),
        other => panic!("one git step with a patch, not {other:?}"),
    }
}

/// The lines of a patch after its three lines of names.
fn body(patch: &str) -> Vec<&str> {
    patch.split_inclusive('\n').skip(3).collect()
}

#[test]
fn diff_unstage_lines__unstaging_the_first_two_of_five_lines_staged_leaves_the_other_three() {
    let head = "top\nend\n";
    let index = "top\na\nb\nc\nd\ne\nend\n";
    let selection = change((DiffSide::Head, DiffSide::Index), (1..1, &[]), (1..3, &["a", "b"]));
    let unstaged = plan(unstage_lines(
        &on(selection),
        &versions(Some(head), Some(index), Some(index)),
    ));
    assert_eq!(
        lines(&unstaged),
        ["git apply --cached --unidiff-zero --whitespace=nowarn -"]
    );
    assert_eq!(unstaged.title.as_str(), "Unstage 2 lines in src/app.rs");
    assert_eq!(body(&patch(&unstaged)), ["@@ -2,2 +1,0 @@\n", "-a\n", "-b\n"]);
}

#[test]
fn diff_unstage_lines__in_the_diff_of_the_working_tree_the_rows_of_a_staged_change_are_found_in_the_index() {
    let head = "one\ntwo\nthree\n";
    let index = "one\nTWO\nthree\n";
    let working_tree = "zero\none\nTWO\nthree\n";
    // The rows of `two` changed into `TWO`, which the working tree has one line lower.
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (2..3, &["TWO"]),
    );
    let unstaged = plan(unstage_lines(
        &on(selection),
        &versions(Some(head), Some(index), Some(working_tree)),
    ));
    assert_eq!(body(&patch(&unstaged)), ["@@ -2,1 +2,1 @@\n", "-TWO\n", "+two\n"]);
}

#[test]
fn diff_unstage_lines__all_the_lines_of_a_file_added_to_the_index_unstage_with_git_restore_staged() {
    let index = "a\nb\n";
    let selection = change((DiffSide::Head, DiffSide::Index), (0..0, &[]), (0..2, &["a", "b"]));
    let unstaged = plan(unstage_lines(&on(selection), &versions(None, Some(index), Some(index))));
    assert_eq!(lines(&unstaged), ["git restore --staged -- src/app.rs"]);
}

#[test]
fn diff_unstage_lines__rows_with_nothing_staged_are_refused() {
    let head = "one\ntwo\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let said = refusal(unstage_lines(
        &on(selection),
        &versions(Some(head), Some(head), Some("one\nTWO\n")),
    ));
    assert_eq!(said, "Nothing of these lines is staged.");
}
