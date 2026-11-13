//! `worktree.open`: opens a worktree where `worktrees.openIn` says, as a double click on it does.

use model::contract::{Answers, Context, Outcome};

use super::{label, opening, refuse_missing, refuse_shown, target_worktree};

/// Opens the worktree invoked on, or the worktree that has the branch invoked on, where `worktrees.openIn` says: a new
/// window of Gitardo Desktop, the external editor, or a terminal. With `editor` and no `editor.external`, or an editor
/// that does not start, it opens in a new window and says why. The worktree the views show, and a missing one, are
/// not opened.
///
/// Feature: worktree.open
#[must_use]
pub fn open(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_shown(worktree).or_else(|| refuse_missing(worktree)) {
        return refusal;
    }
    Outcome::Run(opening::where_set(
        context,
        &worktree.folder,
        &label(context, &worktree.folder),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, FollowUp, Step, Target, WorktreeOpenIn};

    use super::open;
    use crate::testing::plan;
    use crate::worktree::testing::{LOGIN, with_worktrees};

    #[test]
    fn worktree_open__unset_it_opens_a_new_window_on_the_worktree() {
        let context = with_worktrees(Target::Worktree(LOGIN.into()));
        let plan = plan(open(&context, &Answers::new()));
        assert_eq!(plan.steps, []);
        assert_eq!(plan.after, [FollowUp::OpenWindow(LOGIN.into())]);
    }

    #[test]
    fn worktree_open__set_to_terminal_it_opens_a_terminal_in_its_folder() {
        let mut context = with_worktrees(Target::Branch(String::from("feature/login")));
        context.settings.worktrees.open_in = WorktreeOpenIn::Terminal;
        context.settings.worktrees.terminal = String::from("wt -d ${folder}");
        let plan = plan(open(&context, &Answers::new()));
        let [Step::Run { command, directory }] = plan.steps.as_slice() else {
            panic!("{:?}", plan.steps)
        };
        assert!(command.starts_with("wt -d ") && command.contains("login"), "{command}");
        assert_eq!(directory, &std::path::PathBuf::from(LOGIN));
    }

    #[test]
    fn worktree_open__set_to_editor_with_no_editor_it_opens_a_new_window_and_says_why() {
        let mut context = with_worktrees(Target::Worktree(LOGIN.into()));
        context.settings.worktrees.open_in = WorktreeOpenIn::Editor;
        let plan = plan(open(&context, &Answers::new()));
        assert_eq!(plan.steps, []);
        assert_eq!(plan.after[0], FollowUp::OpenWindow(LOGIN.into()));
        assert!(
            matches!(&plan.after[1], FollowUp::Say(said) if said.as_str().contains("No external editor")),
            "{:?}",
            plan.after
        );
    }

    #[test]
    fn an_editor_that_does_not_start_opens_a_new_window_instead() {
        let mut context = with_worktrees(Target::Worktree(LOGIN.into()));
        context.settings.worktrees.open_in = WorktreeOpenIn::Editor;
        context.settings.external_editor = String::from("code --goto ${file}:${line}");
        let plan = plan(open(&context, &Answers::new()));
        assert_eq!(plan.on_failure, [FollowUp::OpenWindow(LOGIN.into())]);
    }
}
