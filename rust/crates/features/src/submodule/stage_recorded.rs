//! `submodule.stage-recorded`: stages the commit a submodule has checked out as the one the repository records for it.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text, Undo};

use super::{initialized, is_apart, refusal, submodule_step};

/// What is said once the commit is staged.
pub const COMMIT_TO_RECORD: &str = "Commit it to record that commit for the submodule.";

/// Runs `git [-C <parent>] add -- <path>` on an initialized submodule apart from its recorded commit, so that the next
/// commit of the repository records the commit it has checked out.
///
/// Feature: submodule.stage-recorded
#[must_use]
pub fn stage_recorded(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ".") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let commit = submodule
        .checkout
        .as_ref()
        .and_then(|checkout| checkout.commit.as_ref());
    let (Some(commit), true) = (commit, is_apart(submodule)) else {
        return *refusal(&format!("{} is on the recorded commit already.", submodule.folder));
    };
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Stage {} as the recorded commit of {}",
            commit.short(),
            submodule.folder
        )),
        steps: vec![submodule_step(context, submodule, &["add"])],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: unstaging the submodule in the Changes page takes it back.",
        )),
        after: vec![FollowUp::Say(Text::from(COMMIT_TO_RECORD))],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, FollowUp, Outcome, Text};

    use super::{COMMIT_TO_RECORD, stage_recorded};
    use crate::submodule::testing::{apart, lib, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_stage_recorded__on_lib_two_commits_ahead_it_stages_lib() {
        let plan = plan(stage_recorded(&on(apart(lib(), (2, 0), Some("main"))), &Answers::new()));
        assert_eq!(lines(&plan), ["git add -- lib"]);
        assert_eq!(plan.title.as_str(), "Stage 9999999 as the recorded commit of lib");
        assert_eq!(plan.after, [FollowUp::Say(Text::from(COMMIT_TO_RECORD))]);
    }

    #[test]
    fn submodule_stage_recorded__a_submodule_not_initialized_is_refused() {
        let mut empty = lib();
        empty.checkout = None;
        let Outcome::Refuse(refusal) = stage_recorded(&on(empty), &Answers::new()) else {
            panic!("it is refused");
        };
        assert_eq!(refusal.message.as_str(), "lib is not initialized.");
    }
}
