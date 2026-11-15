use std::collections::BTreeMap;

use super::{CHECK_OUT, CREATE_ONLY, MODE, NAME, TYPE_NAME, USE_FORMAT, WAY, create};
use crate::branch::submodules::{SUBMODULES_AFTER, SUBMODULES_BEFORE, none};
use crate::testing::{branch, commit, context, lines, plan, question, state, tag};
use model::Timestamp;
use model::branch::{BranchInput, INPUTS, InputKind};
use model::contract::{
    Answer, Answers, Context, FollowUp, Outcome, Query, QuestionKind, Target, Text, Undo, UndoRecord,
};

/// The name and the mode answered, and neither side of the checkout with submodules.
fn answered(name: &str, mode: &str) -> Answers {
    none(
        Answers::new()
            .with(NAME, Answer::Text(name.to_owned()))
            .with(MODE, Answer::Choice(mode.to_owned())),
    )
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
    assert_eq!(plan.after, [FollowUp::SelectHead]);
}

#[test]
fn branch_create__checked_out_where_there_are_submodules_it_ends_with_their_update() {
    let context = context(state(), Target::Commit(commit('b')));
    let named = Answers::new()
        .with(NAME, Answer::Text(String::from("fix")))
        .with(MODE, Answer::Choice(CHECK_OUT.to_owned()));
    let Outcome::Read(key, Query::HasFile { revision, path }) = create(&context, &named) else {
        panic!("whether HEAD has submodules is read first")
    };
    assert_eq!(
        (key, revision.as_str(), path.as_str()),
        (SUBMODULES_BEFORE, "HEAD", ".gitmodules")
    );
    let before = named.with(SUBMODULES_BEFORE, Answer::Flag(false));
    let Outcome::Read(key, Query::HasFile { revision, .. }) = create(&context, &before) else {
        panic!("then whether the start of the branch has them")
    };
    assert_eq!((key, revision), (SUBMODULES_AFTER, commit('b').to_string()));
    let plan = plan(create(&context, &before.with(SUBMODULES_AFTER, Answer::Flag(true))));
    let lines = lines(&plan);
    assert_eq!(
        lines.first().map(String::as_str),
        Some(format!("git checkout -b fix {}", commit('b')).as_str())
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some("git submodule update --init --recursive")
    );
}

#[test]
fn branch_create__with_the_update_of_submodules_off_or_created_only_nothing_is_read_and_nothing_updated() {
    let mut off = context(state(), Target::Head);
    off.settings.update_submodules_on_checkout = false;
    let named = |mode: &str| {
        Answers::new()
            .with(NAME, Answer::Text(String::from("fix")))
            .with(MODE, Answer::Choice(mode.to_owned()))
    };
    assert_eq!(
        lines(&plan(create(&off, &named(CHECK_OUT)))),
        ["git checkout -b fix HEAD"]
    );
    let on = context(state(), Target::Head);
    assert_eq!(lines(&plan(create(&on, &named(CREATE_ONLY)))), ["git branch fix HEAD"]);
}

#[test]
fn branch_create__on_a_commit_creates_only_at_its_full_hash() {
    let context = context(state(), Target::Commit(commit('b')));
    let plan = plan(create(&context, &answered("spike", CREATE_ONLY)));
    assert_eq!(lines(&plan), [format!("git branch spike {}", commit('b'))]);
    // Undo deletes it, from the journal, since nothing moved HEAD.
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::BranchCreated {
            name: String::from("spike")
        })
    );
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

fn with_format(format: &str) -> Context {
    let mut context = context(state(), Target::Head);
    context.settings.branch_format = format.to_owned();
    context.settings.branch_inputs = vec![BranchInput {
        id: String::from("ticket"),
        kind: InputKind::PromptString,
        description: Some(String::from("Ticket")),
        default: None,
        pattern: None,
        options: Vec::new(),
        allow_custom: false,
    }];
    context
}

#[test]
fn branch_create__with_a_format_set_the_first_question_is_use_the_format_or_type_a_branch_name() {
    let way = question(create(&with_format("${input:ticket}-${base}"), &Answers::new()));
    assert_eq!(way.key, WAY);
    assert_eq!(way.title.as_str(), "Create branch");
    assert_eq!(way.default, Some(Answer::Choice(USE_FORMAT.to_owned())));
    let QuestionKind::Pick { choices } = way.kind else {
        panic!("the way is picked")
    };
    let labels: Vec<(&str, Option<&str>)> = choices
        .iter()
        .map(|choice| (choice.label.as_str(), choice.description.as_ref().map(Text::as_str)))
        .collect();
    assert_eq!(
        labels,
        [
            ("Use the format", Some("${input:ticket}-${base}")),
            ("Type a branch name", None)
        ]
    );
    let typed = Answers::new().with(WAY, Answer::Choice(TYPE_NAME.to_owned()));
    let name = question(create(&with_format("x"), &typed));
    assert_eq!((name.key, name.title.as_str()), (NAME, "Create branch"));
}

#[test]
fn branch_create__with_the_format_the_name_it_makes_is_proposed_and_the_inputs_are_remembered_when_it_runs() {
    let context = with_format("${input:ticket}-${base}");
    let answers = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
    assert_eq!(question(create(&context, &answers)).key, INPUTS[0]);
    let answers = answers.with(INPUTS[0], Answer::Text(String::from("12")));
    let name = question(create(&context, &answers));
    assert_eq!(name.title.as_str(), "Name of the new branch");
    assert_eq!(name.default, Some(Answer::Text(String::from("12-main"))));
    let plan = plan(create(
        &context,
        &none(
            answers
                .with(NAME, Answer::Text(String::from("12-main")))
                .with(MODE, Answer::Choice(CHECK_OUT.to_owned())),
        ),
    ));
    assert_eq!(lines(&plan), ["git checkout -b 12-main HEAD"]);
    let remembered: BTreeMap<String, String> = [(String::from("ticket"), String::from("12"))].into();
    assert_eq!(plan.after, [FollowUp::SelectHead, FollowUp::RememberInputs(remembered)]);
}

#[test]
fn branch_create__the_date_of_the_format_is_today_in_local_time() {
    let mut context = with_format("${date}");
    // 23:00 on 1 January 1970 in UTC is already 2 January two hours east of it.
    context.now = Timestamp(86_400 - 3_600);
    context.settings.utc_offset = 7_200;
    let answers = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
    let name = question(create(&context, &answers));
    assert_eq!(name.default, Some(Answer::Text(String::from("1970-01-02"))));
}
