#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::BTreeMap;

use super::{BASE, BaseOf, CONTAINING, FormatAsking, INPUTS, TYPE_ANOTHER, TYPED, base_of, name_from_format};
use crate::branch::{BranchInput, FormatValues, InputKind};
use crate::contract::{Answer, Answers, Outcome, Query, Question, QuestionKind, Target};
use crate::graph::testing::{branch, id, remote_branch};
use crate::state::{Head, RepositoryState};

fn input(id: &str, kind: InputKind) -> BranchInput {
    BranchInput {
        id: id.to_owned(),
        kind,
        description: None,
        default: None,
        pattern: None,
        options: Vec::new(),
        allow_custom: false,
    }
}

fn asking<'a>(format: &'a str, inputs: &'a [BranchInput], used: &'a BTreeMap<String, String>) -> FormatAsking<'a> {
    FormatAsking {
        format,
        inputs,
        used,
        base: BaseOf::Known(String::from("main")),
        values: FormatValues {
            remotes: vec![String::from("origin")],
            ..FormatValues::default()
        },
        title: "Create branch",
    }
}

fn asked(outcome: Result<super::FormatName, Box<Outcome>>) -> Question {
    match outcome.map_err(|outcome| *outcome) {
        Err(Outcome::Ask(question)) => question,
        other => panic!("a question, not {other:?}"),
    }
}

fn labels(question: &Question) -> Vec<(String, Option<String>)> {
    let QuestionKind::Pick { choices } = &question.kind else {
        panic!("a pick, not {:?}", question.kind)
    };
    choices
        .iter()
        .map(|choice| {
            (
                choice.label.as_str().to_owned(),
                choice.description.as_ref().map(|text| text.as_str().to_owned()),
            )
        })
        .collect()
}

/// `main` checked out at `A`, `feature` at `B`, which has `A`, and `origin/main` at `A`.
fn repository() -> RepositoryState {
    RepositoryState {
        head: Head {
            commit: Some(id("A")),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![branch("main", "A", None), branch("feature", "B", None)],
        remote_branches: vec![remote_branch("origin/main", "A")],
        remotes: vec![String::from("origin")],
        ..RepositoryState::default()
    }
}

#[test]
fn branch_name_from_format__an_input_used_twice_is_asked_once() {
    let inputs = [input("a", InputKind::PromptString)];
    let used = BTreeMap::new();
    let asking = asking("${input:a}-${input:a}", &inputs, &used);
    let first = asked(name_from_format(&asking, &Answers::new()));
    assert_eq!(first.key, INPUTS[0]);
    assert_eq!(
        first.detail.map(|text| text.as_str().to_owned()),
        Some(String::from("a"))
    );
    let made = name_from_format(
        &asking,
        &Answers::new().with(INPUTS[0], Answer::Text(String::from("x"))),
    )
    .unwrap();
    assert_eq!(made.name, "x-x");
    assert_eq!(made.inputs.get("a").map(String::as_str), Some("x"));
}

#[test]
fn branch_name_from_format__a_value_that_does_not_match_the_pattern_cannot_be_accepted() {
    let mut ticket = input("ticket", InputKind::PromptString);
    ticket.pattern = Some(String::from(r"^\d+$"));
    let inputs = [ticket];
    let used = BTreeMap::new();
    let question = asked(name_from_format(
        &asking("${input:ticket}", &inputs, &used),
        &Answers::new(),
    ));
    let QuestionKind::Text { validation, .. } = question.kind else {
        panic!("an input box")
    };
    assert_eq!(
        validation.problem("12a").map(|text| text.as_str().to_owned()),
        Some(String::from(r"It has to match `^\d+$`"))
    );
    assert_eq!(validation.problem("12"), None);
    assert!(validation.problem("").is_some(), "it cannot be empty");
}

#[test]
fn branch_name_from_format__a_pattern_that_is_not_a_regular_expression_is_refused_before_any_question() {
    let mut ticket = input("ticket", InputKind::PromptString);
    ticket.pattern = Some(String::from("(unclosed"));
    let inputs = [ticket];
    let used = BTreeMap::new();
    let outcome = name_from_format(&asking("${input:ticket}", &inputs, &used), &Answers::new());
    let Err(Outcome::Refuse(refusal)) = outcome.map_err(|outcome| *outcome) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "The pattern of the input `ticket` in `branches.inputs` is not a valid regular expression."
    );
}

