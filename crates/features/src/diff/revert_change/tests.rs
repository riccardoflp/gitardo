use super::revert_change;
use crate::diff::testing::{change, on, refusal, versions};
use crate::testing::{lines, plan};
use model::contract::{Plan, Step};
use model::diff::DiffSide;

/// What the only step of a plan writes.
fn written(plan: &Plan) -> String {
    match plan.steps.as_slice() {
        [Step::Write { contents, .. }] => String::from_utf8(contents.clone()).unwrap(),
        other => panic!("one write, not {other:?}"),
    }
}

#[test]
fn diff_revert_change__reverting_a_changed_line_against_head_writes_the_line_head_has() {
    let head = "one\ntwo\nthree\n";
    let second = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    let reverted = plan(revert_change(
        &on(second),
        &versions(Some(head), Some(head), Some("ONE\nTWO\nthree\n")),
    ));
    assert_eq!(written(&reverted), "ONE\ntwo\nthree\n");
    let file = std::path::Path::new("repository").join("src/app.rs");
    assert_eq!(lines(&reverted), [format!("write {}", file.display())]);
    assert_eq!(reverted.title.as_str(), "Revert change in src/app.rs");
}

#[test]
fn diff_revert_change__a_removed_change_at_the_end_of_a_file_with_no_final_line_break_comes_back_after_the_last_line() {
    let head = "one\ntwo\nthree\n";
    let removed = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..3, &["two", "three"]),
        (1..1, &[]),
    );
    let reverted = plan(revert_change(
        &on(removed),
        &versions(Some(head), Some(head), Some("one")),
    ));
    assert_eq!(written(&reverted), "one\ntwo\nthree\n");
}

#[test]
fn the_lines_put_back_take_the_line_breaks_of_the_file_and_the_end_of_the_file_is_kept() {
    let changed = change((DiffSide::Index, DiffSide::WorkingTree), (0..1, &["a"]), (0..1, &["A"]));
    let reverted = plan(revert_change(
        &on(changed),
        &versions(None, Some("a\nb\nc"), Some("A\r\nb\r\nc")),
    ));
    assert_eq!(written(&reverted), "a\r\nb\r\nc");
}

#[test]
fn an_added_change_of_an_untracked_file_is_taken_out() {
    let added = change(
        (DiffSide::Empty, DiffSide::WorkingTree),
        (0..0, &[]),
        (0..2, &["a", "b"]),
    );
    let reverted = plan(revert_change(&on(added), &versions(None, None, Some("a\nb\n"))));
    assert_eq!(written(&reverted), "");
}

#[test]
fn refuses_when_the_file_changed_since_the_page_was_drawn() {
    let second = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["TWO"]),
    );
    assert_eq!(
        refusal(revert_change(
            &on(second),
            &versions(Some("one\ntwo\n"), None, Some("one\nTWO!\n"))
        )),
        "The file changed since the diff was shown. Try again."
    );
}
