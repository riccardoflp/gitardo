use super::{CHECK_OUT, CREATE_ONLY, MODE, NAME, create};
use crate::testing::{branch, commit, context, lines, plan, question, state, tag};
use model::contract::{Answer, Answers, QuestionKind, Target, Text, Undo};

fn answered(name: &str, mode: &str) -> Answers {
    Answers::new()
        .with(NAME, Answer::Text(name.to_owned()))
        .with(MODE, Answer::Choice(mode.to_owned()))
}

#[test]
fn branch_create__from_the_toolbar_checks_out_a_new_branch_at_head() {
    let context = context(state(), Target::Head);
    let name = question(create(&context, &Answers::new()));
    assert_eq!(name.title.as_str(), "Create branch");
    let mode = question(create(
        &context,
        &Answers::new().with(NAME, Answer::Text(String::from("fix/login"))),
    ));
    assert_eq!(mode.title.as_str(), "Create branch `fix/login`");
    let plan = plan(create(&context, &answered("fix/login", CHECK_OUT)));
    assert_eq!(lines(&plan), ["git checkout -b fix/login HEAD"]);
    assert_eq!(plan.undo, Undo::ByReflog);
}

#[test]
fn branch_create__on_a_commit_creates_only_at_its_full_hash() {
    let context = context(state(), Target::Commit(commit('b')));
    let plan = plan(create(&context, &answered("spike", CREATE_ONLY)));
    assert_eq!(lines(&plan), [format!("git branch spike {}", commit('b'))]);
    assert!(matches!(plan.undo, Undo::Impossible(_)));
}

#[test]
fn branch_create__names_a_branch_with_a_tag_of_its_name_in_full() {
    let mut state = state();
    state.branches.push(branch("release", 'b', false));
    state.tags.push(tag("release", 'c'));
    let context = context(state, Target::Branch(String::from("release")));
    let plan = plan(create(&context, &answered("hotfix", CREATE_ONLY)));
    assert_eq!(lines(&plan), ["git branch hotfix refs/heads/release"]);
}

#[test]
fn branch_create__refuses_a_name_already_taken() {
    let name = question(create(&context(state(), Target::Head), &Answers::new()));
    let QuestionKind::Text { validation, .. } = name.kind else {
        panic!("the name is typed")
    };
    assert_eq!(
        validation.problem("main").map(|text| text.to_string()),
        Some(String::from("A branch with that name already exists"))
    );
    assert_eq!(validation.problem("fix/login"), None);
}

#[test]
fn branch_create__refuses_a_name_git_does_not_take() {
    let name = question(create(&context(state(), Target::Head), &Answers::new()));
    let QuestionKind::Text { validation, .. } = name.kind else {
        panic!("the name is typed")
    };
    assert_eq!(
        validation.problem("bad name").map(|text| text.to_string()),
        Some(String::from("That is not a valid Git ref name"))
    );
}

#[test]
fn branch_create__offers_to_check_out_or_create_only() {
    let mode = question(create(
        &context(state(), Target::Head),
        &Answers::new().with(NAME, Answer::Text(String::from("x"))),
    ));
    let QuestionKind::Pick { choices } = mode.kind else {
        panic!("the mode is picked")
    };
    let labels: Vec<(&str, Option<&str>)> = choices
        .iter()
        .map(|choice| (choice.label.as_str(), choice.description.as_ref().map(Text::as_str)))
        .collect();
    assert_eq!(
        labels,
        [
            ("Create and check out", None),
            ("Create only", Some("Stay on the current branch"))
        ]
    );
}
