use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, Danger, FollowUp, Outcome, Query, Step, Target};

use super::{BRANCH, DELETE_BRANCH, FORCE, KEEP_BRANCH, LINKS, remove};
use crate::testing::{lines, plan};
use crate::worktree::testing::{LOGIN, MAIN, with_worktrees};

fn on_login() -> Context {
    with_worktrees(Target::Worktree(LOGIN.into()))
}

/// The answers, with no links read on Windows.
fn with(choice: &str) -> Answers {
    Answers::new()
        .with(BRANCH, Answer::Choice(choice.to_owned()))
        .with(LINKS, Answer::Paths(Vec::new()))
}

/// The steps that are git's, without those that delete the folders left empty.
fn git_lines(plan: &model::contract::Plan) -> Vec<String> {
    lines(plan)
        .into_iter()
        .filter(|line| line.starts_with("git "))
        .collect()
}

#[test]
fn worktree_remove__deleting_the_branch_removes_the_worktree_then_the_branch() {
    let plan = plan(remove(&on_login(), &with(DELETE_BRANCH)));
    assert_eq!(
        git_lines(&plan),
        [
            "git worktree remove /code/app.worktrees/feature/login",
            "git branch -d feature/login"
        ]
    );
    assert!(matches!(&plan.danger, Danger::Confirm(said) if said.as_str().contains("if it is merged")));
}

#[test]
fn worktree_remove__a_refusal_for_changes_offers_force_as_destructive_and_nothing_more() {
    let kept = plan(remove(&on_login(), &with(KEEP_BRANCH)));
    let [FollowUp::OfferRun(offer)] = kept.on_failure.as_slice() else {
        panic!("{:?}", kept.on_failure)
    };
    assert_eq!(offer.label.as_str(), "Remove anyway");
    assert!(offer.when_said.iter().any(|said| said == "modified or untracked"));
    assert_eq!(offer.target, Target::Worktree(LOGIN.into()));
    let answers = offer.answers.clone().with(LINKS, Answer::Paths(Vec::new()));
    assert_eq!(answers.flag(FORCE), Some(true));
    let forced = plan(remove(&on_login(), &answers));
    assert_eq!(
        git_lines(&forced)[0],
        "git worktree remove --force /code/app.worktrees/feature/login"
    );
    assert!(matches!(forced.danger, Danger::Destructive(_)));
    assert_eq!(forced.on_failure, []);
}

#[test]
fn worktree_remove__the_branch_is_deleted_last_with_d_so_git_keeps_one_not_merged() {
    let plan = plan(remove(&on_login(), &with(DELETE_BRANCH)));
    assert_eq!(lines(&plan).last().unwrap(), "git branch -d feature/login");
}

#[test]
fn worktree_remove__the_links_to_outside_folders_are_unlinked_before_git_removes_it() {
    let context = on_login();
    let asked = remove(
        &context,
        &Answers::new().with(BRANCH, Answer::Choice(KEEP_BRANCH.to_owned())),
    );
    if cfg!(windows) {
        assert!(
            matches!(asked, Outcome::Read(LINKS, Query::FolderLinks { .. })),
            "{asked:?}"
        );
    }
    let link = PathBuf::from(format!("{LOGIN}/node_modules"));
    let answers = with(KEEP_BRANCH).with(LINKS, Answer::Paths(vec![link.clone()]));
    let plan = plan(remove(&context, &answers));
    if cfg!(windows) {
        assert_eq!(plan.steps[0], Step::Unlink { path: link });
        assert_eq!(lines(&plan)[1], "git worktree remove /code/app.worktrees/feature/login");
    }
}

#[test]
fn worktree_remove__the_only_worktree_in_a_folder_leaves_that_folder_deleted() {
    let plan = plan(remove(&on_login(), &with(KEEP_BRANCH)));
    assert_eq!(
        lines(&plan),
        [
            "git worktree remove /code/app.worktrees/feature/login",
            "delete /code/app.worktrees/feature"
        ]
    );
}

#[test]
fn is_refused_on_the_main_worktree_and_on_the_folder_of_the_window() {
    assert!(matches!(
        remove(&with_worktrees(Target::Worktree(MAIN.into())), &Answers::new()),
        Outcome::Refuse(_)
    ));
    let mut context = on_login();
    context.settings.worktrees.window_folder = Some(LOGIN.into());
    assert!(matches!(remove(&context, &Answers::new()), Outcome::Refuse(_)));
}
