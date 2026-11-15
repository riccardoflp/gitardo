//! `remote.delete-branch`: deletes a branch on its remote, for everyone.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Deletes the remote branch it was invoked on from its remote: `git push <remote> --delete <branch>`, the remote being
/// the longest name among the remotes that the remote branch starts with. The branch is named `refs/heads/<branch>`
/// when a tag has its name, since the remote would find the name ambiguous and refuse the push.
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
    let is_ambiguous = context.repository.tags.iter().any(|tag| tag.name == branch);
    let argument = if is_ambiguous {
        format!("refs/heads/{branch}")
    } else {
        branch.to_owned()
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Delete {branch} from {remote}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["push", remote, "--delete", argument.as_str()],
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
    use crate::testing::{context, lines, plan, state, tag};
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
    fn remote_delete_branch__a_branch_a_tag_has_the_name_of_is_named_in_full() {
        let mut state = state();
        state.remotes = vec![String::from("origin")];
        state.tags.push(tag("release", 'b'));
        let target = Target::RemoteBranch(String::from("origin/release"));
        let plan = plan(delete_branch(&context(state, target), &Answers::new()));
        assert_eq!(lines(&plan), ["git push origin --delete refs/heads/release"]);
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
