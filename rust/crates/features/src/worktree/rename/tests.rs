use std::sync::Arc;

use model::contract::{Answer, Answers, Context, Outcome, Target};

use super::{BRANCH_NAME, KEEP, NEW_NAME, RENAME, RENAME_BRANCH, rename};
use crate::testing::{lines, plan, question};
use crate::worktree::testing::{LOGIN, MAIN, repository, with_worktrees, worktree};

fn on_login() -> Context {
    with_worktrees(Target::Worktree(LOGIN.into()))
}

fn renamed(name: &str) -> Answers {
    Answers::new().with(NEW_NAME, Answer::Text(name.to_owned()))
}

#[test]
fn worktree_rename__with_its_branch_it_moves_the_folder_deletes_the_empty_ones_then_renames_the_branch_proposed_from_the_folder()
 {
    let context = on_login();
    let asked = question(rename(&context, &Answers::new()));
    assert_eq!(asked.default, Some(Answer::Text(String::from("feature/login"))));
    let answers = renamed("feature/signin").with(RENAME_BRANCH, Answer::Choice(RENAME.to_owned()));
    let branch_name = question(rename(&context, &answers));
    assert_eq!(branch_name.key, BRANCH_NAME);
    assert_eq!(branch_name.default, Some(Answer::Text(String::from("feature/signin"))));
    let plan = plan(rename(
        &context,
        &answers.with(BRANCH_NAME, Answer::Text(String::from("feature/signin"))),
    ));
    // The folders left empty go before the branch, so that a rename git refuses leaves none behind.
    assert_eq!(
        lines(&plan),
        [
            "git worktree move /code/app.worktrees/feature/login /code/app.worktrees/feature/signin",
            "delete /code/app.worktrees/feature",
            "git branch -m feature/login feature/signin",
        ]
    );
}

#[test]
fn worktree_rename__to_another_folder_it_makes_the_folder_and_deletes_the_one_left_empty() {
    let answers = renamed("hotfix/login").with(RENAME_BRANCH, Answer::Choice(KEEP.to_owned()));
    let plan = plan(rename(&on_login(), &answers));
    assert_eq!(
        lines(&plan),
        [
            "mkdir /code/app.worktrees/hotfix",
            "git worktree move /code/app.worktrees/feature/login /code/app.worktrees/hotfix/login",
            "delete /code/app.worktrees/feature",
        ]
    );
}

#[test]
fn worktree_rename__outside_the_folder_of_the_worktrees_a_slash_becomes_a_dash_beside_it() {
    let mut state = repository();
    state
        .worktrees
        .push(worktree("/elsewhere/copy", None, (false, false, false)));
    let mut context = on_login();
    context.repository = Arc::new(state);
    context.target = Target::Worktree("/elsewhere/copy".into());
    let asked = question(rename(&context, &Answers::new()));
    assert_eq!(asked.default, Some(Answer::Text(String::from("copy"))));
    let plan = plan(rename(&context, &renamed("a/b")));
    assert_eq!(lines(&plan), ["git worktree move /elsewhere/copy /elsewhere/a-b"]);
}

#[test]
fn worktree_rename__is_refused_on_the_main_worktree_and_on_the_folder_of_the_window() {
    let mut on_main = with_worktrees(Target::Worktree(MAIN.into()));
    let said = |outcome: Outcome| match outcome {
        Outcome::Refuse(refusal) => refusal.message.as_str().to_owned(),
        other => panic!("{other:?}"),
    };
    assert!(said(rename(&on_main, &Answers::new())).starts_with("Git cannot move the main worktree"));
    on_main.target = Target::Worktree(LOGIN.into());
    on_main.settings.worktrees.window_folder = Some(LOGIN.into());
    assert!(said(rename(&on_main, &Answers::new())).starts_with("This worktree is the folder open in this window"));
}

#[test]
fn the_same_name_runs_nothing() {
    assert!(matches!(
        rename(&on_login(), &renamed("feature/login")),
        Outcome::Refuse(_)
    ));
}
