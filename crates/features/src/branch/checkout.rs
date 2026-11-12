//! `branch.checkout`: switches the working tree to a local branch.

use model::branch::other_worktree;
use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};

use super::submodules::update_after_checkout;
use super::target::{local_branch, refuse};

/// Checks out the local branch it was invoked on, then updates the submodules when `submodules.updateOnCheckout`
/// asks for it. The branch is given by its bare name, since `refs/heads/<branch>` would leave `HEAD` detached.
///
/// Feature: branch.checkout
#[must_use]
pub fn checkout(context: &Context, answers: &Answers) -> Outcome {
    let branch = match local_branch(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let name = branch.name.as_str();
    if context.repository.checked_out_branch() == Some(name) {
        return refuse(&format!("{name} is checked out already."));
    }
    if let Some(worktree) = other_worktree(&context.repository, name) {
        let folder = worktree.folder.display();
        return refuse(&format!("{name} is checked out in the worktree {folder}."));
    }
    let mut steps = vec![Step::git(context.repository.folder.clone(), ["checkout", name])];
    match update_after_checkout(context, answers, name) {
        Ok(update) => steps.extend(update),
        Err(read) => return *read,
    }
    Outcome::Run(checkout_plan(format!("Check out {name}"), steps))
}

/// The plan of a checkout: nothing to confirm, taken back by the reflog, and `HEAD` selected after.
pub(crate) fn checkout_plan(title: String, steps: Vec<Step>) -> Plan {
    Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![FollowUp::SelectHead],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::checkout;
    use crate::branch::submodules::{SUBMODULES_AFTER, SUBMODULES_BEFORE, none};
    use crate::testing::{branch, context, lines, plan, state};
    use model::contract::{Answer, Answers, FeatureSettings, Outcome, Query, Target, Undo};
    use model::state::Worktree;

    fn on(name: &str) -> model::contract::Context {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        context(state, Target::Branch(name.to_owned()))
    }

    #[test]
    fn branch_checkout__checks_out_a_branch_by_its_bare_name() {
        let plan = plan(checkout(&on("feature"), &none(Answers::new())));
        assert_eq!(lines(&plan), ["git checkout feature"]);
        assert_eq!(plan.title.as_str(), "Check out feature");
    }

    #[test]
    fn branch_checkout__refuses_the_branch_checked_out() {
        assert!(matches!(
            checkout(&on("main"), &Answers::new()),
            Outcome::Refuse(refusal) if refusal.message.as_str() == "main is checked out already."
        ));
    }

    #[test]
    fn branch_checkout__refuses_a_branch_another_worktree_has() {
        let mut context = on("feature");
        let mut state = (*context.repository).clone();
        state.worktrees.push(Worktree {
            folder: PathBuf::from("app-feature"),
            commit: None,
            branch: Some(String::from("refs/heads/feature")),
            is_main: false,
            is_current: false,
            is_missing: false,
            uncommitted: Some(0),
        });
        context.repository = std::sync::Arc::new(state);
        assert!(matches!(checkout(&context, &Answers::new()), Outcome::Refuse(_)));
    }

    #[test]
    fn branch_checkout__updates_the_submodules_when_the_setting_asks_for_it() {
        let context = on("feature");
        assert_eq!(
            checkout(&context, &Answers::new()),
            Outcome::Read(
                SUBMODULES_BEFORE,
                Query::HasFile {
                    revision: String::from("HEAD"),
                    path: String::from(".gitmodules")
                }
            )
        );
        let before = Answers::new().with(SUBMODULES_BEFORE, Answer::Flag(false));
        assert!(matches!(
            checkout(&context, &before),
            Outcome::Read(SUBMODULES_AFTER, _)
        ));
        let after = before.with(SUBMODULES_AFTER, Answer::Flag(true));
        assert_eq!(
            lines(&plan(checkout(&context, &after))),
            ["git checkout feature", "git submodule update --init --recursive"]
        );
        let mut off = context.clone();
        off.settings = FeatureSettings {
            update_submodules_on_checkout: false,
            ..FeatureSettings::default()
        };
        assert_eq!(lines(&plan(checkout(&off, &Answers::new()))), ["git checkout feature"]);
    }

    #[test]
    fn branch_checkout__is_taken_back_by_the_reflog() {
        assert_eq!(
            plan(checkout(&on("feature"), &none(Answers::new()))).undo,
            Undo::ByReflog
        );
    }
}
