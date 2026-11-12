//! `branch.checkout-remote`: checks out a remote branch in the local branch of its name, a new one that tracks it, or
//! the one there is, brought up to it.

use model::branch::{RefKind, RefNames, local_name, other_worktree, ref_argument};
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Step, Text,
    Undo,
};

use super::checkout::checkout_plan;
use super::submodules::update_after_checkout;
use super::target::{refuse, remote_branch};

/// How the local branch and the remote branch stand: the commits each has that the other has not.
pub const COUNTS: AnswerKey = AnswerKey("counts");

/// What is done when each has commits of its own: [`REBASE`] or [`AS_IT_IS`].
pub const WAY: AnswerKey = AnswerKey("way");

/// The choice that replays the commits of the local branch on top of the remote branch.
pub const REBASE: &str = "rebase";

/// The choice that checks the local branch out as it is.
pub const AS_IT_IS: &str = "as-it-is";

/// Checks out the remote branch it was invoked on: `git checkout --track` when no local branch has its name; otherwise
/// the local branch, fast-forwarded when it is only behind, and rebased onto it or left as it is, as asked, when each
/// has commits the other has not.
///
/// Feature: branch.checkout-remote
#[must_use]
pub fn checkout_remote(context: &Context, answers: &Answers) -> Outcome {
    let remote = match remote_branch(context) {
        Ok(remote) => remote,
        Err(refusal) => return *refusal,
    };
    let state = &context.repository;
    let remote_argument = ref_argument(RefKind::RemoteBranch, &remote.name, &RefNames::of(state));
    let name = local_name(&remote.name, &state.remotes);
    let Some(local) = state.branches.iter().find(|branch| branch.name == name) else {
        let checkout = vec!["checkout", "--track", remote_argument.as_str()];
        let title = format!("Check out {name} from {}", remote.name);
        return run(
            context,
            answers,
            (&remote_argument, title),
            checkout_steps(context, &[checkout]),
        );
    };
    if let Some(worktree) = other_worktree(state, name) {
        let folder = worktree.folder.display();
        if worktree.is_missing {
            return refuse(&format!(
                "The worktree of {name} is missing from {folder}: prune the worktrees to free the branch."
            ));
        }
        return refuse(&format!("{name} is checked out in the worktree {folder}."));
    }
    let Some((own, theirs)) = answers.counts(COUNTS) else {
        let left = format!("refs/heads/{name}");
        return Outcome::Read(
            COUNTS,
            Query::Counts {
                left,
                right: remote_argument,
            },
        );
    };
    let ways = Ways {
        name,
        remote: &remote_argument,
        is_checked_out: local.is_checked_out,
    };
    let way = if own > 0 && theirs > 0 {
        match answers.choice(WAY) {
            None => return Outcome::Ask(way_question(name, &remote.name, (own, theirs))),
            Some(REBASE) => Way::Rebase,
            Some(_) => Way::AsItIs,
        }
    } else if theirs > 0 {
        Way::FastForward
    } else {
        Way::AsItIs
    };
    match ways.commands(way) {
        Some(commands) => {
            let after = if way == Way::AsItIs {
                name
            } else {
                remote_argument.as_str()
            };
            let title = format!("Check out {name} from {}", remote.name);
            run(context, answers, (after, title), checkout_steps(context, &commands))
        }
        None => Outcome::Run(nothing_to_do(name, &remote.name, theirs == 0)),
    }
}

/// What is done with the local branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// It has nothing of its own and is behind: it is brought up to the remote branch.
    FastForward,
    /// Its commits are replayed on top of the remote branch.
    Rebase,
    /// It is checked out as it is.
    AsItIs,
}

/// The local branch, the remote branch, and whether the local branch is checked out.
struct Ways<'a> {
    name: &'a str,
    remote: &'a str,
    is_checked_out: bool,
}

impl Ways<'_> {
    /// The git commands of a way, none when nothing is to run.
    fn commands(&self, way: Way) -> Option<Vec<Vec<&str>>> {
        let checkout = vec!["checkout", self.name];
        let fast_forward = vec!["merge", "--ff-only", self.remote];
        match (way, self.is_checked_out) {
            (Way::FastForward, true) => Some(vec![fast_forward]),
            (Way::FastForward, false) => Some(vec![checkout, fast_forward]),
            (Way::Rebase, _) => Some(vec![vec!["rebase", "--autostash", self.remote, self.name]]),
            (Way::AsItIs, true) => None,
            (Way::AsItIs, false) => Some(vec![checkout]),
        }
    }
}

/// The git steps of some commands, in the repository's folder.
fn checkout_steps(context: &Context, commands: &[Vec<&str>]) -> Vec<Step> {
    commands
        .iter()
        .map(|command| Step::git(context.repository.folder.clone(), command))
        .collect()
}

/// Runs the steps under a title, followed by the update of the submodules to what `after` has.
fn run(context: &Context, answers: &Answers, (after, title): (&str, String), mut steps: Vec<Step>) -> Outcome {
    match update_after_checkout(context, answers, after) {
        Ok(update) => steps.extend(update),
        Err(read) => return *read,
    }
    Outcome::Run(checkout_plan(title, steps))
}

/// The question asked when each has commits the other has not.
fn way_question(name: &str, remote: &str, (own, theirs): (u32, u32)) -> Question {
    let choice = |identifier: &str, label: String, description: String| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    };
    Question {
        key: WAY,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    REBASE,
                    format!("Check out {name} and rebase it onto {remote}"),
                    format!("Replay its {own} commits on top"),
                ),
                choice(
                    AS_IT_IS,
                    format!("Check out {name} as it is"),
                    String::from("Leave the two apart"),
                ),
            ],
        },
        title: Text::from(format!(
            "{name} has {own} commits that {remote} has not, which has {theirs} commits of its own"
        )),
        detail: None,
        default: Some(model::contract::Answer::Choice(REBASE.to_owned())),
    }
}

/// The plan that runs nothing, when the local branch is checked out already: it says so.
fn nothing_to_do(name: &str, remote: &str, has_everything: bool) -> Plan {
    let said = if has_everything {
        format!("{name} is checked out already, with everything {remote} has.")
    } else {
        format!("{name} is checked out already.")
    };
    Plan {
        title: Text::from(format!("Check out {name} from {remote}")),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing ran.")),
        after: vec![FollowUp::Say(Text::from(said))],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
