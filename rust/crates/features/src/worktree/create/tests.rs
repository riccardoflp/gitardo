use std::path::PathBuf;

use model::Timestamp;
use model::contract::{
    Answer, Answers, Context, FeatureId, FollowUp, Outcome, Question, QuestionKind, Step, Target, Validation,
};

use super::{NAME, START, TYPED, WAY, create};
use crate::testing::{branch, context, lines, plan, question, state};
use crate::worktree::add::{created, not_there};
use crate::worktree::names::USE_FORMAT;
use crate::worktree::setup::Setup;
use crate::worktree::testing::MAIN;

/// A repository in `/code/app` on `main`, with `feature/x` and `release`.
fn app() -> Context {
    let mut context = context(
        model::state::RepositoryState {
            folder: PathBuf::from(MAIN),
            branches: vec![
                branch("main", 'a', true),
                branch("feature/x", 'b', false),
                branch("release", 'c', false),
            ],
            ..state()
        },
        Target::Head,
    );
    context.settings.worktrees.home = PathBuf::from("/home/someone");
    context
}

/// The validation of the box of the name.
fn validation(question: &Question) -> &Validation {
    let QuestionKind::Text { validation, .. } = &question.kind else {
        panic!("{question:?}")
    };
    validation
}

fn named(start: &str, name: &str) -> Answers {
    not_there()
        .with(START, Answer::Choice(start.to_owned()))
        .with(NAME, Answer::Text(name.to_owned()))
}

/// The features a plan offers once it ran, by identifier.
fn offered(after: &[FollowUp]) -> Vec<&'static str> {
    after
        .iter()
        .filter_map(|follow_up| match follow_up {
            FollowUp::OfferRun(offer) => Some(offer.feature.0),
            _ => None,
        })
        .collect()
}

#[test]
fn worktree_create__named_from_main_it_adds_the_worktree_with_no_track_and_the_box_said_where() {
    let context = app();
    let asked = question(create(
        &context,
        &Answers::new().with(START, Answer::Choice(String::from("main"))),
    ));
    let hint = validation(&asked).hint("feature/login").unwrap();
    assert_eq!(hint.as_str(), "In `/code/app.worktrees/feature/login`");
    let plan = plan(create(&context, &named("main", "feature/login")));
    assert_eq!(
        lines(&plan),
        ["git worktree add --no-track -b feature/login /code/app.worktrees/feature/login main"]
    );
}

#[test]
fn worktree_create__a_remote_start_gives_the_new_branch_no_upstream() {
    let mut context = app();
    let mut state = (*context.repository).clone();
    state.remote_branches = vec![model::state::RemoteBranch {
        name: String::from("origin/release"),
        remote: String::from("origin"),
        commit: crate::testing::commit('c'),
        date: Timestamp(0),
    }];
    state.remotes = vec![String::from("origin")];
    context.repository = std::sync::Arc::new(state);
    let asked = question(create(&context, &Answers::new()));
    let QuestionKind::Pick { choices } = &asked.kind else {
        panic!()
    };
    let remote = choices
        .iter()
        .find(|choice| choice.identifier == "origin/release")
        .unwrap();
    assert_eq!(remote.description.as_ref().unwrap().as_str(), "Remote");
    let plan = plan(create(&context, &named("origin/release", "try")));
    assert_eq!(
        lines(&plan),
        ["git worktree add --no-track -b try /code/app.worktrees/try origin/release"]
    );
}

#[test]
fn worktree_create__a_branch_there_and_one_beside_a_branch_are_refused_while_typed() {
    let context = app();
    let asked = question(create(
        &context,
        &Answers::new().with(START, Answer::Choice(String::from("main"))),
    ));
    let validation = validation(&asked);
    assert_eq!(
        validation.problem("release").unwrap().as_str(),
        "A branch with that name already exists"
    );
    assert_eq!(
        validation.problem("feature").unwrap().as_str(),
        "Git cannot have it beside the branch `feature/x`"
    );
    assert_eq!(
        validation.problem("feature/x/y").unwrap().as_str(),
        "Git cannot have it beside the branch `feature/x`"
    );
    assert_eq!(
        validation.problem("main").unwrap().as_str(),
        "A branch with that name already exists"
    );
    assert!(validation.problem("fix login").is_some(), "git takes no space");
    assert_eq!(validation.problem("feature-y"), None);
}

#[test]
fn worktree_create__with_a_format_the_name_is_proposed_and_can_be_changed() {
    let mut context = app();
    context.settings.branch_format = String::from("${base}-${date}");
    context.now = Timestamp(1_792_065_600);
    let way = question(create(&context, &Answers::new()));
    assert_eq!(way.key, WAY);
    assert_eq!(way.default, Some(Answer::Choice(USE_FORMAT.to_owned())));
    let answers = Answers::new()
        .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
        .with(START, Answer::Choice(String::from("main")));
    let name = question(create(&context, &answers));
    assert_eq!(name.default, Some(Answer::Text(String::from("main-2026-10-15"))));
    let changed = not_there()
        .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
        .with(START, Answer::Choice(String::from("main")))
        .with(NAME, Answer::Text(String::from("main-spike")));
    let plan = plan(create(&context, &changed));
    assert!(lines(&plan)[0].contains("-b main-spike "), "{:?}", lines(&plan));
    let typed = Answers::new()
        .with(WAY, Answer::Choice(TYPED.to_owned()))
        .with(START, Answer::Choice(String::from("main")));
    assert_eq!(question(create(&context, &typed)).default, None);
}

