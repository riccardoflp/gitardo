use super::stage_change;
use crate::diff::testing::{change, on, refusal, versions, versions_of};
use crate::testing::{lines, plan};
use model::contract::{Answers, CommandPreview, Danger, Outcome, Plan, Query, Step};
use model::diff::{DiffChange, DiffSide};

/// `HEAD` and the index of five lines.
const HEAD: &str = "one\ntwo\nthree\nfour\nfive\n";
/// The working tree with three of them changed: the first, the third and the fifth.
const CHANGED: &str = "ONE\ntwo\nTHREE\nfour\nFIVE\n";

/// The second of three changes of the working tree against `HEAD`.
fn second() -> DiffChange {
    change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (2..3, &["three"]),
        (2..3, &["THREE"]),
    )
}

/// The patch given to git on its standard input by the only step of a plan.
fn patch(plan: &Plan) -> String {
    match plan.steps.as_slice() {
        [Step::Git { input: Some(input), .. }] => String::from_utf8(input.clone()).unwrap(),
        other => panic!("one git step with a patch, not {other:?}"),
    }
}

#[test]
fn diff_stage_change__staging_the_second_of_three_changes_gives_git_apply_its_patch_alone() {
    let staged = plan(stage_change(
        &on(second()),
        &versions(Some(HEAD), Some(HEAD), Some(CHANGED)),
    ));
    assert_eq!(
        lines(&staged),
        ["git apply --cached --unidiff-zero --whitespace=nowarn -"]
    );
    assert_eq!(staged.title.as_str(), "Stage change in src/app.rs");
    assert_eq!(
        patch(&staged),
        "diff --git a/src/app.rs b/src/app.rs\n--- a/src/app.rs\n+++ b/src/app.rs\n@@ -3,1 +3,1 @@\n-three\n+THREE\n"
    );
}

#[test]
fn diff_stage_change__with_command_preview_all_the_plan_shown_has_the_command_and_the_patch() {
    // The plan is not dangerous, so the engine shows it first only when `commandPreview` is `all`
    // (`engine::needs_confirmation`); the dialog shows the patch under its command (`views::feature::plan_lines`).
    let mut context = on(second());
    context.settings.command_preview = CommandPreview::All;
    let staged = plan(stage_change(&context, &versions(Some(HEAD), Some(HEAD), Some(CHANGED))));
    assert_eq!(staged.danger, Danger::None);
    assert!(patch(&staged).contains("@@ -3,1 +3,1 @@\n-three\n+THREE\n"));
}

#[test]
fn diff_stage_change__a_file_edited_since_the_page_was_drawn_refuses_and_stages_nothing() {
    let edited = "ONE\ninserted\ntwo\nTHREE\nfour\nFIVE\n";
    let said = refusal(stage_change(
        &on(second()),
        &versions(Some(HEAD), Some(HEAD), Some(edited)),
    ));
    assert_eq!(said, "The file changed since the diff was shown. Try again.");
}

#[test]
fn diff_stage_change__an_untracked_file_is_added_whole() {
    let untracked = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..0, &[]),
        (0..2, &["a", "b"]),
    );
    let staged = plan(stage_change(&on(untracked), &versions(None, None, Some("a\nb\n"))));
    assert_eq!(lines(&staged), ["git add -- src/app.rs"]);
}

#[test]
fn reads_the_file_again_with_head_before_deciding() {
    let read = stage_change(&on(second()), &Answers::new());
    let Outcome::Read(_, Query::FileVersions { path, commit }) = read else {
        panic!("it reads the file first, not {read:?}");
    };
    assert_eq!(path, "src/app.rs");
    assert_eq!(commit, Some(crate::testing::commit('a')));
}

#[test]
fn refuses_a_change_staged_already_and_a_page_not_of_the_working_tree() {
    let said = refusal(stage_change(
        &on(second()),
        &versions(Some(HEAD), Some("one\ntwo\nTHREE\nfour\nfive\n"), Some(CHANGED)),
    ));
    assert_eq!(said, "This change is staged already.");
    let of_the_index = change((DiffSide::Head, DiffSide::Index), (1..2, &["two"]), (1..2, &["TWO"]));
    assert!(matches!(
        stage_change(&on(of_the_index), &Answers::new()),
        Outcome::Refuse(_)
    ));
}

#[test]
fn refuses_a_patch_of_a_file_not_in_utf8_with_characters_above_127() {
    let latin = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (1..2, &["two"]),
        (1..2, &["t\u{fffd}o"]),
    );
    let answers = versions_of(
        Some(HEAD.as_bytes()),
        Some(HEAD.as_bytes()),
        Some(b"one\nt\xe9o\nthree\n"),
    );
    assert_eq!(
        refusal(stage_change(&on(latin), &answers)),
        "Staging a change needs UTF-8 text: stage the whole file."
    );
}

#[test]
fn refuses_the_first_line_of_a_file_with_a_byte_order_mark() {
    let first = change(
        (DiffSide::Head, DiffSide::WorkingTree),
        (0..1, &["\u{feff}one"]),
        (0..1, &["\u{feff}ONE"]),
    );
    let head = "\u{feff}one\ntwo\n";
    assert_eq!(
        refusal(stage_change(
            &on(first),
            &versions(Some(head), Some(head), Some("\u{feff}ONE\ntwo\n"))
        )),
        "The first line of a file with a byte order mark cannot be staged as a change: stage the whole file."
    );
}
