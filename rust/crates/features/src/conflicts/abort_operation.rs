//! `conflicts.abort-operation`: takes the repository back to how it was before the operation started.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};
use model::state::Operation;

use super::{command, in_progress, noun};

/// Aborts the operation in progress: `git <operation> --abort`, always asked first, since what was resolved and not
/// committed is thrown away.
///
/// Feature: conflicts.abort-operation
#[must_use]
pub fn abort_operation(context: &Context, _answers: &Answers) -> Outcome {
    let operation = match in_progress(context) {
        Ok(operation) => operation,
        Err(refusal) => return *refusal,
    };
    let name = noun(operation);
    let after = if operation == Operation::Rebase {
        vec![FollowUp::DeleteWhenRebaseEnded(crate::rebase::files_folder(context))]
    } else {
        Vec::new()
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Abort {name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            [command(operation), "--abort"],
        )],
        danger: Danger::Destructive(Text::from(format!(
            "This goes back to the state before the {name} started. Conflicts you already resolved will be lost."
        ))),
        undo: Undo::Impossible(Text::from(
            "What was resolved and not committed is thrown away, and git keeps no copy of it.",
        )),
        after,
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::abort_operation;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Danger, Target};
    use model::state::Operation;

    fn plan_of(operation: Operation) -> model::contract::Plan {
        let mut state = state();
        state.operation = Some(operation);
        plan(abort_operation(&context(state, Target::Head), &Answers::new()))
    }

    #[test]
    fn conflicts_abort_operation__a_merge_is_aborted_with_the_warning() {
        let plan = plan_of(Operation::Merge);
        assert_eq!(lines(&plan), ["git merge --abort"]);
        assert_eq!(plan.title.as_str(), "Abort merge");
        assert_eq!(
            plan.danger,
            Danger::Destructive(
                "This goes back to the state before the merge started. Conflicts you already resolved will be lost."
                    .into()
            )
        );
    }

    #[test]
    fn conflicts_abort_operation__git_am_is_aborted_with_am_abort() {
        assert_eq!(lines(&plan_of(Operation::Patch)), ["git am --abort"]);
    }
}
