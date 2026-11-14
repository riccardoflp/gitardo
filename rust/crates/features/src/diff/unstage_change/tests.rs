use super::unstage_change;
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

#[test]
fn diff_unstage_change__unstaging_one_of_two_staged_changes_takes_only_it_out_of_the_index() {
    // Both changes are staged: the index is the working tree. The first, at line 1, is unstaged.
    let head = "one\ntwo\nthree\n";
    let staged = "ONE\ntwo\nTHREE\n";
    let first = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..1, &["one"]),
        (0..1, &["ONE"]),
    );
    let unstaged = plan(unstage_change(
        &on(first),
        &versions(Some(head), Some(staged), Some(staged)),
    ));
    assert_eq!(
        lines(&unstaged),
        ["git apply --cached --unidiff-zero --whitespace=nowarn -"]
    );
    assert!(patch(&unstaged).ends_with("@@ -1,1 +1,1 @@\n-ONE\n+one\n"));
    assert_eq!(unstaged.title.as_str(), "Unstage change in src/app.rs");
}

#[test]
fn diff_unstage_change__in_the_diff_of_the_index_against_head_the_change_is_found_in_the_lines_of_the_index() {
    let head = "one\ntwo\n";
    let index = "zero\none\nTWO\n";
    let second = change((DiffSide::Head, DiffSide::Index), (1..2, &["two"]), (2..3, &["TWO"]));
    let unstaged = plan(unstage_change(&on(second), &versions(Some(head), Some(index), None)));
    assert!(patch(&unstaged).ends_with("@@ -3,1 +3,1 @@\n-TWO\n+two\n"));
}

#[test]
fn diff_unstage_change__a_newly_added_file_unstages_with_git_restore_staged() {
    let added = change((DiffSide::Head, DiffSide::Index), (0..0, &[]), (0..1, &["new"]));
    let unstaged = plan(unstage_change(
        &on(added),
        &versions(None, Some("new\n"), Some("new\n")),
    ));
    assert_eq!(lines(&unstaged), ["git restore --staged -- src/app.rs"]);
}

#[test]
fn refuses_a_change_with_nothing_staged() {
    let head = "one\n";
    let first = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..1, &["one"]),
        (0..1, &["ONE"]),
    );
    assert_eq!(
        refusal(unstage_change(
            &on(first),
            &versions(Some(head), Some(head), Some("ONE\n"))
        )),
        "Nothing of this change is staged."
    );
}
