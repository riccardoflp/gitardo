//! `branch.checkout-remote-and-rebase`: checks out the local branch of a remote branch's name and replays its own
//! commits on top of the remote branch, in one step.

use model::branch::{RefKind, RefNames, rebase_onto, ref_argument};
use model::contract::{Answers, Context, Outcome, Step};

use super::checkout::checkout_plan;
use super::submodules::update_after_checkout;
use super::target::{refuse, remote_branch};

/// Rebases the local branch that follows the remote branch it was invoked on onto it, with what is not committed put
/// aside meanwhile: `git rebase --autostash <remote branch> <branch>`, which checks the branch out first.
///
/// Feature: branch.checkout-remote-and-rebase
#[must_use]
pub fn checkout_remote_and_rebase(context: &Context, answers: &Answers) -> Outcome {
    let remote = match remote_branch(context) {
        Ok(remote) => remote,
        Err(refusal) => return *refusal,
    };
    let Some(local) = rebase_onto(&context.repository, &remote.name) else {
        return refuse(&format!(
            "No branch can be checked out and rebased onto {}: it needs a branch that follows it, with commits of \
             its own and commits to take, that is not checked out.",
            remote.name
        ));
    };
    let argument = ref_argument(RefKind::RemoteBranch, &remote.name, &RefNames::of(&context.repository));
    let mut steps = vec![Step::git(
        context.repository.folder.clone(),
        ["rebase", "--autostash", argument.as_str(), local.name.as_str()],
    )];
    match update_after_checkout(context, answers, &local.name) {
        Ok(update) => steps.extend(update),
        Err(read) => return *read,
    }
    let title = format!("Check out {} and rebase it onto {}", local.name, remote.name);
    Outcome::Run(checkout_plan(title, steps))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::checkout_remote_and_rebase;
    use crate::branch::submodules::none;
    use crate::testing::{branch, commit, context, lines, plan, state};
    use model::Timestamp;
    use model::contract::{Answers, Outcome, Target};
    use model::state::{RemoteBranch, RepositoryState, Upstream};

    fn repository(ahead: u32, behind: u32, is_checked_out: bool) -> RepositoryState {
        let mut state = state();
        state.remotes = vec![String::from("origin")];
        state.remote_branches.push(RemoteBranch {
            name: String::from("origin/feature"),
            remote: String::from("origin"),
            commit: commit('c'),
            date: Timestamp(0),
        });
        let mut feature = branch("feature", 'b', is_checked_out);
        feature.upstream = Some(Upstream {
            reference: String::from("refs/remotes/origin/feature"),
            remote: String::from("origin"),
            remote_branch: String::from("refs/heads/feature"),
            ahead,
            behind,
            is_gone: false,
        });
        state.branches.push(feature);
        state
    }

    fn run(state: RepositoryState) -> Outcome {
        let context = context(state, Target::RemoteBranch(String::from("origin/feature")));
        checkout_remote_and_rebase(&context, &none(Answers::new()))
    }

    #[test]
    fn branch_checkout_remote_and_rebase__rebases_the_branch_onto_its_remote_branch() {
        assert_eq!(
            lines(&plan(run(repository(1, 2, false)))),
            ["git rebase --autostash origin/feature feature"]
        );
    }

    #[test]
    fn branch_checkout_remote_and_rebase__refuses_a_branch_only_behind_or_checked_out() {
        assert!(matches!(run(repository(0, 2, false)), Outcome::Refuse(_)));
        assert!(matches!(run(repository(1, 2, true)), Outcome::Refuse(_)));
    }
}
