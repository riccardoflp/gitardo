//! What **Continue** does in the banner of a rebase stopped to edit a commit (`rebase.edit-stop`).

use model::contract::{Answers, Context, Outcome};
use model::rebase::StopKind;

use super::{continue_plan, refuse};

/// Goes on with a rebase stopped right after a commit marked **Edit**: `git -c core.editor=true rebase --continue`,
/// once nothing is left uncommitted in tracked files, since git would take it into the commit after. Untracked files do
/// not count. Its card is the banner's, `rebase.edit-stop`, which says what **Continue** does there.
#[must_use]
pub fn continue_edit(context: &Context, _answers: &Answers) -> Outcome {
    let state = &context.repository;
    let Some(stop) = state.rebase_stop.as_ref().filter(|stop| stop.kind == StopKind::Edit) else {
        return refuse("The rebase did not stop to edit a commit.");
    };
    let working_tree = &state.working_tree;
    if !working_tree.conflicted.is_empty() {
        return refuse("Resolve the conflicts and stage the files first.");
    }
    if !(working_tree.staged.is_empty() && working_tree.unstaged.is_empty()) {
        return refuse(
            "Some changes are not committed: amend the commit with them, commit them, or stash them, then continue.",
        );
    }
    Outcome::Run(continue_plan(
        context,
        format!("Continue the rebase after {}", stop.commit.short()),
        Vec::new(),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::continue_edit;
    use crate::testing::{commit, context, lines, modified, plan, state};
    use model::contract::{Answers, Outcome, Target};
    use model::rebase::{RebaseStop, StopKind};
    use model::state::{Operation, RepositoryState};

    fn stopped() -> RepositoryState {
        let mut state = state();
        state.operation = Some(Operation::Rebase);
        state.rebase_stop = Some(RebaseStop {
            kind: StopKind::Edit,
            commit: commit('b'),
            subject: String::from("Add the login page"),
        });
        state
    }

    #[test]
    fn rebase_edit_stop__continue_with_a_file_modified_is_refused_and_runs_nothing() {
        let mut state = stopped();
        state.working_tree.unstaged.push(modified("a.txt"));
        let Outcome::Refuse(refusal) = continue_edit(&context(state, Target::Head), &Answers::new()) else {
            panic!("refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "Some changes are not committed: amend the commit with them, commit them, or stash them, then continue."
        );
    }

    #[test]
    fn rebase_edit_stop__continue_runs_rebase_continue_with_untracked_files_there() {
        let mut state = stopped();
        state.working_tree.untracked.push(String::from("notes.txt"));
        let plan = plan(continue_edit(&context(state, Target::Head), &Answers::new()));
        assert_eq!(lines(&plan), ["git -c core.editor=true rebase --continue"]);
    }
}
