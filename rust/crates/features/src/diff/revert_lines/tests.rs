use super::{revert_lines, reverted_lines};
use crate::diff::testing::{change, on, refusal, versions};
use crate::testing::plan;
use model::contract::Step;
use model::diff::DiffSide;

/// The text a plan writes in its only step.
fn written(plan: &model::contract::Plan) -> String {
    match plan.steps.as_slice() {
        [Step::Write { contents, .. }] => String::from_utf8(contents.clone()).unwrap(),
        other => panic!("one step that writes, not {other:?}"),
    }
}

#[test]
fn diff_revert_lines__reverting_the_row_of_the_second_of_three_lines_changed_gives_its_old_line_alone() {
    let head = "zero\none\ntwo\nthree\nfour\n";
    let working_tree = "zero\nONE\nTWO\nTHREE\nfour\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (2..3, &["two"]),
        (2..3, &["TWO"]),
    );
    let reverted = plan(revert_lines(
        &on(selection),
        &versions(Some(head), Some(head), Some(working_tree)),
    ));
    assert_eq!(reverted.title.as_str(), "Revert 1 line in src/app.rs");
    assert_eq!(written(&reverted), "zero\nONE\ntwo\nTHREE\nfour\n");
}

#[test]
fn diff_revert_lines__rows_across_two_changes_revert_what_is_selected_of_both_in_one_edit() {
    let head = "one\ntwo\nthree\nfour\nfive\n";
    let working_tree = "one\nTWO\nthree\nFOUR\nfive\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..4, &["two", "three", "four"]),
        (1..4, &["TWO", "three", "FOUR"]),
    );
    let reverted = reverted_lines(working_tree.as_bytes(), head.as_bytes(), &selection);
    assert_eq!(reverted.as_deref(), Some(head.as_bytes()));
}

#[test]
fn diff_revert_lines__reverting_the_rows_of_lines_removed_puts_them_back_after_the_line_before() {
    let head = "one\ntwo\nthree\nfour\n";
    let working_tree = "one\nfour\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..3, &["two", "three"]),
        (1..1, &[]),
    );
    let reverted = plan(revert_lines(
        &on(selection),
        &versions(Some(head), Some(head), Some(working_tree)),
    ));
    assert_eq!(written(&reverted), head);
}

#[test]
fn diff_revert_lines__against_a_commit_the_lines_reverted_are_those_of_the_commit() {
    let commit = model::CommitId::parse(&"1".repeat(40)).unwrap();
    let in_commit = "one\nold\nthree\n";
    let working_tree = "one\nnew\nthree\n";
    let selection = change(
        (DiffSide::Commit(commit), DiffSide::WorkingTree),
        (1..2, &["old"]),
        (1..2, &["new"]),
    );
    let reverted = plan(revert_lines(
        &on(selection),
        &versions(Some(in_commit), Some(working_tree), Some(working_tree)),
    ));
    assert_eq!(written(&reverted), in_commit);
}

#[test]
fn diff_revert_lines__a_file_edited_since_the_selection_refuses() {
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let said = refusal(revert_lines(
        &on(selection),
        &versions(Some("one\ntwo\n"), None, Some("one\nedited\n")),
    ));
    assert_eq!(said, "The file changed since the diff was shown. Try again.");
}
