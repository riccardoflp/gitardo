//! `worktree.show-in-graph`: selects in the timeline the commit a worktree has checked out.

use model::contract::{Answers, Context, FollowUp, Outcome, Refusal};

use super::{label, no_steps, refuse_missing, target_worktree};

/// Selects in the timeline the commit the worktree invoked on has checked out, going to it as `graph.show-in-graph`
/// does: the timeline loads more of the history when it is not listed yet.
///
/// Feature: worktree.show-in-graph
#[must_use]
pub fn show_in_graph(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_missing(worktree) {
        return refusal;
    }
    let Some(commit) = worktree.commit.clone() else {
        return Outcome::Refuse(Refusal::saying("This worktree has no commit yet."));
    };
    Outcome::Run(no_steps(
        format!("Show the commit of the worktree {}", label(context, &worktree.folder)),
        vec![FollowUp::SelectCommit(commit)],
    ))
}

#[cfg(test)]
mod tests {
    use model::contract::{Answers, FollowUp, Target};

    use super::show_in_graph;
    use crate::testing::{commit, plan};
    use crate::worktree::testing::{LOGIN, with_worktrees};

    #[test]
    fn selects_the_commit_of_the_worktree() {
        let context = with_worktrees(Target::Worktree(LOGIN.into()));
        let plan = plan(show_in_graph(&context, &Answers::new()));
        assert_eq!(plan.after, [FollowUp::SelectCommit(commit('b'))]);
    }
}