#[test]
fn worktree_create__the_first_worktree_with_nothing_set_offers_choose_what_it_gets_and_the_next_does_not() {
    let mut context = app();
    let plan_of = |context: &Context| plan(create(context, &named("main", "fix")));
    assert!(offered(&plan_of(&context).after).contains(&"worktree-setup.page"));
    context.settings.worktrees.setup_offered = true;
    assert!(!offered(&plan_of(&context).after).contains(&"worktree-setup.page"));
    let set = Setup {
        is_set: true,
        ..Setup::default()
    };
    context.settings.worktrees.setup_offered = false;
    let (after, _) = created(&context, &PathBuf::from("/code/app.worktrees/fix"), &set);
    assert!(!offered(&after).contains(&"worktree-setup.page"));
}

#[test]
fn worktree_create__with_commands_set_opening_is_offered_only_once_the_last_command_succeeded() {
    let context = app();
    let setup = Setup {
        steps: vec![Step::Task {
            command: String::from("npm ci"),
            directory: PathBuf::from("/code/app.worktrees/fix"),
            name: String::from("fix"),
        }],
        has_commands: true,
        is_set: true,
        said: None,
        trusts: None,
    };
    let (after, on_failure) = created(&context, &PathBuf::from("/code/app.worktrees/fix"), &setup);
    // What follows a plan comes only once all its steps, the commands last, succeeded.
    assert_eq!(offered(&after)[0], "worktree.open-in-new-window");
    // When a command fails, the worktree can be opened all the same.
    assert_eq!(offered(&on_failure)[0], "worktree.open-in-new-window");
    let made = plan(create(&context, &named("main", "fix")));
    assert_eq!(made.on_failure, [], "with no command, a failure offers nothing to open");
}

#[test]
fn worktree_create__backing_out_of_any_question_runs_nothing() {
    let mut context = app();
    context.settings.branch_format = String::from("${base}-x");
    let stages = [
        Answers::new(),
        Answers::new().with(WAY, Answer::Choice(TYPED.to_owned())),
        Answers::new()
            .with(WAY, Answer::Choice(TYPED.to_owned()))
            .with(START, Answer::Choice(String::from("main"))),
    ];
    for answers in stages {
        assert!(matches!(create(&context, &answers), Outcome::Ask(_)), "{answers:?}");
    }
    let refused = create(
        &context,
        &crate::worktree::add::there()
            .with(WAY, Answer::Choice(TYPED.to_owned()))
            .with(START, Answer::Choice(String::from("main")))
            .with(NAME, Answer::Text(String::from("fix"))),
    );
    assert!(
        matches!(refused, Outcome::Refuse(_)),
        "a folder there by then is refused"
    );
    let _ = FeatureId("worktree.create");
}

#[test]
fn worktree_create__a_format_with_inputs_asks_them_after_the_start_and_keeps_them() {
    let mut context = app();
    context.settings.branch_format = String::from("PRJ-${input:ticket}-${base}");
    context.settings.branch_inputs = vec![model::branch::BranchInput {
        id: String::from("ticket"),
        kind: model::branch::InputKind::PromptString,
        description: Some(String::from("Ticket")),
        default: None,
        pattern: None,
        options: Vec::new(),
        allow_custom: false,
    }];
    let Some(&ticket) = model::branch::INPUTS.first() else {
        panic!("the key of the first input");
    };
    let way = question(create(&context, &Answers::new()));
    assert_eq!(way.key, WAY, "a format with an input is offered");
    let started = Answers::new()
        .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
        .with(START, Answer::Choice(String::from("release")));
    assert_eq!(question(create(&context, &started)).key, ticket);
    let typed = started.with(ticket, Answer::Text(String::from("7")));
    let name = question(create(&context, &typed));
    assert_eq!(name.default, Some(Answer::Text(String::from("PRJ-7-release"))));
    let plan = plan(create(
        &context,
        &not_there()
            .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
            .with(START, Answer::Choice(String::from("release")))
            .with(ticket, Answer::Text(String::from("7")))
            .with(NAME, Answer::Text(String::from("PRJ-7-release"))),
    ));
    let remembered = plan.after.iter().any(|follow_up| {
        matches!(follow_up, FollowUp::RememberInputs(values) if values.get("ticket").map(String::as_str) == Some("7"))
    });
    assert!(remembered, "{:?}", plan.after);
}

#[test]
fn worktree_create__from_a_detached_head_the_base_of_the_format_is_its_short_hash() {
    let mut context = app();
    let mut state = (*context.repository).clone();
    state.head.branch = None;
    for branch in &mut state.branches {
        branch.is_checked_out = false;
    }
    context.repository = std::sync::Arc::new(state);
    context.settings.branch_format = String::from("${base}-spike");
    let answers = Answers::new()
        .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
        .with(START, Answer::Choice(String::from("HEAD")));
    let short = crate::testing::commit('a').short().to_owned();
    assert_eq!(
        question(create(&context, &answers)).default,
        Some(Answer::Text(format!("{short}-spike")))
    );
}