#[test]
fn branch_name_from_format__the_second_time_the_box_starts_with_the_value_given_the_first_time() {
    let mut kind = input("kind", InputKind::PromptString);
    kind.default = Some(String::from("feat"));
    let inputs = [kind];
    let none = BTreeMap::new();
    let first = asked(name_from_format(
        &asking("${input:kind}", &inputs, &none),
        &Answers::new(),
    ));
    assert_eq!(
        first.default,
        Some(Answer::Text(String::from("feat"))),
        "the default before any was used"
    );
    let given = name_from_format(
        &asking("${input:kind}", &inputs, &none),
        &Answers::new().with(INPUTS[0], Answer::Text(String::from("fix"))),
    )
    .unwrap();
    let second = asked(name_from_format(
        &asking("${input:kind}", &inputs, &given.inputs),
        &Answers::new(),
    ));
    assert_eq!(second.default, Some(Answer::Text(String::from("fix"))));
}

#[test]
fn branch_name_from_format__a_value_typed_last_is_listed_first_as_used_last() {
    let mut kind = input("kind", InputKind::PickString);
    kind.options = vec![String::from("feat"), String::from("fix")];
    kind.allow_custom = true;
    let inputs = [kind];
    let used: BTreeMap<String, String> = [(String::from("kind"), String::from("chore"))].into();
    let question = asked(name_from_format(
        &asking("${input:kind}", &inputs, &used),
        &Answers::new(),
    ));
    let used_last = Some(String::from("Used last"));
    assert_eq!(
        labels(&question),
        [
            (String::from("chore"), used_last),
            (String::from("feat"), None),
            (String::from("fix"), None),
            (String::from("Type another value\u{2026}"), None)
        ]
    );
    let answers = Answers::new().with(INPUTS[0], Answer::Choice(TYPE_ANOTHER.to_owned()));
    let typed = asked(name_from_format(&asking("${input:kind}", &inputs, &used), &answers));
    assert_eq!(typed.key, TYPED[0]);
    let answers = answers.with(TYPED[0], Answer::Text(String::from("docs")));
    assert_eq!(
        name_from_format(&asking("${input:kind}", &inputs, &used), &answers)
            .unwrap()
            .name,
        "docs"
    );
}

#[test]
fn branch_name_from_format__from_the_header_on_main_base_is_main_and_nothing_is_asked() {
    let state = repository();
    let used = BTreeMap::new();
    let mut asking = asking("${base}-x", &[], &used);
    asking.base = base_of(&state, &Target::Head);
    assert_eq!(asking.base, BaseOf::Known(String::from("main")));
    assert_eq!(name_from_format(&asking, &Answers::new()).unwrap().name, "main-x");
}

#[test]
fn branch_name_from_format__at_a_commit_main_points_at_and_feature_has_the_pick_lists_main_the_hash_then_feature() {
    let state = repository();
    let used = BTreeMap::new();
    let mut asking = asking("${base}-x", &[], &used);
    asking.base = base_of(&state, &Target::Commit(id("A")));
    assert_eq!(
        name_from_format(&asking, &Answers::new()),
        Err(Box::new(Outcome::Read(
            CONTAINING,
            Query::Containing { commit: id("A") }
        )))
    );
    let read = Answers::new().with(
        CONTAINING,
        Answer::Names(vec![
            String::from("refs/heads/feature"),
            String::from("refs/heads/main"),
            String::from("refs/remotes/origin/main"),
        ]),
    );
    let question = asked(name_from_format(&asking, &read));
    assert_eq!(question.title.as_str(), "Create branch: what ${base} is");
    let at = Some(String::from("Branch at this commit"));
    assert_eq!(
        labels(&question),
        [
            (String::from("main"), at),
            (id("A").short().to_owned(), Some(String::from("This commit"))),
            (
                String::from("feature"),
                Some(String::from("Branch that has this commit"))
            )
        ],
        "origin/main is main, listed once"
    );
    let picked = read.with(BASE, Answer::Choice(String::from("feature")));
    assert_eq!(name_from_format(&asking, &picked).unwrap().name, "feature-x");
}

#[test]
fn branch_name_from_format__a_format_without_base_asks_nothing_about_it_also_at_a_commit() {
    let state = repository();
    let used = BTreeMap::new();
    let mut asking = asking("spike-${repo}", &[], &used);
    asking.values.repository = String::from("app");
    asking.base = base_of(&state, &Target::Commit(id("A")));
    assert_eq!(name_from_format(&asking, &Answers::new()).unwrap().name, "spike-app");
}
