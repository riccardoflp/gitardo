//! `worktree.open-in-worktree`: works on a branch in its own worktree, with nothing asked: the worktree is made if it
//! is not there yet, and opens.

use model::branch::{local_name, other_worktree};
use model::contract::{Answers, Context, Outcome, Refusal, Target};
use model::worktree::folder_in;

use super::add::{Addition, add, created};
use super::{folder_of_worktrees, label, opening};

/// Opens the worktree of the branch invoked on, a local branch or the local branch of a remote branch, where
/// `worktrees.openIn` says; when no worktree has it, makes one first in the folder named after it, `git worktree add
/// <folder> <branch>`, or `git worktree add --track -b <branch> <folder> <remote>/<branch>` for a remote branch with
/// no local branch of its name, followed by what new worktrees get. It opens once all that succeeded; when a command of
/// the setup fails, opening is offered all the same. Nothing is asked.
///
/// Feature: worktree.open-in-worktree
#[must_use]
pub fn open_in_worktree(context: &Context, answers: &Answers) -> Outcome {
    let state = &context.repository;
    let (local, remote) = match &context.target {
        Target::Branch(name) => (name.as_str(), None),
        Target::RemoteBranch(name) => (local_name(name, &state.remotes), Some(name.as_str())),
        _ => return Outcome::Refuse(Refusal::saying("Choose a branch to open in a worktree.")), // Only branches have one.
    };
    let local_branch = state.branches.iter().find(|branch| branch.name == local);
    let worktree = other_worktree(state, local);
    if worktree.is_none() && local_branch.is_some_and(|branch| branch.is_checked_out) {
        return Outcome::Refuse(Refusal::saying(&format!("`{local}` is checked out in this window.")));
    }
    if let Some(worktree) = worktree {
        if worktree.is_missing {
            return Outcome::Refuse(Refusal::saying(&format!(
                "The worktree of `{local}` is missing from `{}`: prune the worktrees to free the branch.",
                worktree.folder.display()
            )));
        }
        return Outcome::Run(opening::where_set(
            context,
            &worktree.folder,
            &label(context, &worktree.folder),
        ));
    }
    let folder = folder_in(&folder_of_worktrees(context), local);
    let path = folder.to_string_lossy().into_owned();
    let (arguments, start) = match remote.filter(|_| local_branch.is_none()) {
        Some(remote) => (
            vec![
                String::from("--track"),
                String::from("-b"),
                local.to_owned(),
                path,
                remote.to_owned(),
            ],
            remote.to_owned(),
        ),
        None => (vec![path, local.to_owned()], local.to_owned()),
    };
    let addition = Addition {
        title: format!("Open {local} in a worktree"),
        arguments,
        folder: folder.clone(),
        start,
    };
    let (mut plan, setup) = match add(context, answers, addition) {
        Ok(made) => made,
        Err(outcome) => return *outcome,
    };
    let opened = opening::where_set(context, &folder, &label(context, &folder));
    plan.steps.extend(opened.steps);
    plan.after = opened.after;
    plan.on_failure = created(context, &folder, &setup).1;
    plan.on_failure.extend(opened.on_failure);
    Outcome::Run(plan)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::sync::Arc;

    use model::Timestamp;
    use model::contract::{Answers, Context, FollowUp, Outcome, Target};
    use model::state::RemoteBranch;

    use super::open_in_worktree;
    use crate::testing::{commit, lines, plan};
    use crate::worktree::add::not_there;
    use crate::worktree::testing::{LOGIN, repository, with_worktrees};

    /// The repository of the tests with `origin/fix`, and a local `fix` when asked.
    fn with_fix(has_local: bool) -> Context {
        let mut state = repository();
        state.remotes = vec![String::from("origin")];
        state.remote_branches = vec![RemoteBranch {
            name: String::from("origin/fix"),
            remote: String::from("origin"),
            commit: commit('c'),
            date: Timestamp(0),
        }];
        if has_local {
            state.branches.push(crate::testing::branch("fix", 'c', false));
        }
        let mut context = with_worktrees(Target::RemoteBranch(String::from("origin/fix")));
        context.repository = Arc::new(state);
        context
    }

    #[test]
    fn worktree_open_in_worktree__a_branch_no_worktree_has_is_added_and_opened_with_no_question() {
        let context = with_worktrees(Target::Branch(String::from("release")));
        assert!(matches!(open_in_worktree(&context, &Answers::new()), Outcome::Read(..)));
        let plan = plan(open_in_worktree(&context, &not_there()));
        assert_eq!(lines(&plan), ["git worktree add /code/app.worktrees/release release"]);
        assert_eq!(plan.after, [FollowUp::OpenWindow("/code/app.worktrees/release".into())]);
    }

    #[test]
    fn worktree_open_in_worktree__a_remote_branch_with_no_local_one_makes_it_tracking_the_remote() {
        let plan = plan(open_in_worktree(&with_fix(false), &not_there()));
        assert_eq!(
            lines(&plan),
            ["git worktree add --track -b fix /code/app.worktrees/fix origin/fix"]
        );
    }

    #[test]
    fn worktree_open_in_worktree__a_remote_branch_with_a_local_one_puts_the_local_one() {
        let plan = plan(open_in_worktree(&with_fix(true), &not_there()));
        assert_eq!(lines(&plan), ["git worktree add /code/app.worktrees/fix fix"]);
    }

    #[test]
    fn worktree_open_in_worktree__the_branch_checked_out_here_is_refused_so_the_window_stays_on_it() {
        let context = with_worktrees(Target::Branch(String::from("main")));
        assert!(matches!(
            open_in_worktree(&context, &Answers::new()),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn worktree_open_in_worktree__a_branch_with_a_worktree_opens_it_and_creates_nothing() {
        let context = with_worktrees(Target::Branch(String::from("feature/login")));
        let plan = plan(open_in_worktree(&context, &Answers::new()));
        assert_eq!(plan.steps, []);
        assert_eq!(plan.after, [FollowUp::OpenWindow(LOGIN.into())]);
    }

    #[test]
    fn a_missing_worktree_is_refused_with_the_prune_message() {
        let context = with_worktrees(Target::Branch(String::from("gone")));
        let Outcome::Refuse(refusal) = open_in_worktree(&context, &Answers::new()) else {
            panic!()
        };
        assert!(refusal.message.as_str().contains("prune the worktrees"));
    }
}
