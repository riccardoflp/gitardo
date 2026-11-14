//! The history of a label under the pointer (`graph.hover-history`): after the pointer has rested 150 ms on a label its
//! history stands out; from one label to another it changes at once; leaving the labels brings the timeline back 100 ms
//! later.

use std::time::Duration;

use ui::Ui;

use super::intent::{GraphIntent, HoverTarget};
use crate::intent::Intent;

/// How long the pointer rests on a label before its history stands out.
pub const REST: Duration = Duration::from_millis(150);
/// How long after the pointer left the labels the timeline comes back.
pub const LINGER: Duration = Duration::from_millis(100);

/// The labels and the pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoverState<'a> {
    /// The label whose history stands out.
    pub current: Option<&'a HoverTarget>,
    /// When the pointer left the labels, while the history still stands out.
    pub left_at: Option<Duration>,
    /// The label under the pointer in this frame, with since when the pointer rests on it.
    pub under_pointer: Option<(HoverTarget, Duration)>,
    /// The time of this frame.
    pub now: Duration,
}

/// What the pointer asks of the history brought out, and when to look again if it may ask something later.
#[must_use]
pub fn hover_intent(state: &HoverState<'_>) -> (Option<GraphIntent>, Option<Duration>) {
    match (&state.under_pointer, state.current) {
        (Some((target, _)), Some(current)) if target == current => {
            let back = state.left_at.map(|_| GraphIntent::Hover(Some(target.clone())));
            (back, None)
        }
        (Some((target, _)), Some(_)) => (Some(GraphIntent::Hover(Some(target.clone()))), None),
        (Some((target, since)), None) => {
            if state.now.saturating_sub(*since) >= REST {
                (Some(GraphIntent::Hover(Some(target.clone()))), None)
            } else {
                (None, Some(*since + REST))
            }
        }
        (None, Some(_)) => match state.left_at {
            None => (Some(GraphIntent::HoverLeft(state.now)), Some(state.now + LINGER)),
            Some(left) if state.now.saturating_sub(left) >= LINGER => (Some(GraphIntent::Hover(None)), None),
            Some(left) => (None, Some(left + LINGER)),
        },
        (None, None) => (None, None),
    }
}

/// Brings out the history of the label the pointer rests on, from the labels the rows found under it.
///
/// Feature: graph.hover-history
pub fn hover_history_view(state: &HoverState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let (intent, wake) = hover_intent(state);
    if let Some(time) = wake {
        ui.request_frame_at(time);
    }
    intent.map(Intent::Graph).into_iter().collect()
}
