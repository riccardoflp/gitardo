//! `submodule.deinitialize`: empties a submodule's folder and forgets it locally; it stays in `.gitmodules` and in the
//! commits.

use model::contract::{Answers, Context, Danger, FailureOffer, FollowUp, Outcome, Plan, Text, Undo};

use super::{initialized, submodule_step};

/// What git says when it refuses to deinitialize a submodule with changes that are not committed.
pub const LOCAL_MODIFICATIONS: &str = "local modifications";

/// Why it is asked first.
const EMPTIED: &str = "Its folder is emptied, and it stays in .gitmodules and in the commits: update it to get it back. \
Its commits stay in its clone in the git directory.";

/// Why the offer with `--force` warns.
const CHANGES_LOST: &str = "The submodule has changes that are not committed: they will be lost. Its commits stay in \
its clone in the git directory.";

/// Runs `git [-C <parent>] submodule deinit -- <path>` on an initialized submodule, asking first. When git refuses for
/// changes that are not committed, `git [-C <parent>] submodule deinit --force -- <path>` is offered, with a warning.
///
/// Feature: submodule.deinitialize
#[must_use]
pub fn deinitialize(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ".") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let undo = Undo::Impossible(Text::from(
        "Not taken back by Undo: Initialize brings it back from its clone.",
    ));
    let title = format!("Deinitialize {}", submodule.folder);
    let forced = Plan {
        title: Text::from(format!("{title}, losing its changes")),
        steps: vec![submodule_step(context, submodule, &["submodule", "deinit", "--force"])],
        danger: Danger::Destructive(Text::from(CHANGES_LOST)),
        undo: undo.clone(),
        after: Vec::new(),
        on_failure: Vec::new(),
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![submodule_step(context, submodule, &["submodule", "deinit"])],
        danger: Danger::Confirm(Text::from(EMPTIED)),
        undo,
        after: Vec::new(),
        on_failure: vec![FollowUp::OfferWhenFailed(Box::new(FailureOffer {
            when: String::from(LOCAL_MODIFICATIONS),
            plan: forced,
        }))],
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Danger, FollowUp, Step};

    use super::{LOCAL_MODIFICATIONS, deinitialize};
    use crate::submodule::testing::{lib, on};
    use crate::testing::{lines, plan};

    #[test]
    fn submodule_deinitialize__on_a_clean_lib_it_asks_then_runs_git_submodule_deinit() {
        let plan = plan(deinitialize(&on(lib()), &Answers::new()));
        assert_eq!(lines(&plan), ["git submodule deinit -- lib"]);
        assert!(matches!(plan.danger, Danger::Confirm(_)));
    }

    #[test]
    fn submodule_deinitialize__git_s_refusal_for_changes_is_followed_by_the_offer_with_force_shown_as_destructive() {
        let plan = plan(deinitialize(&on(lib()), &Answers::new()));
        let [FollowUp::OfferWhenFailed(offer)] = plan.on_failure.as_slice() else {
            panic!("one offer: {:?}", plan.on_failure);
        };
        assert_eq!(offer.when, LOCAL_MODIFICATIONS);
        let forced: Vec<String> = offer.plan.steps.iter().map(Step::log_line).collect();
        assert_eq!(forced, ["git submodule deinit --force -- lib"]);
        assert!(matches!(offer.plan.danger, Danger::Destructive(_)));
    }
}
