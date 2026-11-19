#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::Timestamp;
use model::commit::Commit;
use model::contract::{Answer, Answers, Context, Outcome, Query, QuestionKind, Target};
use model::tag::{RemoteTags, TagObject};

use super::{COMMITS, REMOTE, REMOTE_TAGS, TAGS, push_all};
use crate::testing::{commit, context, lines, plan, question, state, tag};

/// A repository with `v1`, `v2` and `v3`, `v3` the newest, and the remotes given.
fn repository(remotes: &[&str]) -> Context {
    let mut state = state();
    for (name, digit, date) in [("v1", 'a', 1), ("v2", 'b', 2), ("v3", 'c', 3)] {
        let mut made = tag(name, digit);
        made.date = Timestamp(date);
        state.tags.push(made);
    }
    state.remotes = remotes.iter().map(|remote| (*remote).to_owned()).collect();
    context(state, Target::Head)
}

fn object(name: &str, digit: char) -> TagObject {
    TagObject {
        name: name.to_owned(),
        object: digit.to_string().repeat(40),
    }
}

/// What `origin` has: `v1` as here, or `v1` under another object when `moved`.
fn read(moved: bool) -> Answers {
    let there = vec![object("v1", if moved { 'f' } else { 'a' })];
    let here = vec![object("v1", 'a'), object("v2", 'b'), object("v3", 'c')];
    let commits = ['c', 'b']
        .into_iter()
        .map(|digit| Commit {
            id: commit(digit),
            parents: Vec::new(),
            author_name: String::new(),
            author_email: String::new(),
            author_date: Timestamp(0),
            subject: format!("Release {digit}"),
        })
        .collect();
    Answers::new()
        .with(
            REMOTE_TAGS,
            Answer::RemoteTags(RemoteTags {
                remote: String::from("origin"),
                there,
                here,
            }),
        )
        .with(COMMITS, Answer::Commits(commits))
}

#[test]
fn tag_push_all__the_pick_lists_the_tags_the_remote_lacks_ticked_and_every_one_ticked_pushes_with_tags() {
    let context = repository(&["origin"]);
    let Outcome::Read(_, Query::RemoteTags { remote }) = push_all(&context, &Answers::new()) else {
        panic!("the remote is asked for its tags first")
    };
    assert_eq!(remote, "origin");
    let asked = question(push_all(&context, &read(false)));
    assert_eq!(asked.title.as_str(), "Tags not on origin");
    let QuestionKind::PickMany { choices } = &asked.kind else {
        panic!("a pick of several")
    };
    let names: Vec<&str> = choices.iter().map(|choice| choice.identifier.as_str()).collect();
    assert_eq!(names, ["v3", "v2"]);
    assert_eq!(
        choices[0].description.as_ref().map(model::contract::Text::as_str),
        Some("ccccccc Release c")
    );
    assert_eq!(
        asked.default,
        Some(Answer::Choices(vec![String::from("v3"), String::from("v2")]))
    );
    let answers = read(false).with(TAGS, Answer::Choices(vec![String::from("v3"), String::from("v2")]));
    assert_eq!(lines(&plan(push_all(&context, &answers))), ["git push origin --tags"]);
}

#[test]
fn tag_push_all__unticking_one_names_the_others_in_full() {
    let answers = read(false).with(TAGS, Answer::Choices(vec![String::from("v2")]));
    assert_eq!(
        lines(&plan(push_all(&repository(&["origin"]), &answers))),
        ["git push origin refs/tags/v2"]
    );
}

#[test]
fn tag_push_all__a_tag_moved_here_is_left_out_and_said_and_the_push_names_the_others() {
    let context = repository(&["origin"]);
    let asked = question(push_all(&context, &read(true)));
    let QuestionKind::PickMany { choices } = &asked.kind else {
        panic!("a pick of several")
    };
    assert!(choices.iter().all(|choice| choice.identifier != "v1"));
    assert_eq!(
        asked.detail.as_ref().map(model::contract::Text::as_str),
        Some("origin has another tag of this name, which is not pushed: v1.")
    );
    let answers = read(true).with(TAGS, Answer::Choices(vec![String::from("v3"), String::from("v2")]));
    assert_eq!(
        lines(&plan(push_all(&context, &answers))),
        ["git push origin refs/tags/v3 refs/tags/v2"]
    );
}

#[test]
fn tag_push_all__with_every_tag_on_the_remote_no_pick_opens_and_it_says_so() {
    let mut context = repository(&["origin"]);
    let mut state = (*context.repository).clone();
    state.tags.truncate(1);
    context.repository = std::sync::Arc::new(state);
    assert!(matches!(
        push_all(&context, &read(false)),
        Outcome::Refuse(refusal) if refusal.message.as_str() == "origin has every tag of this repository."
    ));
}

#[test]
fn tag_push_all__nothing_ticked_and_no_remote_refuse_and_several_remotes_ask_which() {
    let answers = read(false).with(TAGS, Answer::Choices(Vec::new()));
    assert!(matches!(
        push_all(&repository(&["origin"]), &answers),
        Outcome::Refuse(refusal) if refusal.message.as_str() == "No tags selected."
    ));
    assert!(matches!(
        push_all(&repository(&[]), &Answers::new()),
        Outcome::Refuse(refusal) if refusal.offer.is_some()
    ));
    let asked = question(push_all(&repository(&["upstream", "origin"]), &Answers::new()));
    assert_eq!(asked.key, REMOTE);
    assert_eq!(asked.title.as_str(), "Push tags to");
}
