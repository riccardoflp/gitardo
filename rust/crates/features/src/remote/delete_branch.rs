//! `remote.delete-branch`: deletes a branch on its remote, for everyone.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Deletes the remote branch it was invoked on from its remote: `git push <remote> --delete <branch>`, the remote being
/// the longest name among the remotes that the remote branch starts with.
///
/// Feature: remote.delete-branch
#[must_use]
pub fn delete_branch(context: &Context, _answers: &Answers) -> Outcome {
    let Target::RemoteBranch(name) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Pick a remote branch first."));
    };
    let Some((remote, branch)) = model::remote::split_remote_branch(name, &context.repository.remotes) else {
        return Outcome::Refuse(Refusal::saying(&format!(
            "Cannot tell which remote `{name}` belongs to."
        )));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Delete {branch} from {remote}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["push", remote, "--delete", branch],
        )],
        danger: Danger::Destructive(Text::from("This deletes the branch on the remote for everyone.")),
        undo: Undo::Impossible(Text::from(crate::sync::PUSH_UNDO)),
        after: vec![model::contract::FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::delete_branch;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Danger, Outcome, Target};

    fn deleted(remotes: &[&str], remote_branch: &str) -> Outcome {
        let mut state = state();
        state.remotes = remotes.iter().map(|remote| (*remote).to_owned()).collect();
        let target = Target::RemoteBranch(remote_branch.to_owned());
        delete_branch(&context(state, target), &Answers::new())
    }

    #[test]
    fn remote_delete_branch__deletes_the_branch_from_its_remote_with_a_warning() {
        let plan = plan(deleted(&["origin"], "origin/feature/login"));
        assert_eq!(lines(&plan), ["git push origin --delete feature/login"]);
        assert!(matches!(plan.danger, Danger::Destructive(_)));
    }

    #[test]
    fn remote_delete_branch__the_longest_remote_the_name_starts_with_is_taken() {
        let plan = plan(deleted(&["a", "a/b"], "a/b/x"));
        assert_eq!(lines(&plan), ["git push a/b --delete x"]);
    }

    #[test]
    fn refuses_a_branch_of_no_remote() {
        let Outcome::Refuse(refusal) = deleted(&["origin"], "fork/x") else {
            panic!("no remote matches")
        };
        assert_eq!(
            refusal.message.as_str(),
            "Cannot tell which remote `fork/x` belongs to."
        );
    }
}
