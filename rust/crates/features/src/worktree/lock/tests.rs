use model::contract::{Answer, Answers, Context, Outcome, Target};

use super::{REASON, UNLOCK, UNLOCK_CHOICE, lock, unlock};
use crate::testing::{context, lines, plan, question};
use crate::worktree::rename::NEW_NAME;
use crate::worktree::testing::{GONE, LOGIN, MAIN, repository, with_worktrees};
use crate::worktree::{BRANCH_NAME, KEEP, KEEP_BRANCH, RENAME_BRANCH, prune, remove, rename};

/// The repository of the tests, with the worktree of `folder` locked for `reason`.
fn locked(folder: &str, reason: &str, target: &str) -> Context {
    let mut state = repository();
    for worktree in &mut state.worktrees {
        if worktree.folder.to_string_lossy() == folder {
            worktree.locked = Some(reason.to_owned());
        }
    }
    let mut context = context(state, Target::Worktree(target.into()));
    context.settings.worktrees.home = "/home/someone".into();
    context
}

fn reason(reason: &str) -> Answers {
    Answers::new().with(REASON, Answer::Text(reason.to_owned()))
}

#[test]
fn worktree_lock__the_reason_is_asked_then_given_to_git_lock() {
    let on_login = with_worktrees(Target::Worktree(LOGIN.into()));
    let asked = question(lock(&on_login, &Answers::new()));
    assert_eq!(asked.key, REASON);
    assert_eq!(asked.title.as_str(), "Why feature/login is locked");
    let plan = plan(lock(&on_login, &reason("On the USB drive")));
    assert_eq!(
        lines(&plan),
        ["git worktree lock --reason \"On the USB drive\" /code/app.worktrees/feature/login"]
    );
}

#[test]
fn worktree_lock__with_the_reason_left_empty_it_runs_lock_without_reason() {
    let on_login = with_worktrees(Target::Worktree(LOGIN.into()));
    let plan = plan(lock(&on_login, &reason("  ")));
    assert_eq!(lines(&plan), ["git worktree lock /code/app.worktrees/feature/login"]);
}

#[test]
fn worktree_lock__the_main_worktree_and_a_worktree_locked_already_are_refused() {
    let on_main = with_worktrees(Target::Worktree(MAIN.into()));
    assert!(matches!(lock(&on_main, &reason("x")), Outcome::Refuse(_)));
    assert!(matches!(
        lock(&locked(LOGIN, "", LOGIN), &reason("x")),
        Outcome::Refuse(_)
    ));
    let missing = with_worktrees(Target::Worktree(GONE.into()));
    assert!(
        matches!(lock(&missing, &reason("")), Outcome::Run(_)),
        "a missing worktree may be locked"
    );
}

#[test]
fn worktree_unlock__a_locked_worktree_is_unlocked_and_one_not_locked_refused() {
    let plan = plan(unlock(&locked(LOGIN, "On the USB drive", LOGIN), &Answers::new()));
    assert_eq!(lines(&plan), ["git worktree unlock /code/app.worktrees/feature/login"]);
    let on_login = with_worktrees(Target::Worktree(LOGIN.into()));
    assert!(matches!(unlock(&on_login, &Answers::new()), Outcome::Refuse(_)));
}

#[test]
fn worktree_locked__remove_on_a_locked_worktree_says_its_reason_then_unlocks_before_removing() {
    let context = locked(LOGIN, "On the USB drive", LOGIN);
    let asked = question(remove(&context, &Answers::new()));
    assert_eq!(asked.key, UNLOCK);
    assert_eq!(asked.title.as_str(), "feature/login is locked: On the USB drive.");
    let answers = Answers::new()
        .with(UNLOCK, Answer::Choice(UNLOCK_CHOICE.to_owned()))
        .with(crate::worktree::BRANCH, Answer::Choice(KEEP_BRANCH.to_owned()))
        .with(model::contract::AnswerKey("folder links"), Answer::Paths(Vec::new()));
    let git: Vec<String> = lines(&plan(remove(&context, &answers)))
        .into_iter()
        .filter(|line| line.starts_with("git "))
        .collect();
    assert_eq!(
        git,
        [
            "git worktree unlock /code/app.worktrees/feature/login",
            "git worktree remove /code/app.worktrees/feature/login"
        ]
    );
}

#[test]
fn worktree_locked__rename_on_a_locked_worktree_unlocks_moves_and_locks_the_new_folder_again() {
    let context = locked(LOGIN, "On the USB drive", LOGIN);
    assert_eq!(question(rename(&context, &Answers::new())).key, UNLOCK);
    let answers = Answers::new()
        .with(UNLOCK, Answer::Choice(UNLOCK_CHOICE.to_owned()))
        .with(NEW_NAME, Answer::Text("feature/signin".to_owned()))
        .with(RENAME_BRANCH, Answer::Choice(KEEP.to_owned()))
        .with(BRANCH_NAME, Answer::Text(String::new()));
    let git: Vec<String> = lines(&plan(rename(&context, &answers)))
        .into_iter()
        .filter(|line| line.starts_with("git "))
        .collect();
    assert_eq!(
        git,
        [
            "git worktree unlock /code/app.worktrees/feature/login",
            "git worktree move /code/app.worktrees/feature/login /code/app.worktrees/feature/signin",
            "git worktree lock --reason \"On the USB drive\" /code/app.worktrees/feature/signin"
        ]
    );
}

#[test]
fn worktree_locked__prune_is_refused_while_the_only_missing_worktree_is_locked() {
    let context = locked(GONE, "On the USB drive", MAIN);
    assert!(matches!(prune(&context, &Answers::new()), Outcome::Refuse(_)));
    let unlocked = with_worktrees(Target::Head);
    assert_eq!(lines(&plan(prune(&unlocked, &Answers::new()))), ["git worktree prune"]);
}
