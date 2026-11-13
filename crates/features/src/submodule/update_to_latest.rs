//! `submodule.update-to-latest`: takes a submodule to the last commit of the branch it follows on its remote, and its
//! own submodules to what that commit records.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};

use super::{IN_ITS_REFLOG, initialized, submodule_step};

/// What is said once it ran.
pub const STAGE_AFTERWARDS: &str = "Stage the submodule afterwards to record the commit it is on now.";

/// Runs `git [-C <parent>] submodule update --init --remote -- <path>`, then `git -C <folder> submodule update --init
/// --recursive` for its own submodules, on an initialized submodule.
///
/// Feature: submodule.update-to-latest
#[must_use]
pub fn update_to_latest(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ": update it first.") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let branch = submodule
        .branch
        .as_deref()
        .map_or_else(|| String::from("its remote"), str::to_owned);
    Outcome::Run(Plan {
        title: Text::from(format!("Update {} to the latest of {branch}", submodule.folder)),
        steps: vec![
            submodule_step(context, submodule, &["submodule", "update", "--init", "--remote"]),
            Step::git(
                context.repository.folder.clone(),
                [
                    "-C",
                    submodule.folder.as_str(),
                    "submodule",
                    "update",
                    "--init",
                    "--recursive",
                ],
            ),
        ],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(IN_ITS_REFLOG)),
        after: vec![FollowUp::Say(Text::from(STAGE_AFTERWARDS))],
        on_failure: vec![FollowUp::AllowFileProtocol],
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::Answers;

    use super::update_to_latest;
    use crate::submodule::testing::{lib, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_update_to_latest__fetches_the_latest_of_its_branch_then_updates_its_own_submodules() {
        let mut following = lib();
        following.branch = Some(String::from("main"));
        let plan = plan(update_to_latest(&on(following), &Answers::new()));
        assert_eq!(
            lines(&plan),
            [
                "git submodule update --init --remote -- lib",
                "git -C lib submodule update --init --recursive"
            ]
        );
        assert_eq!(plan.title.as_str(), "Update lib to the latest of main");
    }

    #[test]
    fn submodule_update_to_latest__a_submodule_with_no_branch_follows_its_remote() {
        let plan = plan(update_to_latest(&on(lib()), &Answers::new()));
        assert_eq!(plan.title.as_str(), "Update lib to the latest of its remote");
    }
}
