#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{
    Answer, Answers, Danger, FollowUp, Outcome, Query, QuestionKind, Step, Target, Undo, UndoRecord,
};
use model::patch::{PatchCheck, PatchInput};

use super::{CHECK, CLIPBOARD, CONTENTS, FILE, FROM, TEMPORARY, WAY, apply};
use crate::patch::testing::format_patch;
use crate::testing::{commit, context, lines, modified, plan, question, state};

/// A patch of `git diff`, which has no commits.
const DIFF: &str = "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-a\n+b\n";

/// The answers of a patch chosen in a file, read, and applied one way, checked to apply with some conflicts.
fn from_file(text: &str, way: Option<&str>, conflicts: Option<&[&str]>) -> Answers {
    let mut answers = Answers::new()
        .with(FROM, Answer::Choice(String::from("file")))
        .with(FILE, Answer::Path(PathBuf::from("two.patch")))
        .with(CONTENTS, Answer::Contents(text.as_bytes().to_vec()));
    if let Some(way) = way {
        answers = answers.with(WAY, Answer::Choice(way.to_owned()));
    }
    if let Some(conflicts) = conflicts {
        let conflicts = conflicts.iter().map(|&file| file.to_owned()).collect();
        answers = answers.with(CHECK, Answer::PatchCheck(PatchCheck::Applies { conflicts }));
    }
    answers
}

/// A patch of two commits made by `git format-patch`.
fn two_commits() -> String {
    format_patch(&[&commit('1'), &commit('2')])
}

