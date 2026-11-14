//! `worktree.open-in-new-window`: opens a worktree as the folder of a new window of Gitardo Desktop.

use model::contract::{Answers, Context, Outcome};

use super::{label, opening, refuse_missing, refuse_shown, target_worktree};

/// Opens the worktree invoked on, or the worktree that has the branch invoked on, in a new window of Gitardo Desktop;
/// this window is left as it was. The worktree the views show, and a missing one, are not opened.
///
/// Feature: worktree.open-in-new-window
#[must_use]
pub fn open_in_new_window(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_shown(worktree).or_else(|| refuse_missing(worktree)) {
        return refusal;
    }
    Outcome::Run(opening::in_new_window(
        &worktree.folder,
        &label(context, &worktree.folder),
    ))
}

#[cfg(test)]
mod tests {
    use model::contract::{Answers, FollowUp, Outcome, Target};

    use super::open_in_new_window;
    use crate::testing::plan;
    use crate::worktree::testing::{GONE, LOGIN, MAIN, with_worktrees};

    #[test]
    fn opens_a_window_on_the_worktree_and_not_on_the_one_shown_nor_a_missing_one() {
        let context = with_worktrees(Target::Worktree(LOGIN.into()));
        let plan = plan(open_in_new_window(&context, &Answers::new()));
        assert_eq!(plan.after, [FollowUp::OpenWindow(LOGIN.into())]);
        for folder in [MAIN, GONE] {
            let context = with_worktrees(Target::Worktree(folder.into()));
            assert!(matches!(
                open_in_new_window(&context, &Answers::new()),
                Outcome::Refuse(_)
            ));
        }
    }
}
