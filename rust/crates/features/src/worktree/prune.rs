//! `worktree.prune`: has git forget the worktrees whose folders are gone.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Text, Undo};

/// `git worktree prune`, while a worktree is missing: git forgets the worktrees whose folders are gone, which frees
/// their branches.
///
/// Feature: worktree.prune
#[must_use]
pub fn prune(context: &Context, _answers: &Answers) -> Outcome {
    if !context.repository.worktrees.iter().any(|worktree| worktree.is_missing) {
        return Outcome::Refuse(Refusal::saying("No worktree is missing: there is nothing to prune."));
    }
    Outcome::Run(Plan {
        title: Text::from("Prune missing worktrees"),
        steps: vec![Step::git(context.repository.folder.clone(), ["worktree", "prune"])],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: git forgets only worktrees whose folders are gone.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use model::contract::{Answers, Outcome, Target};

    use super::prune;
    use crate::testing::{context, lines, plan, state};
    use crate::worktree::testing::with_worktrees;

    #[test]
    fn prunes_while_a_worktree_is_missing_and_refuses_otherwise() {
        let plan = plan(prune(&with_worktrees(Target::Head), &Answers::new()));
        assert_eq!(lines(&plan), ["git worktree prune"]);
        assert!(matches!(
            prune(&context(state(), Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
