use super::{AGAIN, RESOLVE, TEXTS, resolve_automatically};
use crate::testing::{context, lines, plan, question, state};
use model::conflicts::ConflictTexts;
use model::contract::{Answer, Answers, Context, FeatureId, FollowUp, Outcome, Query, Target, Text, Undo, UndoRecord};
use model::state::{ConflictKind, ConflictedFile, Operation};

/// A merge stopped with these files in conflict, `conflicts.autoResolve` on.
fn merging(paths: &[&str]) -> Context {
    let mut state = state();
    state.operation = Some(Operation::Merge);
    state.working_tree.conflicted = paths
        .iter()
        .map(|path| ConflictedFile {
            path: (*path).to_owned(),
            kind: ConflictKind::BothModified,
        })
        .collect();
    let mut context = context(state, Target::Head);
    context.settings.conflicts_auto_resolve = true;
    context
}

/// A file whose one conflict is two changes to lines next to each other.
fn next_to_each_other(path: &str) -> ConflictTexts {
    texts(
        path,
        "a\n<<<<<<< HEAD\nB\nc\n=======\nb\nC\n>>>>>>> feature\n",
        ["a\nb\nc\n", "a\nB\nc\n", "a\nb\nC\n"],
    )
}

/// A file with a conflict like that, then one that needs a choice.
fn in_part(path: &str) -> ConflictTexts {
    texts(
        path,
        "a\n<<<<<<< HEAD\nB\nc\n=======\nb\nC\n>>>>>>> feature\nd\nx\n<<<<<<< HEAD\nY1\n=======\nY2\n>>>>>>> feature\nz\n",
        [
            "a\nb\nc\nd\nx\ny\nz\n",
            "a\nB\nc\nd\nx\nY1\nz\n",
            "a\nb\nC\nd\nx\nY2\nz\n",
        ],
    )
}

fn texts(path: &str, file: &str, [base, current, incoming]: [&str; 3]) -> ConflictTexts {
    ConflictTexts {
        path: path.to_owned(),
        file: Some(file.as_bytes().to_vec()),
        base: Some(base.as_bytes().to_vec()),
        current: Some(current.as_bytes().to_vec()),
        incoming: Some(incoming.as_bytes().to_vec()),
    }
}

/// The answers once the texts are read, the question answered and the texts read again, the same.
fn answered(texts: &[ConflictTexts], choice: &str) -> Answers {
    Answers::new()
        .with(TEXTS, Answer::ConflictTexts(texts.to_vec()))
        .with(RESOLVE, Answer::Choice(choice.to_owned()))
        .with(AGAIN, Answer::ConflictTexts(texts.to_vec()))
}

#[test]
fn conflicts_resolve_automatically__writes_both_files_then_stages_only_the_one_resolved_in_full() {
    let context = merging(&["first.txt", "second.txt"]);
    assert!(matches!(
        resolve_automatically(&context, &Answers::new()),
        Outcome::Read(TEXTS, Query::ConflictTexts)
    ));
    let texts = [next_to_each_other("first.txt"), in_part("second.txt")];
    let asked = question(resolve_automatically(
        &context,
        &Answers::new().with(TEXTS, Answer::ConflictTexts(texts.to_vec())),
    ));
    assert_eq!(asked.title.as_str(), "Resolve 2 of 3 conflicts automatically?");
    assert_eq!(
        asked.detail.as_ref().map(Text::as_str),
        Some(
            "first.txt: the conflict (changes to lines next to each other)\nsecond.txt: 1 of 2 conflicts (changes to \
             lines next to each other)\nEvery line written is a line of one of the two sides.\n1 conflict will be \
             left to resolve by hand, with the markers as they are.\nWrites first.txt, second.txt, then runs git add \
             -- first.txt."
        )
    );
    let plan = plan(resolve_automatically(&context, &answered(&texts, "resolve")));
    let folder = std::path::Path::new("repository");
    assert_eq!(
        lines(&plan),
        [
            format!("write {}", folder.join("first.txt").display()),
            format!("write {}", folder.join("second.txt").display()),
            String::from("git add -- first.txt"),
        ]
    );
    let Some(model::contract::Step::Write { contents, .. }) = plan.steps.get(1) else {
        panic!("the second file is written")
    };
    assert_eq!(
        String::from_utf8_lossy(contents),
        "a\nB\nC\nd\nx\n<<<<<<< HEAD\nY1\n=======\nY2\n>>>>>>> feature\nz\n"
    );
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::ConflictsResolved {
            files: vec![String::from("first.txt"), String::from("second.txt")]
        })
    );
}

