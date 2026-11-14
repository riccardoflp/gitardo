//! `cherry-pick.skip-empty`: when a cherry-pick or a revert stops on a commit whose changes are already there, git
//! will not go on until that commit is skipped; this skips it, as a rebase would, and says so in the log.

use model::contract::{AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Refusal, Step, Text, Undo};
use model::state::Operation;

use super::skip_when_empty;

/// The stop with nothing left to do, read.
const STOP: AnswerKey = AnswerKey("stop");

/// Skips the commit a cherry-pick or a revert stopped on when nothing is left to do on it: a warning in the log, then
/// `git cherry-pick --skip` (or `git revert --skip`); the next commit of a series that stops the same way is skipped
/// the same way. It does nothing when a file is left in conflict or a tracked file differs from `HEAD`.
///
/// Feature: cherry-pick.skip-empty
#[must_use]
pub fn skip_empty(context: &Context, answers: &Answers) -> Outcome {
    let Some(stop) = answers.empty_stop(STOP) else {
        return Outcome::Read(STOP, Query::EmptyStop);
    };
    let Some(stop) = stop else {
        return Outcome::Refuse(Refusal::saying("Nothing is stopped on a commit that changes nothing."));
    };
    let (command, words) = match stop.operation {
        Operation::Revert => ("revert", "the revert"),
        // Only a cherry-pick or a revert stops with a commit to skip; any other operation is read as neither.
        _ => ("cherry-pick", "the cherry-pick"),
    };
    let short = stop.commit.short();
    Outcome::Run(Plan {
        title: Text::from(format!("Skip {short}")),
        steps: vec![
            Step::Warn {
                words: format!("{short} changes nothing on top of HEAD, so {words} skips it."),
            },
            Step::git(context.repository.folder.clone(), [command, "--skip"]),
        ],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: skip_when_empty(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{STOP, skip_empty};
    use crate::testing::{commit, context, lines, plan, state};
    use model::contract::{Answer, Answers, EmptyStop, Outcome, Query, Target};
    use model::state::Operation;

    #[test]
    fn cherry_pick_skip_empty__skips_with_a_warning_when_nothing_is_left_to_do() {
        let context = context(state(), Target::Head);
        assert!(matches!(
            skip_empty(&context, &Answers::new()),
            Outcome::Read(STOP, Query::EmptyStop)
        ));
        let stop = EmptyStop {
            operation: Operation::Revert,
            commit: commit('b'),
        };
        let plan = plan(skip_empty(
            &context,
            &Answers::new().with(STOP, Answer::EmptyStop(Some(stop))),
        ));
        assert_eq!(
            lines(&plan),
            [
                "bbbbbbb changes nothing on top of HEAD, so the revert skips it.",
                "git revert --skip"
            ]
        );
    }

    #[test]
    fn cherry_pick_skip_empty__with_a_conflict_or_a_change_left_nothing_is_skipped() {
        let context = context(state(), Target::Head);
        assert!(matches!(
            skip_empty(&context, &Answers::new().with(STOP, Answer::EmptyStop(None))),
            Outcome::Refuse(_)
        ));
    }
}
