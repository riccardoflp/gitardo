use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target};
use model::rerere::RerereState;
use model::state::RepositoryState;

use super::{LEAVE, STAGE, WHEN_RESOLVED, turn_off, turn_on};
use crate::testing::{context, lines, plan, question, state};

/// A repository whose rerere is so.
fn with_rerere(rerere: RerereState) -> RepositoryState {
    RepositoryState { rerere, ..state() }
}

/// The answer of what git does with a conflict it resolved before.
fn when(answer: &str) -> Answers {
    Answers::new().with(WHEN_RESOLVED, Answer::Choice(answer.to_owned()))
}

#[test]
fn rerere_turn_on__with_the_first_answer_it_runs_rerere_enabled_true_alone() {
    let off = context(with_rerere(RerereState::default()), Target::Head);
    let ran = plan(turn_on(&off, &when(LEAVE)));
    assert_eq!(lines(&ran), ["git config --local rerere.enabled true"]);
    let said = ran.after.iter().find_map(|follow_up| match follow_up {
        FollowUp::Say(said) => Some(said.as_str().to_owned()),
        _ => None,
    });
    assert_eq!(
        said.as_deref(),
        Some("Git now records how you resolve conflicts in repository, and resolves the same conflicts the same way.")
    );
    let on = RerereState {
        is_on: true,
        ..RerereState::default()
    };
    assert!(matches!(
        turn_on(&context(with_rerere(on), Target::Head), &when(LEAVE)),
        Outcome::Refuse(_)
    ));
}

#[test]
fn rerere_turn_on__with_the_second_it_runs_both_commands() {
    let off = context(with_rerere(RerereState::default()), Target::Head);
    let ran = plan(turn_on(&off, &when(STAGE)));
    assert_eq!(
        lines(&ran),
        [
            "git config --local rerere.enabled true",
            "git config --local rerere.autoUpdate true"
        ]
    );
}

#[test]
fn rerere_turn_on__it_asks_first_and_runs_nothing_without_an_answer() {
    let off = context(with_rerere(RerereState::default()), Target::Head);
    let asked = question(turn_on(&off, &Answers::new()));
    assert_eq!(asked.title.as_str(), "When git meets a conflict it resolved before");
    let QuestionKind::Pick { choices } = asked.kind else {
        panic!("a pick");
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Write the same resolution, and leave the file for me to look at",
            "Write the same resolution and stage it"
        ]
    );
    assert_eq!(asked.default, Some(Answer::Choice(String::from(LEAVE))));
}

#[test]
fn rerere_turn_off__it_runs_rerere_enabled_false_and_unsets_auto_update_when_the_repository_sets_it() {
    let on = RerereState {
        is_on: true,
        ..RerereState::default()
    };
    let ran = plan(turn_off(&context(with_rerere(on), Target::Head), &Answers::new()));
    assert_eq!(lines(&ran), ["git config --local rerere.enabled false"]);
    let staging = RerereState {
        is_on: true,
        stages: true,
        sets_stages: true,
    };
    let ran = plan(turn_off(&context(with_rerere(staging), Target::Head), &Answers::new()));
    assert_eq!(
        lines(&ran),
        [
            "git config --local rerere.enabled false",
            "git config --local --unset rerere.autoUpdate"
        ]
    );
    assert!(matches!(
        turn_off(
            &context(with_rerere(RerereState::default()), Target::Head),
            &Answers::new()
        ),
        Outcome::Refuse(_)
    ));
}
