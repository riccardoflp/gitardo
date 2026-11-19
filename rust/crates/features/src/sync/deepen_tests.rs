#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::CommitId;
use model::contract::{Answer, Answers, Outcome, Target, Validation};
use model::state::{RepositoryState, Shallow};

use super::{COUNT, deepen, unshallow};
use crate::testing::{context, lines, plan, state};

/// A shallow clone of `origin`, its history ending at one commit.
fn shallow_clone() -> RepositoryState {
    let mut state = state();
    state.remotes.push(String::from("origin"));
    state.shallow = Shallow {
        ends: vec![CommitId::parse("0123456789abcdef0123456789abcdef01234567").unwrap()],
    };
    state
}

fn count(text: &str) -> Answers {
    Answers::new().with(COUNT, Answer::Text(text.to_owned()))
}

#[test]
fn sync_deepen__three_more_commits_runs_git_fetch_deepen_3_from_origin() {
    let context = context(shallow_clone(), Target::Head);
    assert_eq!(
        lines(&plan(deepen(&context, &count("3")))),
        ["git fetch --deepen=3 origin"]
    );
}

#[test]
fn sync_deepen__the_count_asked_starts_at_100_and_refuses_what_is_not_a_whole_number_of_at_least_1() {
    let context = context(shallow_clone(), Target::Head);
    let Outcome::Ask(question) = deepen(&context, &Answers::new()) else {
        panic!("the count is asked");
    };
    assert_eq!(question.default, Some(Answer::Text(String::from("100"))));
    let model::contract::QuestionKind::Text { validation, .. } = &question.kind else {
        panic!("a text box");
    };
    assert_eq!(*validation, Validation::Count);
    for refused in ["0", "ten"] {
        assert!(validation.problem(refused).is_some(), "{refused}");
    }
    assert_eq!(validation.problem("100"), None);
}

#[test]
fn sync_deepen__a_repository_that_is_not_shallow_is_refused() {
    let mut state = state();
    state.remotes.push(String::from("origin"));
    assert!(matches!(
        deepen(&context(state, Target::Head), &count("3")),
        Outcome::Refuse(_)
    ));
}

#[test]
fn sync_deepen__a_shallow_repository_with_no_remote_is_refused_with_add_remote() {
    let mut state = shallow_clone();
    state.remotes.clear();
    let Outcome::Refuse(refusal) = deepen(&context(state, Target::Head), &count("3")) else {
        panic!("refused");
    };
    assert_eq!(refusal.message.as_str(), "This repository has no remote.");
}

#[test]
fn sync_unshallow__it_runs_git_fetch_unshallow_from_origin_titled_load_the_whole_history_from_origin() {
    let outcome = unshallow(&context(shallow_clone(), Target::Head), &Answers::new());
    let Outcome::Run(plan) = &outcome else {
        panic!("a plan");
    };
    assert_eq!(plan.title.as_str(), "Load the whole history from origin");
    assert_eq!(lines(plan), ["git fetch --unshallow origin"]);
}
