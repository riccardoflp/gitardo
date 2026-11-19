use super::{KEEP_FILES, KEEPS, NAME, NO_FILES, create_orphan};
use crate::testing::{context, lines, modified, plan, question, state};
use model::contract::{Answer, Answers, Danger, Outcome, QuestionKind, Target, Undo, UndoRecord, Validation};
use model::state::Operation;

/// The name `site` and what the working tree keeps answered.
fn answered(keeps: &str) -> Answers {
    Answers::new()
        .with(NAME, Answer::Text(String::from("site")))
        .with(KEEPS, Answer::Choice(keeps.to_owned()))
}

#[test]
fn branch_create_orphan__start_with_no_files_switches_to_the_branch_with_no_history_once_confirmed() {
    let context = context(state(), Target::Head);
    let name = question(create_orphan(&context, &Answers::new()));
    assert_eq!(name.title.as_str(), "Create branch with no history");
    let keeps = question(create_orphan(
        &context,
        &Answers::new().with(NAME, Answer::Text(String::from("site"))),
    ));
    assert_eq!(keeps.title.as_str(), "What the working tree keeps");
    assert_eq!(keeps.default, Some(Answer::Choice(NO_FILES.to_owned())));
    let plan = plan(create_orphan(&context, &answered(NO_FILES)));
    assert_eq!(lines(&plan), ["git switch --orphan site"]);
    let Danger::Confirm(reason) = &plan.danger else {
        panic!("the files leaving the working tree are confirmed")
    };
    assert!(reason.as_str().contains("main still has them"), "{reason}");
}

#[test]
fn branch_create_orphan__keep_the_files_staged_checks_out_the_branch_with_no_history() {
    let plan = plan(create_orphan(&context(state(), Target::Head), &answered(KEEP_FILES)));
    assert_eq!(lines(&plan), ["git checkout --orphan site"]);
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn branch_create_orphan__a_tracked_file_modified_refuses_no_files_and_lets_the_files_be_kept() {
    let mut changed = state();
    changed.working_tree.unstaged.push(modified("README.md"));
    let context = context(changed, Target::Head);
    let Outcome::Refuse(refusal) = create_orphan(&context, &answered(NO_FILES)) else {
        panic!("starting with no files is refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "Commit or stash the changes first: starting with no files would take them out of the working tree."
    );
    assert!(matches!(
        create_orphan(&context, &answered(KEEP_FILES)),
        Outcome::Run(_)
    ));
}

#[test]
fn branch_create_orphan__undo_comes_from_the_journal_with_what_was_checked_out_before() {
    let context = context(state(), Target::Head);
    let plan = plan(create_orphan(&context, &answered(NO_FILES)));
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::BranchWithNoHistory {
            name: String::from("site"),
            before: context.repository.head.clone(),
        })
    );
}

#[test]
fn branch_create_orphan__a_name_already_taken_is_said_as_it_is_typed() {
    let name = question(create_orphan(&context(state(), Target::Head), &Answers::new()));
    let QuestionKind::Text {
        validation: Validation::ReferenceName { taken, taken_message },
        ..
    } = name.kind
    else {
        panic!("the name is checked as a reference name")
    };
    assert_eq!(taken, ["main"]);
    assert_eq!(taken_message.as_str(), "A branch with that name already exists");
}

#[test]
fn branch_create_orphan__refuses_while_an_operation_is_under_way_and_before_the_first_commit() {
    let mut merging = state();
    merging.operation = Some(Operation::Merge);
    assert!(matches!(
        create_orphan(&context(merging, Target::Head), &Answers::new()),
        Outcome::Refuse(_)
    ));
    let mut empty = state();
    empty.head.commit = None;
    assert!(matches!(
        create_orphan(&context(empty, Target::Head), &Answers::new()),
        Outcome::Refuse(_)
    ));
}
