//! Where the branch checked out is pushed, and what a pull runs: shared by Push, its forced ways and Sync.

use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{Context, FeatureId, FollowUp, Outcome, Refusal, Step, Text};

/// The refusal of fetch, pull, push and sync in a repository with no remote, which offers to add one.
pub(crate) fn no_remote() -> Outcome {
    Outcome::Refuse(Refusal {
        message: Text::from("This repository has no remote."),
        offer: Some((FeatureId("remote.add"), Text::from("Add remote…"))),
    })
}

/// Why a push changes nothing Undo can take back.
pub(crate) const PUSH_UNDO: &str = "A push changes the remote, which Undo does not take back.";

/// Where the branch checked out goes when it is pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PushTarget {
    /// The branch checked out, by its short name.
    pub branch: String,
    /// The remote it goes to.
    pub remote: String,
    /// What git is given after the remote: the branch, `<branch>:<upstream branch>` when the names differ.
    pub refspec: String,
    /// Its upstream, in full (`refs/remotes/origin/main`), when it has one that was not deleted on the remote.
    pub upstream: Option<String>,
    /// Whether the push makes the remote branch its upstream: the first push of a branch.
    pub sets_upstream: bool,
}

impl PushTarget {
    /// The arguments of `git push`, with an option such as `--force` before the rest.
    pub(crate) fn arguments(&self, option: Option<&str>) -> Vec<String> {
        let mut arguments = vec![String::from("push")];
        arguments.extend(option.map(str::to_owned));
        if self.sets_upstream {
            arguments.push(String::from("--set-upstream"));
        }
        arguments.push(self.remote.clone());
        arguments.push(self.refspec.clone());
        arguments
    }

    /// The step of `git push`.
    pub(crate) fn step(&self, context: &Context, option: Option<&str>) -> Step {
        Step::git(context.repository.folder.clone(), self.arguments(option))
    }

    /// The remote branch it goes to, by its short name: `origin/main`.
    pub(crate) fn remote_branch(&self) -> String {
        let name = self.refspec.rsplit(':').next().unwrap_or(&self.refspec);
        let name = name.strip_prefix("refs/heads/").unwrap_or(name);
        format!("{}/{name}", self.remote)
    }
}

/// Where the branch checked out is pushed: to its upstream, or for its first push to the remote of `remote.default`;
/// refused on a detached `HEAD` (`Cannot <verb>: HEAD is detached…`) and in a repository with no remote.
pub(crate) fn push_target(context: &Context, verb: &str) -> Result<PushTarget, Box<Outcome>> {
    let repository = &context.repository;
    if repository.remotes.is_empty() {
        return Err(Box::new(no_remote()));
    }
    let Some(name) = repository.checked_out_branch() else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(&format!(
            "Cannot {verb}: HEAD is detached. Check out a branch first."
        )))));
    };
    let argument = ref_argument(RefKind::Branch, name, &RefNames::of(repository));
    let upstream = repository
        .branches
        .iter()
        .find(|branch| branch.name == name)
        .and_then(|branch| branch.upstream.as_ref());
    let target = match upstream {
        Some(upstream) => {
            let remote_name = upstream
                .remote_branch
                .strip_prefix("refs/heads/")
                .unwrap_or(&upstream.remote_branch);
            let refspec = if remote_name == name {
                argument
            } else {
                format!("{argument}:{remote_name}")
            };
            PushTarget {
                branch: name.to_owned(),
                remote: upstream.remote.clone(),
                refspec,
                upstream: (!upstream.is_gone).then(|| upstream.reference.clone()),
                sets_upstream: false,
            }
        }
        None => PushTarget {
            branch: name.to_owned(),
            remote: model::remote::default(&repository.remotes).unwrap_or_default(),
            refspec: argument,
            upstream: None,
            sets_upstream: true,
        },
    };
    Ok(target)
}

/// The arguments of `git pull`, with `--rebase` and `--autostash` as asked.
pub(crate) fn pull_arguments(rebase: bool, autostash: bool) -> Vec<&'static str> {
    let mut arguments = vec!["pull"];
    if rebase {
        arguments.push("--rebase");
    }
    if autostash {
        arguments.push("--autostash");
    }
    arguments
}

/// What follows a pull: the pull requests read again and, with `--autostash`, whether it left a stash.
pub(crate) fn after_pull(autostash: bool) -> Vec<FollowUp> {
    let mut after = vec![FollowUp::ReadPullRequests];
    if autostash {
        after.push(FollowUp::CheckAutostash);
    }
    after
}
