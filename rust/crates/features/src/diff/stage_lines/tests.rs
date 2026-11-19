use super::stage_lines;
use crate::diff::testing::{change, on, refusal, versions};
use crate::testing::{lines, plan};
use model::contract::{CommandPreview, Danger, Plan, Step};
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

/// A text of so many lines, `line 1` to `line N`.
fn numbered(count: usize) -> String {
    (1..=count)
        .map(|line| format!("line {line}\n"))
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn diff_stage_lines__the_second_and_third_of_five_lines_added_are_staged_alone_with_git_apply() {
    let head = numbered(4);
    let working_tree = "line 1\nline 2\none\ntwo\nthree\nfour\nfive\nline 3\nline 4\n";
    // Rows of `two` and `three`, which face filler rows on the left.
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (2..2, &[]),
        (3..5, &["two", "three"]),
    );
    let staged = plan(stage_lines(
        &on(selection),
        &versions(Some(&head), Some(&head), Some(working_tree)),
    ));
    assert_eq!(
        lines(&staged),
        ["git apply --cached --unidiff-zero --whitespace=nowarn -"]
    );
    assert_eq!(staged.title.as_str(), "Stage 2 lines in src/app.rs");
    assert_eq!(body(&patch(&staged)), ["@@ -2,0 +3,2 @@\n", "+two\n", "+three\n"]);
}

#[test]
fn diff_stage_lines__rows_across_three_changes_with_the_second_staged_stage_the_first_and_the_third() {
    let head = "one\ntwo\nthree\nfour\nfive\n";
    let index = "one\ntwo\nTHREE\nfour\nfive\n";
    let working_tree = "ONE\ntwo\nTHREE\nfour\nFIVE\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..5, &["one", "two", "three", "four", "five"]),
        (0..5, &["ONE", "two", "THREE", "four", "FIVE"]),
    );
    let staged = plan(stage_lines(
        &on(selection),
        &versions(Some(head), Some(index), Some(working_tree)),
    ));
    assert_eq!(
        body(&patch(&staged)),
        [
            "@@ -1,1 +1,1 @@\n",
            "-one\n",
            "+ONE\n",
            "@@ -5,1 +5,1 @@\n",
            "-five\n",
            "+FIVE\n"
        ]
    );
}

#[test]
fn diff_stage_lines__the_row_of_a_line_changed_into_another_stages_the_new_line_in_place_of_the_old() {
    let head = "one\ntwo\nthree\n";
    let working_tree = "one\nTWO\nthree\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let staged = plan(stage_lines(
        &on(selection),
        &versions(Some(head), Some(head), Some(working_tree)),
    ));
    assert_eq!(body(&patch(&staged)), ["@@ -2,1 +2,1 @@\n", "-two\n", "+TWO\n"]);
}

#[test]
fn diff_stage_lines__two_lines_of_an_untracked_file_create_it_in_the_index_with_them() {
    let working_tree = "one\ntwo\nthree\nfour\nfive\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..0, &[]),
        (1..3, &["two", "three"]),
    );
    let staged = plan(stage_lines(&on(selection), &versions(None, None, Some(working_tree))));
    assert_eq!(
        patch(&staged),
        "diff --git a/src/app.rs b/src/app.rs\nnew file mode 100644\n--- /dev/null\n+++ b/src/app.rs\n@@ -0,0 +1,2 @@\n+two\n+three\n"
    );
}

#[test]
fn diff_stage_lines__lines_staged_take_the_crlf_of_the_index() {
    let index = "one\r\ntwo\r\n";
    let working_tree = "one\r\ntwo\r\nthree\r\n";
    let selection = change(
        (DiffSide::Index, DiffSide::WorkingTree),
        (2..2, &[]),
        (2..3, &["three"]),
    );
    let staged = plan(stage_lines(
        &on(selection),
        &versions(Some(index), Some(index), Some(working_tree)),
    ));
    assert_eq!(body(&patch(&staged)), ["@@ -2,0 +3,1 @@\n", "+three\r\n"]);
}

#[test]
fn diff_stage_lines__a_file_edited_since_the_selection_refuses_and_stages_nothing() {
    let head = "one\ntwo\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let said = refusal(stage_lines(
        &on(selection),
        &versions(Some(head), Some(head), Some("one\nedited\nTWO\n")),
    ));
    assert_eq!(said, "The file changed since the diff was shown. Try again.");
}

#[test]
fn diff_stage_lines__with_command_preview_all_the_plan_shown_has_the_command_and_the_patch() {
    let head = "one\ntwo\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let mut context = on(selection);
    context.settings.command_preview = CommandPreview::All;
    let staged = plan(stage_lines(
        &context,
        &versions(Some(head), Some(head), Some("one\nTWO\n")),
    ));
    assert_eq!(staged.danger, Danger::None);
    assert!(patch(&staged).contains("@@ -2,1 +2,1 @@\n-two\n+TWO\n"));
}

#[test]
fn diff_stage_lines__rows_with_nothing_left_to_stage_are_refused() {
    let head = "one\ntwo\n";
    let working_tree = "one\nTWO\n";
    let selection = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let said = refusal(stage_lines(
        &on(selection),
        &versions(Some(head), Some(working_tree), Some(working_tree)),
    ));
    assert_eq!(said, "These lines are staged already.");
}
