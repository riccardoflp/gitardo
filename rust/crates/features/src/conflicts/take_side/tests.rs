use super::{SIDE, STAGES, take_side};
use crate::testing::{context, lines, plan, state};
use model::conflicts::FileStages;
use model::contract::{Answer, Answers, Danger, Outcome, Query, Target, Undo, UndoRecord};
use model::state::Operation;

fn during(operation: Operation) -> model::contract::Context {
    let mut state = state();
    state.operation = Some(operation);
    context(state, Target::File(String::from("src/app.rs")))
}

fn stages(has_current: bool, has_incoming: bool) -> FileStages {
    FileStages {
        path: String::from("src/app.rs"),
        has_base: true,
        has_current,
        has_incoming,
    }
}

fn taking(side: &str, stages: FileStages) -> Answers {
    Answers::new()
        .with(SIDE, Answer::Choice(side.to_owned()))
        .with(STAGES, Answer::Stages(Some(stages)))
}

#[test]
fn conflicts_take_side__the_current_side_asks_with_the_words_of_the_operation_then_checks_it_out_and_stages_it() {
    let context = during(Operation::Merge);
    let read = take_side(
        &context,
        &Answers::new().with(SIDE, Answer::Choice(String::from("current"))),
    );
    assert!(matches!(read, Outcome::Read(STAGES, Query::ConflictStages { .. })));
    let plan = plan(take_side(&context, &taking("current", stages(true, true))));
    assert_eq!(
        lines(&plan),
        ["git checkout --ours -- src/app.rs", "git add -- src/app.rs"]
    );
    assert_eq!(plan.title.as_str(), "Take the current side of src/app.rs");
    assert_eq!(
        plan.danger,
        Danger::Confirm(
            "The current side is the branch checked out. The whole file becomes what that side has: the changes the \
             other side made to it are left out."
                .into()
        )
    );
}

#[test]
fn conflicts_take_side__in_a_rebase_the_current_side_is_what_the_branch_is_being_rebased_onto() {
    let plan = plan(take_side(
        &during(Operation::Rebase),
        &taking("current", stages(true, true)),
    ));
    let Danger::Confirm(reason) = &plan.danger else {
        panic!("asks first")
    };
    assert!(
        reason
            .as_str()
            .starts_with("The current side is what the branch is being rebased onto.")
    );
}

#[test]
fn conflicts_take_side__the_incoming_side_that_deleted_the_file_removes_it() {
    let plan = plan(take_side(
        &during(Operation::Merge),
        &taking("incoming", stages(true, false)),
    ));
    assert_eq!(lines(&plan), ["git rm -- src/app.rs"]);
    assert_eq!(plan.title.as_str(), "Delete src/app.rs, as the incoming side did");
    assert_eq!(
        plan.danger,
        Danger::Confirm(
            "The incoming side is the branch being merged, which has no such file. The changes the other side made \
             to it are left out."
                .into()
        )
    );
}

#[test]
fn conflicts_take_side__undo_of_a_file_with_both_sides_marks_its_conflicts_again() {
    let plan = plan(take_side(
        &during(Operation::Merge),
        &taking("incoming", stages(true, true)),
    ));
    assert_eq!(
        lines(&plan).first().map(String::as_str),
        Some("git checkout --theirs -- src/app.rs")
    );
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::ConflictsResolved {
            files: vec![String::from("src/app.rs")]
        })
    );
}

#[test]
fn conflicts_take_side__undo_of_a_file_one_side_deleted_is_not_offered() {
    let plan = plan(take_side(
        &during(Operation::Merge),
        &taking("incoming", stages(true, false)),
    ));
    assert!(matches!(plan.undo, Undo::Impossible(_)));
}

#[test]
fn a_file_that_is_not_in_conflict_is_refused() {
    let answers = Answers::new()
        .with(SIDE, Answer::Choice(String::from("current")))
        .with(STAGES, Answer::Stages(None));
    let Outcome::Refuse(refusal) = take_side(&during(Operation::Merge), &answers) else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "src/app.rs is not in conflict.");
}
