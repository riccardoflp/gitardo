//! The row of the review page that asks the AI provider (`ai.resolve-conflicts`): **Ask the AI about the rest (N)** and
//! the button with the model's name (`ai.choose-model`), or, while the AI is asked, the bar that counts its answers with
//! **Stop**.

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{Button, Label, Tone, Ui};

use super::{ReviewIntent, ReviewState};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::review as words;

/// The row that asks the AI, over the rectangle its caller clipped it to, with `conflicts.ai`: while it is asked,
/// *Asking the AI: 3 of 5 conflicts answered* and **Stop**; otherwise **Ask the AI about the rest (N)**, disabled when
/// none is left to ask about, and the button with the model's name.
///
/// Feature: ai.resolve-conflicts
pub fn resolve_conflicts_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(model) = state.model.as_deref() else {
        return Vec::new();
    };
    let rect = ui.current_clip();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let mut intents = Vec::new();
    if let Some((answered, total)) = state.asking {
        let stop = Button::secondary(words::STOP).with_tooltip(words::STOP_TOOLTIP);
        if left_button(ui, &mut area, ("stop asking", &stop)) {
            intents.push(Intent::Review(ReviewIntent::StopAi));
        }
        let said = words::asking(answered, total);
        let _ = ui.label(area.rest(), &Label::new(&said).tone(Tone::Muted));
        return intents;
    }
    let count = state.to_ask();
    let said = words::ask_the_rest(count);
    let ask = Button::secondary(&said).with_tooltip(words::ASK_THE_REST_TOOLTIP);
    let ask = if count == 0 || state.is_running {
        ask.disabled()
    } else {
        ask
    };
    if left_button(ui, &mut area, ("ask the rest", &ask)) {
        intents.push(Intent::Review(ReviewIntent::AskAi(None)));
    }
    let choose = Button::secondary(model).with_tooltip(words::MODEL_TOOLTIP);
    let choose = if state.is_running { choose.disabled() } else { choose };
    if left_button(ui, &mut area, ("choose model", &choose)) {
        let run = FeatureIntent::Run(FeatureId("ai.choose-model"), Target::Head);
        intents.push(Intent::Feature(run));
    }
    intents
}

/// A button at the left of an area; whether it was clicked.
fn left_button(ui: &mut Ui<'_>, area: &mut Area, (key, button): (&str, &Button<'_>)) -> bool {
    let (width, height) = ui.button_size(button);
    let cell = place(area.left(width), (width, height), (Align::Start, Align::Center));
    ui.button(key, cell, button).clicked
}
