//! What the window shows of the operations: the question asked, the plan shown before it runs, and what was said
//! once the last one ended.

use views::feature::{QuestionState, SaidState};

use super::super::State;
use super::{Asking, Plan};

/// What the window shows of the question asked.
#[must_use]
pub fn question_state(state: &State) -> Option<QuestionState<'_>> {
    match state.operations.running.as_ref()?.asking.as_ref()? {
        Asking::Question {
            question,
            typed,
            picked,
        } => Some(QuestionState {
            question,
            typed,
            picked,
        }),
        Asking::Plan(_) => None,
    }
}

/// The plan shown before it runs.
#[must_use]
pub fn plan_shown(state: &State) -> Option<&Plan> {
    match state.operations.running.as_ref()?.asking.as_ref()? {
        Asking::Plan(plan) => Some(plan),
        Asking::Question { .. } => None,
    }
}

/// What was said once the last operation ended.
#[must_use]
pub fn said_state(state: &State) -> Option<SaidState<'_>> {
    state.operations.said.as_ref().map(|said| SaidState {
        text: &said.text,
        is_error: said.is_error,
        can_undo: said.can_undo,
        offers_stashes: said.offers_stashes,
        found: said.found.as_ref(),
        offer: said.offer.as_ref().map(|(feature, label)| (*feature, label.as_str())),
        stopped: super::super::conflicts::stopped_state(state, said),
        runs: &said.runs,
        link: said.link.as_ref().map(|(label, url)| (label.as_str(), url.as_str())),
    })
}
