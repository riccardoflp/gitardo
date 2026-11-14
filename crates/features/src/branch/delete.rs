//! `branch.delete`: deletes a local branch, after saying whether its commits are merged, so that none is lost without
//! knowing.

use model::branch::other_worktree;
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Query, Step, Target,
    Text, Undo, UndoRecord,
};

use super::target::{local_branch, refuse};

/// Whether the commits of the branch are merged into its upstream, or into `HEAD` when it has none.
pub const MERGED: AnswerKey = AnswerKey("merged");

/// What git says when `git branch -d` keeps a branch whose commits it finds unmerged.
const NOT_MERGED_SAID: &str = "not fully merged";

/// Deletes the local branch it was invoked on: with `git branch -d` when its commits are merged into its upstream, or
/// into `HEAD` when it has none, after a confirmation; with `git branch -D` otherwise, after a warning. Gitardo keeps
/// the branch and its commit, so that Undo right after creates it again. When git still keeps the branch, saying it is
/// not fully merged, **Delete anyway** offers the plan with `-D`.
///
/// Feature: branch.delete
#[must_use]
pub fn delete(context: &Context, answers: &Answers) -> Outcome {
    let branch = match local_branch(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let name = branch.name.as_str();
    if branch.is_checked_out {
        return refuse(&format!("{name} is checked out: check out another branch first."));
    }
    if let Some(worktree) = other_worktree(&context.repository, name) {
        let folder = worktree.folder.display();
        return refuse(&format!(
            "{name} is checked out in the worktree {folder}: remove the worktree first."
        ));
    }
    let Some(is_merged) = answers.flag(MERGED) else {
        let into = branch
            .upstream
            .as_ref()
            .filter(|upstream| !upstream.is_gone)
            .map_or_else(|| String::from("HEAD"), |upstream| upstream.reference.clone());
        return Outcome::Read(
            MERGED,
            Query::IsAncestor {
                ancestor: format!("refs/heads/{name}"),
                descendant: into,
            },
        );
    };
    let (flag, danger) = if is_merged {
        let reason = "Its commits are merged, so none is lost. Undo brings the branch back.";
        ("-d", Danger::Confirm(Text::from(reason)))
    } else {
        let reason = "This branch is not fully merged. Commits that are only on it will no longer be reachable from \
                      any branch.";
        ("-D", Danger::Destructive(Text::from(reason)))
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Delete branch {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["branch", flag, name])],
        danger,
        undo: Undo::Journal(UndoRecord::BranchDeleted {
            name: name.to_owned(),
            commit: branch.commit.clone(),
        }),
        after: Vec::new(),
        on_failure: if is_merged { delete_anyway(name) } else { Vec::new() },
    })
}

/// **Delete anyway**, offered when `git branch -d` keeps the branch because it is not fully merged: the same feature,
/// told the branch is not merged, so that it plans `git branch -D` with its warning.
fn delete_anyway(name: &str) -> Vec<FollowUp> {
    vec![FollowUp::OfferRun(Box::new(OfferedRun {
        label: Text::from("Delete anyway"),
        feature: FeatureId("branch.delete"),
        target: Target::Branch(name.to_owned()),
        answers: Answers::new().with(MERGED, Answer::Flag(false)),
        when_said: vec![NOT_MERGED_SAID.to_owned()],
    }))]
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MERGED, delete};
    use crate::testing::{branch, commit, context, lines, plan, state};
    use model::contract::{Answer, Answers, Danger, FollowUp, Outcome, Query, Target, Undo, UndoRecord};
    use model::state::Upstream;

    fn on_old(has_upstream: bool) -> model::contract::Context {
        let mut state = state();
        let mut old = branch("old", 'b', false);
        if has_upstream {
            old.upstream = Some(Upstream {
                reference: String::from("refs/remotes/origin/old"),
                remote: String::from("origin"),
                remote_branch: String::from("refs/heads/old"),
                ahead: 0,
                behind: 0,
                is_gone: false,
            });
        }
        state.branches.push(old);
        context(state, Target::Branch(String::from("old")))
    }

    #[test]
    fn branch_delete__a_branch_merged_into_its_upstream_is_deleted_with_d_after_a_confirmation() {
        let context = on_old(true);
        assert_eq!(
            delete(&context, &Answers::new()),
            Outcome::Read(
                MERGED,
                Query::IsAncestor {
                    ancestor: String::from("refs/heads/old"),
                    descendant: String::from("refs/remotes/origin/old")
                }
            )
        );
        let plan = plan(delete(&context, &Answers::new().with(MERGED, Answer::Flag(true))));
        assert_eq!(lines(&plan), ["git branch -d old"]);
        assert!(
            matches!(&plan.danger, Danger::Confirm(reason) if reason.as_str().starts_with("Its commits are merged"))
        );
    }

    #[test]
    fn branch_delete__a_branch_not_merged_into_head_is_deleted_with_capital_d_after_a_warning() {
        let context = on_old(false);
        assert!(matches!(
            delete(&context, &Answers::new()),
            Outcome::Read(_, Query::IsAncestor { descendant, .. }) if descendant == "HEAD"
        ));
        let plan = plan(delete(&context, &Answers::new().with(MERGED, Answer::Flag(false))));
        assert_eq!(lines(&plan), ["git branch -D old"]);
        assert!(matches!(plan.danger, Danger::Destructive(_)));
    }

    #[test]
    fn branch_delete__when_git_still_finds_it_not_fully_merged_delete_anyway_offers_capital_d() {
        let context = on_old(true);
        let merged = plan(delete(&context, &Answers::new().with(MERGED, Answer::Flag(true))));
        let [FollowUp::OfferRun(offered)] = merged.on_failure.as_slice() else {
            panic!("Delete anyway is offered: {:?}", merged.on_failure)
        };
        assert_eq!(offered.label.as_str(), "Delete anyway");
        assert_eq!(offered.when_said, ["not fully merged"]);
        let again = plan(delete(&context, &offered.answers));
        assert_eq!(lines(&again), ["git branch -D old"]);
        assert!(again.on_failure.is_empty(), "-D has nothing to fall back to");
    }

    #[test]
    fn branch_delete__leaves_the_branch_and_its_commit_for_undo() {
        let plan = plan(delete(&on_old(false), &Answers::new().with(MERGED, Answer::Flag(true))));
        assert_eq!(
            plan.undo,
            Undo::Journal(UndoRecord::BranchDeleted {
                name: String::from("old"),
                commit: commit('b')
            })
        );
    }

    #[test]
    fn branch_delete__refuses_the_branch_checked_out() {
        let context = context(state(), Target::Branch(String::from("main")));
        assert!(matches!(delete(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
