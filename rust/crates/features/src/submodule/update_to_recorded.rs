//! `submodule.update-to-recorded`: checks out in a submodule the commit the repository records for it, and in its own
//! submodules theirs.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text, Undo};
use model::submodule::{has_changes, lost_commits};

use super::{IN_ITS_REFLOG, is_apart, refusal, submodule_step, target_submodule};

/// Runs `git [-C <parent>] submodule update --init --recursive -- <path>` on a submodule apart from its recorded
/// commit, which brings that commit in when it does not have it. It asks first when that leaves something behind:
/// commits on a detached `HEAD` that no branch will have, or changes not committed.
///
/// Feature: submodule.update-to-recorded
#[must_use]
pub fn update_to_recorded(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match target_submodule(context) {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let Some(recorded) = submodule.recorded.as_ref() else {
        return *refusal(&format!("No commit is recorded for {} yet.", submodule.folder));
    };
    if submodule.checkout.is_some() && !is_apart(submodule) {
        return *refusal(&format!("{} is on the recorded commit already.", submodule.folder));
    }
    let mut left = Vec::new();
    if let Some(count) = lost_commits(submodule) {
        let commits = if count == 1 {
            String::from("1 commit")
        } else {
            format!("{count} commits")
        };
        left.push(format!(
            "It has {commits} after the recorded one on a detached HEAD: no branch will have them. Create a branch in \
             it first to keep them."
        ));
    }
    if has_changes(submodule) {
        left.push(String::from(
            "It has changes that are not committed: git keeps them, or stops if they are in the way.",
        ));
    }
    Outcome::Run(Plan {
        title: Text::from(format!(
            "Update {} to the recorded commit {}",
            submodule.folder,
            recorded.short()
        )),
        steps: vec![submodule_step(
            context,
            submodule,
            &["submodule", "update", "--init", "--recursive"],
        )],
        danger: if left.is_empty() {
            Danger::None
        } else {
            Danger::Confirm(Text::from(left.join("\n\n")))
        },
        undo: Undo::Impossible(Text::from(IN_ITS_REFLOG)),
        after: Vec::new(),
        on_failure: vec![FollowUp::AllowFileProtocol],
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Danger, FollowUp, Outcome};

    use super::update_to_recorded;
    use crate::submodule::testing::{apart, lib, nested, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_update_to_recorded__on_lib_one_commit_behind_it_updates_lib() {
        let plan = plan(update_to_recorded(
            &on(apart(lib(), (0, 1), Some("main"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git submodule update --init --recursive -- lib"]);
        assert_eq!(plan.danger, Danger::None);
        assert_eq!(plan.on_failure, [FollowUp::AllowFileProtocol]);
    }

    #[test]
    fn submodule_update_to_recorded__two_commits_on_a_detached_head_ask_first_and_say_they_will_be_on_no_branch() {
        let plan = plan(update_to_recorded(&on(apart(lib(), (2, 0), None)), &Answers::new()));
        let Danger::Confirm(reason) = &plan.danger else {
            panic!("it asks first: {:?}", plan.danger);
        };
        assert!(
            reason
                .as_str()
                .starts_with("It has 2 commits after the recorded one on a detached HEAD: no branch")
        );
    }

    #[test]
    fn submodule_update_to_recorded__two_commits_on_its_branch_main_do_not_ask() {
        let plan = plan(update_to_recorded(
            &on(apart(lib(), (2, 0), Some("main"))),
            &Answers::new(),
        ));
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn submodule_update_to_recorded__on_vendor_a_b_it_runs_from_vendor_a() {
        let plan = plan(update_to_recorded(&on(apart(nested(), (0, 1), None)), &Answers::new()));
        assert_eq!(
            lines(&plan),
            ["git -C vendor/a submodule update --init --recursive -- b"]
        );
    }

    #[test]
    fn submodule_update_to_recorded__is_not_offered_on_a_submodule_on_its_recorded_commit() {
        assert!(matches!(
            update_to_recorded(&on(lib()), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
