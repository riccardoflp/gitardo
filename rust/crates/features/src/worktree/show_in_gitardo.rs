//! `worktree.show-in-gitardo`: points the views of this window at another worktree.

use model::contract::{Answers, Context, FollowUp, Outcome};

use super::{label, no_steps, refuse_missing, refuse_shown, target_worktree};

/// Points the timeline with its draft, the branch rail and the Changes page of this window at the worktree invoked on,
/// or at the worktree that has the branch invoked on, so that what is done there is done in it. The worktree the
/// views show, and a missing one, are not shown again.
///
/// Feature: worktree.show-in-gitardo
#[must_use]
pub fn show_in_gitardo(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_shown(worktree).or_else(|| refuse_missing(worktree)) {
        return refusal;
    }
    Outcome::Run(no_steps(
        format!("Show the worktree {}", label(context, &worktree.folder)),
        vec![FollowUp::ShowFolder(worktree.folder.clone())],
    ))
}

#[cfg(test)]
mod tests {
    use model::contract::{Answers, FollowUp, Outcome, Target};

    use super::show_in_gitardo;
    use crate::testing::plan;
    use crate::worktree::testing::{LOGIN, MAIN, with_worktrees};

    #[test]
    fn shows_the_worktree_of_a_branch_and_not_the_one_shown() {
        let context = with_worktrees(Target::Branch(String::from("feature/login")));
        let plan = plan(show_in_gitardo(&context, &Answers::new()));
        assert_eq!(plan.after, [FollowUp::ShowFolder(LOGIN.into())]);
        let context = with_worktrees(Target::Worktree(MAIN.into()));
        assert!(matches!(show_in_gitardo(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
