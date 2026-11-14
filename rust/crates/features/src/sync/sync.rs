//! `sync.sync`: fetches, pulls what the upstream has, then pushes what the branch checked out has ahead of it.

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text, Undo,
};

use super::push::push_plan;
use super::target::{PushTarget, after_pull, pull_arguments, push_target};

/// The confirmation that publishes a branch with no upstream.
pub const PUBLISH: AnswerKey = AnswerKey("publish");

/// Fetches every remote, pulls as **Pull** does, and pushes when the branch has commits to push, in one plan that stops
/// at the first step that fails. A branch with no upstream is only published, once confirmed. Refuses on a detached
/// `HEAD`, in a repository with no remote, and when the pull would rebase over changes it does not set aside.
///
/// Feature: sync.sync
#[must_use]
pub fn sync(context: &Context, answers: &Answers) -> Outcome {
    let target = match push_target(context, "sync") {
        Ok(target) => target,
        Err(refusal) => return *refusal,
    };
    if target.sets_upstream {
        if !answers.is_confirmed(PUBLISH) {
            return Outcome::Ask(publish_question(&target));
        }
        return Outcome::Run(push_plan(context, &target));
    }
    let settings = &context.settings;
    let working_tree = &context.repository.working_tree;
    let has_tracked_changes = !(working_tree.staged.is_empty() && working_tree.unstaged.is_empty());
    if settings.pull_rebase && !settings.pull_autostash && has_tracked_changes {
        return Outcome::Refuse(Refusal::saying(
            "Commit or stash the changes first: the pull cannot rebase over them.",
        ));
    }
    Outcome::Run(sync_plan(context, &target))
}

fn publish_question(target: &PushTarget) -> Question {
    Question {
        key: PUBLISH,
        kind: QuestionKind::Confirm {
            accept: Text::from("Publish branch"),
        },
        title: Text::from(format!(
            "`{}` has no upstream. Publish it to `{}`?",
            target.branch, target.remote
        )),
        detail: None,
        default: None,
    }
}

/// `git fetch --all --prune`, the pull of the settings, and the push when the branch is ahead of its upstream.
fn sync_plan(context: &Context, target: &PushTarget) -> Plan {
    let folder = context.repository.folder.clone();
    let settings = &context.settings;
    let mut steps = vec![
        Step::git(folder.clone(), ["fetch", "--all", "--prune"]),
        Step::git(folder, pull_arguments(settings.pull_rebase, settings.pull_autostash)),
    ];
    let ahead = context
        .repository
        .branches
        .iter()
        .find(|branch| branch.name == target.branch)
        .and_then(|branch| branch.upstream.as_ref())
        .map_or(0, |upstream| upstream.ahead);
    if ahead > 0 {
        steps.push(target.step(context, None));
    }
    Plan {
        title: Text::from(format!("Sync {} with {}", target.branch, target.remote_branch())),
        steps,
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: after_pull(settings.pull_autostash),
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
