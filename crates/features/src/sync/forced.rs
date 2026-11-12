//! What the two forced pushes share: the commits they throw away, read and said in the warning.

use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Text, Undo};
use model::remote::Overwritten;

use super::target::{PUSH_UNDO, PushTarget, push_target};

/// The commits the push throws away, as of the last fetch.
pub const OVERWRITTEN: AnswerKey = AnswerKey("overwritten");

/// The plan of a forced push with `option`, once what it overwrites is read; the refusals of `sync.push`.
pub(crate) fn forced_push(context: &Context, answers: &Answers, option: &str) -> Outcome {
    let target = match push_target(context, "push") {
        Ok(target) => target,
        Err(refusal) => return *refusal,
    };
    let overwritten = match (&target.upstream, answers.overwritten(OVERWRITTEN)) {
        (Some(upstream), None) => {
            let upstream = upstream.clone();
            return Outcome::Read(OVERWRITTEN, Query::Overwritten { upstream });
        }
        (Some(_), Some(overwritten)) => Some(overwritten),
        (None, _) => None,
    };
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Push {} to {} with {option}",
            target.branch,
            target.remote_branch()
        )),
        steps: vec![target.step(context, Some(option))],
        danger: Danger::Destructive(Text::from(warning(&target, overwritten))),
        undo: Undo::Impossible(Text::from(PUSH_UNDO)),
        after: vec![FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

/// The warning of a forced push: the commits only the remote branch has, which are lost for everyone.
fn warning(target: &PushTarget, overwritten: Option<&Overwritten>) -> String {
    let remote_branch = target.remote_branch();
    let branch = &target.branch;
    let Some(overwritten) = overwritten else {
        return format!("This replaces `{remote_branch}` with `{branch}` for everyone, whatever it has.");
    };
    if overwritten.commits.is_empty() {
        return format!("As of the last fetch, every commit on `{remote_branch}` is also in `{branch}`.");
    }
    let mut lines = vec![format!(
        "These commits on `{remote_branch}` are not in `{branch}` and will be overwritten:"
    )];
    lines.extend(
        overwritten
            .commits
            .iter()
            .map(|commit| format!("{} {}", commit.short_id, commit.subject)),
    );
    if overwritten.has_more {
        lines.push(String::from("\u{2026}and more"));
    }
    lines.join("\n")
}
