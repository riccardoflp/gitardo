use super::{CAN_FAST_FORWARD, COUNTS, WAY, choose_way};
use crate::merge::CONFLICTS;
use crate::testing::{branch, context, lines, plan, question, state};
use model::contract::{
    Answer, Answers, Context, Danger, FollowUp, Outcome, Query, QuestionKind, Target, Undo, UndoRecord,
};

fn on_feature() -> Context {
    let mut state = state();
    state.branches.push(branch("feature", 'b', false));
    context(state, Target::Branch(String::from("feature")))
}

fn answered(can_fast_forward: bool, way: &str) -> Answers {
    Answers::new()
        .with(CAN_FAST_FORWARD, Answer::Flag(can_fast_forward))
        .with(WAY, Answer::Choice(way.to_owned()))
        .with(CONFLICTS, Answer::Conflicts(Some(Vec::new())))
}

#[test]
fn merge_choose_way__lists_four_ways_and_the_third_is_not_possible_with_a_commit_of_its_own() {
    let context = on_feature();
    assert!(matches!(
        choose_way(&context, &Answers::new()),
        Outcome::Read(CAN_FAST_FORWARD, Query::IsAncestor { .. })
    ));
    let read = Answers::new().with(CAN_FAST_FORWARD, Answer::Flag(false));
    let asked = question(choose_way(&context, &read));
    assert_eq!(asked.title.as_str(), "Merge feature into main");
    let QuestionKind::Pick { choices } = asked.kind else {
        panic!("the way is picked")
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Merge",
            "Merge (no fast-forward)",
            "Merge (fast-forward only)",
            "Merge (squash)"
        ]
    );
    let third = choices[2].description.as_ref().unwrap().as_str();
    assert_eq!(
        third,
        "git merge --ff-only feature · Not possible: main has commits that feature has not"
    );
}

#[test]
fn merge_choose_way__a_fast_forward_only_that_is_not_possible_runs_nothing_and_says_why() {
    let context = on_feature();
    let answers = answered(false, "ff-only");
    assert!(matches!(
        choose_way(&context, &answers),
        Outcome::Read(COUNTS, Query::Counts { .. })
    ));
    let Outcome::Refuse(refusal) = choose_way(&context, &answers.with(COUNTS, Answer::Counts { left: 1, right: 2 }))
    else {
        panic!("it is refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "main cannot be fast-forwarded to feature: it has commits that feature has not. Merge it another way, or \
         rebase main onto it."
    );
}

#[test]
fn merge_choose_way__no_fast_forward_runs_git_merge_no_ff() {
    let plan = plan(choose_way(&on_feature(), &answered(true, "no-ff")));
    assert_eq!(lines(&plan), ["git merge --no-ff feature"]);
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn merge_choose_way__a_squash_keeps_its_undo_in_the_journal_and_fills_the_message_of_the_draft() {
    let plan = plan(choose_way(&on_feature(), &answered(true, "squash")));
    assert_eq!(lines(&plan), ["git merge --squash feature"]);
    assert_eq!(plan.title.as_str(), "Merge feature into main (squash)");
    assert_eq!(plan.undo, Undo::Journal(UndoRecord::SquashMerge));
    assert!(plan.after.contains(&FollowUp::PreparedMessage));
    assert!(matches!(plan.danger, Danger::Confirm(ref reason) if reason.as_str().ends_with("No conflicts expected.")));
}

#[test]
fn merge_choose_way__closing_the_pick_runs_nothing() {
    let context = on_feature();
    let read = Answers::new().with(CAN_FAST_FORWARD, Answer::Flag(true));
    assert!(matches!(choose_way(&context, &read), Outcome::Ask(_)));
}