#[test]
fn patch_apply__a_patch_of_two_commits_from_a_file_as_commits_runs_git_am() {
    let context = context(state(), Target::Head);
    assert!(matches!(
        question(apply(&context, &Answers::new())).kind,
        QuestionKind::Pick { .. }
    ));
    let file = Answers::new().with(FROM, Answer::Choice(String::from("file")));
    let asked = question(apply(&context, &file));
    assert!(matches!(asked.kind, QuestionKind::OpenFile(_)), "{asked:?}");
    let chosen = file.with(FILE, Answer::Path(PathBuf::from("two.patch")));
    assert_eq!(
        apply(&context, &chosen),
        Outcome::Read(
            CONTENTS,
            Query::FileContents {
                path: PathBuf::from("two.patch")
            }
        )
    );
    let way = question(apply(&context, &from_file(&two_commits(), None, None)));
    assert_eq!(way.title.as_str(), "Apply two.patch");
    assert_eq!(
        apply(&context, &from_file(&two_commits(), Some("commits"), None)),
        Outcome::Read(
            CHECK,
            Query::PatchCheck {
                patch: PatchInput::File(PathBuf::from("two.patch"))
            }
        )
    );
    let plan = plan(apply(&context, &from_file(&two_commits(), Some("commits"), Some(&[]))));
    assert_eq!(lines(&plan), ["git am --3way two.patch"]);
    assert_eq!(plan.title.as_str(), "Apply two.patch as 2 commits");
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn patch_apply__the_same_patch_to_the_working_tree_only_runs_git_apply() {
    let context = context(state(), Target::Head);
    let plan = plan(apply(
        &context,
        &from_file(&two_commits(), Some("working-tree"), Some(&[])),
    ));
    assert_eq!(lines(&plan), ["git apply --3way two.patch"]);
    assert_eq!(plan.title.as_str(), "Apply two.patch to the working tree");
    assert!(matches!(plan.undo, Undo::Impossible(_)));
}

#[test]
fn patch_apply__a_patch_of_git_diff_is_applied_to_the_working_tree_without_the_question() {
    let context = context(state(), Target::Head);
    assert!(matches!(
        apply(&context, &from_file(DIFF, None, None)),
        Outcome::Read(CHECK, _)
    ));
    let plan = plan(apply(&context, &from_file(DIFF, None, Some(&[]))));
    assert_eq!(lines(&plan), ["git apply --3way two.patch"]);
}

#[test]
fn patch_apply__from_the_clipboard_writes_a_temporary_file_runs_git_on_it_then_deletes_it() {
    let context = context(state(), Target::Head);
    let clipboard = Answers::new().with(FROM, Answer::Choice(String::from("clipboard")));
    assert_eq!(apply(&context, &clipboard), Outcome::Read(CLIPBOARD, Query::Clipboard));
    let read = clipboard.with(CLIPBOARD, Answer::Clipboard(Some(DIFF.to_owned())));
    assert_eq!(
        apply(&context, &read),
        Outcome::Read(
            CHECK,
            Query::PatchCheck {
                patch: PatchInput::Text(DIFF.as_bytes().to_vec())
            }
        )
    );
    let checked = read.with(CHECK, Answer::PatchCheck(PatchCheck::Applies { conflicts: Vec::new() }));
    assert!(matches!(
        apply(&context, &checked),
        Outcome::Read(TEMPORARY, Query::TemporaryFile { .. })
    ));
    let temporary = PathBuf::from("gitardo-1.patch");
    let plan = plan(apply(
        &context,
        &checked.with(TEMPORARY, Answer::Path(temporary.clone())),
    ));
    assert_eq!(
        lines(&plan),
        ["write gitardo-1.patch", "git apply --3way gitardo-1.patch"]
    );
    assert_eq!(plan.after, [FollowUp::Cleanup(Step::Delete { path: temporary })]);
    let empty = Answers::new()
        .with(FROM, Answer::Choice(String::from("clipboard")))
        .with(CLIPBOARD, Answer::Clipboard(None));
    let Outcome::Refuse(refusal) = apply(&context, &empty) else {
        panic!("no patch in the clipboard")
    };
    assert_eq!(refusal.message.as_str(), "There is no patch in the clipboard.");
}

#[test]
fn patch_apply__a_patch_that_does_not_apply_is_refused_with_the_reasons_of_git() {
    let context = context(state(), Target::Head);
    let reasons: Vec<String> = (1..=17).map(|line| format!("patch failed: a.txt:{line}")).collect();
    let answers = from_file(DIFF, None, None).with(CHECK, Answer::PatchCheck(PatchCheck::Refused { reasons }));
    let Outcome::Refuse(refusal) = apply(&context, &answers) else {
        panic!("a patch that does not apply is refused")
    };
    let said: Vec<&str> = refusal.message.as_str().lines().collect();
    assert_eq!(said.first(), Some(&"Two.patch does not apply, so nothing was changed."));
    assert_eq!(said.len(), 1 + 15 + 1);
    assert_eq!(said.last(), Some(&"\u{2026}and 2 more"));
}

#[test]
fn patch_apply__a_patch_that_will_conflict_names_the_files_and_asks_first() {
    let context = context(state(), Target::Head);
    let plan = plan(apply(&context, &from_file(DIFF, None, Some(&["a.txt", "b.txt"]))));
    let Danger::Confirm(said) = plan.danger else {
        panic!("asks first")
    };
    assert!(
        said.as_str()
            .contains("This will stop with conflicts in 2 files:\na.txt\nb.txt"),
        "{said:?}"
    );
}

#[test]
fn patch_apply__as_commits_with_staged_changes_is_refused_before_git_runs() {
    let mut state = state();
    state.working_tree.staged.push(modified("a.txt"));
    let context = context(state, Target::Head);
    let Outcome::Refuse(refusal) = apply(&context, &from_file(&two_commits(), Some("commits"), None)) else {
        panic!("refused with something staged")
    };
    assert_eq!(
        refusal.message.as_str(),
        "git am applies patches only while nothing is staged: commit or unstage the staged changes first."
    );
}

#[test]
fn patch_apply__undo_takes_back_every_commit_of_the_patch_at_once() {
    let context = context(state(), Target::Head);
    let plan = plan(apply(&context, &from_file(&two_commits(), Some("commits"), Some(&[]))));
    let head = context.repository.head.commit.clone().unwrap();
    assert_eq!(plan.undo, Undo::Journal(UndoRecord::Series { head_before: head }));
}
