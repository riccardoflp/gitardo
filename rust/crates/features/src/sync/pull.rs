//! `sync.pull`: brings what the upstream has into the branch checked out, the way the settings choose.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use super::target::{after_pull, no_remote, pull_arguments};

/// Pulls into the branch checked out: `git pull`, with `--rebase` when `pull.rebase` is on and `--autostash` when
/// `pull.autostash` is on; refuses in a repository with no remote, offering to add one.
///
/// Feature: sync.pull
#[must_use]
pub fn pull(context: &Context, _answers: &Answers) -> Outcome {
    let settings = &context.settings;
    pull_plan(context, settings.pull_rebase, settings.pull_autostash)
}

/// The plan of a pull with these options, or the refusal of a repository with no remote.
pub(crate) fn pull_plan(context: &Context, rebase: bool, autostash: bool) -> Outcome {
    if context.repository.remotes.is_empty() {
        return no_remote();
    }
    let arguments = pull_arguments(rebase, autostash);
    Outcome::Run(Plan {
        title: Text::from("Pull"),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: after_pull(autostash),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::pull;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target, Undo};

    fn with_origin() -> model::state::RepositoryState {
        let mut state = state();
        state.remotes.push(String::from("origin"));
        state
    }

    #[test]
    fn sync_pull__rebases_and_autostashes_by_default() {
        let plan = plan(pull(&context(with_origin(), Target::Head), &Answers::new()));
        assert_eq!(lines(&plan), ["git pull --rebase --autostash"]);
        assert_eq!(plan.undo, Undo::ByReflog);
    }

    #[test]
    fn sync_pull__leaves_out_each_option_whose_setting_is_off() {
        let mut context = context(with_origin(), Target::Head);
        context.settings.pull_rebase = false;
        assert_eq!(lines(&plan(pull(&context, &Answers::new()))), ["git pull --autostash"]);
        context.settings.pull_autostash = false;
        assert_eq!(lines(&plan(pull(&context, &Answers::new()))), ["git pull"]);
    }

    #[test]
    fn refuses_without_a_remote() {
        assert!(matches!(
            pull(&context(state(), Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
