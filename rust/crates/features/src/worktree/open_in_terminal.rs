//! `worktree.open-in-terminal`: opens a terminal with a worktree as its folder.

use model::contract::{Answers, Context, Outcome};

use super::{label, opening, refuse_missing, target_worktree};

/// Opens the terminal of `terminal.external`, or the terminal of the system, in the folder of the worktree invoked on,
/// the one the views show included: `run <command>`, run in that folder.
///
/// Feature: worktree.open-in-terminal
#[must_use]
pub fn open_in_terminal(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_missing(worktree) {
        return refusal;
    }
    Outcome::Run(opening::in_terminal(
        context,
        &worktree.folder,
        &label(context, &worktree.folder),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answers, Step, Target};

    use super::open_in_terminal;
    use crate::testing::plan;
    use crate::worktree::testing::{LOGIN, MAIN, with_worktrees};

    #[test]
    fn worktree_open_in_terminal__the_terminal_opens_in_the_worktrees_folder_the_current_one_included() {
        for folder in [LOGIN, MAIN] {
            let context = with_worktrees(Target::Worktree(folder.into()));
            let plan = plan(open_in_terminal(&context, &Answers::new()));
            let [Step::Run { directory, .. }] = plan.steps.as_slice() else {
                panic!("{:?}", plan.steps)
            };
            assert_eq!(directory, &PathBuf::from(folder));
        }
    }
}
