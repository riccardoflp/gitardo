//! `conflicts.skip-step`: leaves out the commit or patch the operation stopped on and goes on with the next one.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};
use model::state::Operation;

use super::{command, in_progress, noun, refuse};

/// Skips the step the operation stopped on: `git <operation> --skip`. A merge has no steps; a rebase stopped to edit
/// or split a commit is not skipped, which would throw away what is not committed and keep the commit anyway.
///
/// Feature: conflicts.skip-step
#[must_use]
pub fn skip_step(context: &Context, _answers: &Answers) -> Outcome {
    let operation = match in_progress(context) {
        Ok(operation) => operation,
        Err(refusal) => return *refusal,
    };
    if operation == Operation::Merge {
        return refuse("A merge has no steps to skip.");
    }
    if context.repository.rebase_stop.is_some() {
        return refuse("The rebase stopped here for you to change the commit: Continue or Abort instead.");
    }
    let (after, on_failure) = match operation {
        Operation::CherryPick | Operation::Revert => (Vec::new(), crate::cherry_pick::skip_when_empty()),
        Operation::Rebase => {
            let folder = crate::rebase::files_folder(context);
            (
                vec![FollowUp::DeleteWhenRebaseEnded(folder.clone())],
                vec![FollowUp::DeleteWhenRebaseEnded(folder)],
            )
        }
        Operation::Merge | Operation::Patch => (Vec::new(), Vec::new()),
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Skip this step of the {}", noun(operation))),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            [command(operation), "--skip"],
        )],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after,
        on_failure,
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::skip_step;
    use crate::testing::{commit, context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target};
    use model::rebase::{RebaseStop, StopKind};
    use model::state::Operation;

    fn during(operation: Operation) -> model::contract::Context {
        let mut state = state();
        state.operation = Some(operation);
        context(state, Target::Head)
    }

    #[test]
    fn conflicts_skip_step__a_cherry_pick_skips_with_cherry_pick_skip() {
        let picked = plan(skip_step(&during(Operation::CherryPick), &Answers::new()));
        assert_eq!(lines(&picked), ["git cherry-pick --skip"]);
        assert_eq!(picked.title.as_str(), "Skip this step of the cherry-pick");
        let am = plan(skip_step(&during(Operation::Patch), &Answers::new()));
        assert_eq!(lines(&am), ["git am --skip"]);
    }

    #[test]
    fn conflicts_skip_step__a_merge_has_no_steps_to_skip() {
        let Outcome::Refuse(refusal) = skip_step(&during(Operation::Merge), &Answers::new()) else {
            panic!("refused")
        };
        assert_eq!(refusal.message.as_str(), "A merge has no steps to skip.");
    }

    #[test]
    fn conflicts_skip_step__a_stop_to_edit_a_commit_is_not_skipped() {
        let mut state = state();
        state.operation = Some(Operation::Rebase);
        state.rebase_stop = Some(RebaseStop {
            kind: StopKind::Edit,
            commit: commit('b'),
            subject: String::from("Add the login page"),
        });
        assert!(matches!(
            skip_step(&context(state, Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
