//! `conflicts.continue-operation`: goes on with the operation that stopped, once its conflicts are resolved and staged.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};
use model::rebase::StopKind;
use model::state::Operation;

use super::{command, in_progress, noun, refuse};

/// Goes on with the operation in progress: `git -c core.editor=true <operation> --continue`, which keeps the message
/// git prepared rather than opening an editor. While a rebase is stopped to edit or to split a commit, **Continue** is
/// that of the stop (`rebase.edit-stop`, `split.continue`), which commits what is left first. A cherry-pick or a revert
/// that git will not continue because its commit changes nothing on top of `HEAD` skips it
/// (`cherry-pick.skip-empty`).
///
/// Feature: conflicts.continue-operation
#[must_use]
pub fn continue_operation(context: &Context, answers: &Answers) -> Outcome {
    let operation = match in_progress(context) {
        Ok(operation) => operation,
        Err(refusal) => return *refusal,
    };
    let state = &context.repository;
    if !state.working_tree.conflicted.is_empty() {
        return refuse("Resolve the conflicts and stage the files first.");
    }
    match state.rebase_stop.as_ref().map(|stop| stop.kind) {
        Some(StopKind::Edit) => return crate::rebase::continue_edit(context, answers),
        Some(StopKind::Split) => return crate::split::continue_split(context, answers),
        None => {}
    }
    let title = format!("Continue the {}", noun(operation));
    if operation == Operation::Rebase {
        return Outcome::Run(crate::rebase::continue_plan(context, title, Vec::new()));
    }
    let on_failure = match operation {
        Operation::CherryPick | Operation::Revert => crate::cherry_pick::skip_when_empty(),
        Operation::Merge | Operation::Rebase | Operation::Patch => Vec::new(),
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::git(
            state.folder.clone(),
            ["-c", "core.editor=true", command(operation), "--continue"],
        )],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure,
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::continue_operation;
    use crate::testing::{commit, context, lines, plan, state};
    use model::contract::{Answers, FeatureId, FollowUp, Outcome, Target};
    use model::rebase::{RebaseStop, StopKind};
    use model::state::{ConflictKind, ConflictedFile, Operation, RepositoryState};

    fn stopped(operation: Operation) -> RepositoryState {
        let mut state = state();
        state.operation = Some(operation);
        state
    }

    #[test]
    fn conflicts_continue_operation__a_merge_resolved_and_staged_continues_with_the_editor_kept_shut() {
        let plan = plan(continue_operation(
            &context(stopped(Operation::Merge), Target::Head),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git -c core.editor=true merge --continue"]);
        assert_eq!(plan.title.as_str(), "Continue the merge");
    }

    #[test]
    fn conflicts_continue_operation__a_rebase_continues_with_rebase_continue() {
        let plan = plan(continue_operation(
            &context(stopped(Operation::Rebase), Target::Head),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git -c core.editor=true rebase --continue"]);
        let am = plan_of(Operation::Patch);
        assert_eq!(lines(&am), ["git -c core.editor=true am --continue"]);
    }

    fn plan_of(operation: Operation) -> model::contract::Plan {
        plan(continue_operation(
            &context(stopped(operation), Target::Head),
            &Answers::new(),
        ))
    }

    #[test]
    fn conflicts_continue_operation__a_cherry_pick_that_changes_nothing_is_skipped_when_git_refuses_to_go_on() {
        let plan = plan_of(Operation::CherryPick);
        assert_eq!(lines(&plan), ["git -c core.editor=true cherry-pick --continue"]);
        assert_eq!(
            plan.on_failure,
            [FollowUp::Recover(FeatureId("cherry-pick.skip-empty"))]
        );
        assert_eq!(
            plan_of(Operation::Revert).on_failure,
            [FollowUp::Recover(FeatureId("cherry-pick.skip-empty"))]
        );
    }

    #[test]
    fn conflicts_continue_operation__while_a_file_is_in_conflict_it_refuses() {
        let mut state = stopped(Operation::Merge);
        state.working_tree.conflicted.push(ConflictedFile {
            path: String::from("a.txt"),
            kind: ConflictKind::BothModified,
        });
        let Outcome::Refuse(refusal) = continue_operation(&context(state, Target::Head), &Answers::new()) else {
            panic!("refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "Resolve the conflicts and stage the files first."
        );
    }

    #[test]
    fn at_a_stop_to_edit_it_hands_over_to_the_continue_of_the_stop() {
        let mut state = stopped(Operation::Rebase);
        state.working_tree.unstaged.push(crate::testing::modified("a.txt"));
        state.rebase_stop = Some(RebaseStop {
            kind: StopKind::Edit,
            commit: commit('b'),
            subject: String::from("Add the login page"),
        });
        let Outcome::Refuse(refusal) = continue_operation(&context(state, Target::Head), &Answers::new()) else {
            panic!("refused as the stop does")
        };
        assert!(refusal.message.as_str().starts_with("Some changes are not committed"));
    }

    #[test]
    fn with_no_operation_it_refuses() {
        assert!(matches!(
            continue_operation(&context(state(), Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
