//! What a plan whose step failed does next: the plan it offers in its place, and the follow-ups the window carries out.

use model::contract::{FollowUp, Plan};

use crate::steps::StepFailure;

/// The plan a plan whose step failed offers in its place for what git said, asked first: the plan again from that step
/// with the file protocol allowed (`submodule.allow-file-protocol`), or another plan for some words of git's
/// (`FollowUp::OfferWhenFailed`). None when the step was cancelled, or nothing is offered for what git said.
pub(super) fn offered_instead(plan: &Plan, failure: &StepFailure) -> Option<Plan> {
    if failure.cancelled {
        return None;
    }
    plan.on_failure.iter().find_map(|follow_up| match follow_up {
        FollowUp::AllowFileProtocol => model::submodule::allow_file_protocol(&model::submodule::FailedPlan {
            plan,
            failed_step: failure.ran.len(),
            said: &failure.said,
        }),
        FollowUp::OfferWhenFailed(offer) if failure.said.contains(&offer.when) => Some(offer.plan.clone()),
        _ => None, // The other follow-ups get past nothing.
    })
}

/// The follow-ups of a plan that failed which the window carries out: the operations offered for what was said, those
/// with no words to look for whatever was said, and any other the window does. What the engine did itself, getting
/// past the stop or deleting the files of a rebase, is not among them.
pub(super) fn for_the_window(on_failure: &[FollowUp], failure: &StepFailure) -> Box<[FollowUp]> {
    if failure.cancelled {
        return Box::default();
    }
    let said = failure.said.to_lowercase();
    on_failure
        .iter()
        .filter(|follow_up| match follow_up {
            FollowUp::Recover(_) | FollowUp::DeleteWhenRebaseEnded(_) | FollowUp::Cleanup(_) => false,
            FollowUp::OfferRun(offer) => {
                offer.when_said.is_empty() || offer.when_said.iter().any(|words| said.contains(&words.to_lowercase()))
            }
            _ => true, // The window carries out the others as it does after a plan.
        })
        .cloned()
        .collect()
}
