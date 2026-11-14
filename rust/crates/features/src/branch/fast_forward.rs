//! `branch.fast-forward`: brings a local branch up to its upstream without switching to it; the files and the branch
//! checked out are not touched.

use model::branch::{can_fast_forward, other_worktree};
use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use super::target::{local_branch, refuse};

/// Fast-forwards the local branch it was invoked on to its upstream: by fetching the upstream into it when it is
/// checked out nowhere, or by pulling with `--ff-only` in the worktree that has it, so that its files move with it.
/// Both are named in full, so that a tag of the same name is not taken for either.
///
/// Feature: branch.fast-forward
#[must_use]
pub fn fast_forward(context: &Context, _answers: &Answers) -> Outcome {
    let branch = match local_branch(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let name = branch.name.as_str();
    let (Some(upstream), true) = (&branch.upstream, can_fast_forward(&context.repository, branch)) else {
        return refuse(&format!(
            "{name} cannot be fast-forwarded: it needs an upstream, no commits of its own, and not to be checked out \
             here or in a worktree with changes not committed."
        ));
    };
    let remote = upstream.remote.as_str();
    let remote_branch = upstream.remote_branch.as_str();
    let folder = context.repository.folder.clone();
    let step = match other_worktree(&context.repository, name) {
        None => Step::git(folder, ["fetch", remote, &format!("{remote_branch}:refs/heads/{name}")]),
        Some(worktree) => {
            let mut arguments = vec![std::ffi::OsString::from("-C"), worktree.folder.clone().into_os_string()];
            arguments.extend(
                [
                    "pull",
                    "--ff-only",
                    "--no-rebase",
                    "--no-autostash",
                    remote,
                    remote_branch,
                ]
                .map(Into::into),
            );
            Step::git(folder, arguments)
        }
    };
    let short = upstream
        .reference
        .strip_prefix("refs/remotes/")
        .unwrap_or(&upstream.reference);
    Outcome::Run(Plan {
        title: Text::from(format!("Fast-forward {name} to {short}")),
        steps: vec![step],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo, which follows what is checked out here: the branch's own reflog has where it was.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::fast_forward;
    use crate::testing::{branch, context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target};
    use model::state::{RepositoryState, Upstream, Worktree};

    fn following(name: &str, remote_branch: &str, ahead: u32) -> RepositoryState {
        let mut state = state();
        let mut local = branch(name, 'b', false);
        local.upstream = Some(Upstream {
            reference: format!("refs/remotes/origin/{remote_branch}"),
            remote: String::from("origin"),
            remote_branch: format!("refs/heads/{remote_branch}"),
            ahead,
            behind: 2,
            is_gone: false,
        });
        state.branches.retain(|branch| branch.name != name);
        state.branches.push(local);
        if name == "main" {
            state.head.branch = Some(String::from("refs/heads/other"));
        }
        state
    }

    fn in_worktree(mut state: RepositoryState, uncommitted: usize) -> RepositoryState {
        state.worktrees.push(Worktree {
            folder: PathBuf::from("app-feature"),
            commit: None,
            branch: Some(String::from("refs/heads/feature")),
            is_main: false,
            is_current: false,
            is_missing: false,
            uncommitted: Some(uncommitted),
        });
        state
    }

    fn run(state: RepositoryState, name: &str) -> Outcome {
        fast_forward(&context(state, Target::Branch(name.to_owned())), &Answers::new())
    }

    #[test]
    fn branch_fast_forward__fetches_the_upstream_into_the_branch() {
        let plan = plan(run(following("feature", "feature", 0), "feature"));
        assert_eq!(lines(&plan), ["git fetch origin refs/heads/feature:refs/heads/feature"]);
        assert_eq!(plan.title.as_str(), "Fast-forward feature to origin/feature");
    }

    #[test]
    fn branch_fast_forward__names_an_upstream_of_another_name() {
        let plan = plan(run(following("main", "trunk", 0), "main"));
        assert_eq!(lines(&plan), ["git fetch origin refs/heads/trunk:refs/heads/main"]);
    }

    #[test]
    fn branch_fast_forward__refuses_a_branch_with_a_commit_of_its_own() {
        assert!(matches!(
            run(following("feature", "feature", 1), "feature"),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn branch_fast_forward__pulls_in_the_worktree_that_has_the_branch() {
        let plan = plan(run(in_worktree(following("feature", "feature", 0), 0), "feature"));
        assert_eq!(
            lines(&plan),
            ["git -C app-feature pull --ff-only --no-rebase --no-autostash origin refs/heads/feature"]
        );
    }

    #[test]
    fn branch_fast_forward__refuses_a_branch_whose_worktree_has_changes() {
        assert!(matches!(
            run(in_worktree(following("feature", "feature", 0), 1), "feature"),
            Outcome::Refuse(_)
        ));
    }
}