#[test]
fn conflicts_resolve_automatically__review_opens_the_review_of_the_same_files_and_writes_nothing() {
    let context = merging(&["first.txt", "second.txt"]);
    let texts = [next_to_each_other("first.txt"), in_part("second.txt")];
    let answers = Answers::new()
        .with(TEXTS, Answer::ConflictTexts(texts.to_vec()))
        .with(RESOLVE, Answer::Choice(String::from("review")));
    let plan = plan(resolve_automatically(&context, &answers));
    assert_eq!(plan.steps, []);
    assert_eq!(
        plan.after,
        [FollowUp::OpenReview(vec![
            String::from("first.txt"),
            String::from("second.txt")
        ])]
    );
}

#[test]
fn conflicts_resolve_automatically__a_file_edited_after_the_question_writes_no_file_and_says_which() {
    let context = merging(&["first.txt", "second.txt"]);
    let texts = [next_to_each_other("first.txt"), in_part("second.txt")];
    let mut edited = texts.to_vec();
    if let Some(second) = edited.get_mut(1) {
        second.file = Some(b"typed meanwhile\n".to_vec());
    }
    let answers = answered(&texts, "resolve").with(AGAIN, Answer::ConflictTexts(edited));
    let Outcome::Refuse(refusal) = resolve_automatically(&context, &answers) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "second.txt has changed since it was read, so nothing was written. Try again."
    );
}

#[test]
fn conflicts_resolve_automatically__switched_off_it_refuses_and_offers_the_settings() {
    let mut context = merging(&["first.txt"]);
    context.settings.conflicts_auto_resolve = false;
    let Outcome::Refuse(refusal) = resolve_automatically(&context, &Answers::new()) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "Resolving conflicts automatically is experimental, and it is switched off in the settings."
    );
    assert_eq!(
        refusal.offer,
        Some((FeatureId("settings.edit"), Text::from("Open settings")))
    );
}

#[test]
fn conflicts_resolve_automatically__with_no_file_left_during_a_merge_it_offers_continue_and_undo() {
    let context = merging(&["first.txt"]);
    let texts = [next_to_each_other("first.txt")];
    let plan = plan(resolve_automatically(&context, &answered(&texts, "resolve")));
    assert_eq!(
        plan.after,
        [
            FollowUp::Say(Text::from(
                "Gitardo resolved conflicts in 1 file. No file is left in conflict: the merge can continue."
            )),
            FollowUp::Offer(FeatureId("conflicts.continue-operation"), Text::from("Continue")),
        ]
    );
    assert!(matches!(plan.undo, Undo::Journal(_)));
}

#[test]
fn says_which_files_are_still_in_conflict_and_refuses_when_nothing_can_be_resolved() {
    let context = merging(&["first.txt", "second.txt"]);
    let parts = [in_part("first.txt")];
    let plan = plan(resolve_automatically(&context, &answered(&parts, "resolve")));
    assert_eq!(
        plan.after,
        [FollowUp::Say(Text::from(
            "Gitardo resolved conflicts in 1 file. 2 files still have conflicts to resolve by hand."
        ))]
    );
    let choice = texts(
        "a.txt",
        "a\n<<<<<<< HEAD\nB1\n=======\nB2\n>>>>>>> feature\nc\n",
        ["a\nb\nc\n", "a\nB1\nc\n", "a\nB2\nc\n"],
    );
    let answers = Answers::new().with(TEXTS, Answer::ConflictTexts(vec![choice]));
    let Outcome::Refuse(refusal) = resolve_automatically(&context, &answers) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "The conflict cannot be resolved without choosing a side."
    );
}

#[test]
fn conflicts_resolve_automatically__its_question_is_the_confirmation_whatever_command_preview_says() {
    let mut context = merging(&["first.txt"]);
    context.settings.command_preview = model::contract::CommandPreview::All;
    let texts = [next_to_each_other("first.txt")];
    for choice in ["resolve", "review"] {
        let plan = plan(resolve_automatically(&context, &answered(&texts, choice)));
        assert_eq!(plan.danger, model::contract::Danger::Agreed, "{choice}");
    }
}
