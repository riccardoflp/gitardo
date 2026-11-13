//! The bottom of a card (`review.card`): where the proposal comes from beside **Resolve by hand** and **Accept**
//! (`review.accept`), the card folded to one line, and the choices under what is proposed (`review.choose`).

use model::review::ProposalSource;
use ui::layout::{Align, Area, place};
use ui::{Button, Icon, Label, Rect, TextKind, Tone, Ui};

use super::card::{CardState, fold_arrow};
use super::{ConflictCard, Layout, ReviewIntent};
use crate::intent::Intent;
use crate::merge_view::MergeIntent;
use crate::text::review as words;

/// The bottom of an open card, or the card folded to one line.
pub(super) fn bottom_and_folded(
    state: &CardState<'_>,
    ui: &mut Ui<'_>,
    (row, is_folded): (Rect, bool),
    intents: &mut Vec<Intent>,
) {
    if is_folded {
        folded(state, ui, row, intents);
    } else {
        bottom(state, ui, row, intents);
    }
}

/// Where the proposal comes from, and the buttons: the sides, the base, **Resolve by hand**, and **Accept** last.
fn bottom(state: &CardState<'_>, ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let card = state.card;
    let mut area = Area::with_gap(row, ui.theme().sizes.spacing.small);
    intents.extend(ui.clip(row, |ui| accept_view(state, ui)));
    let accept = Button::primary(words::ACCEPTED);
    let _ = area.right(ui.button_size(&accept).0);
    let by_hand = Button::secondary(words::RESOLVE_BY_HAND).with_tooltip(words::RESOLVE_BY_HAND_TOOLTIP);
    if ui
        .button("by hand", right_cell(ui, &mut area, &by_hand), &by_hand)
        .clicked
    {
        intents.push(Intent::MergeView(MergeIntent::OpenAt(card.key.path.clone(), card.line)));
    }
    if let Some(said) = ask_words(state) {
        let ask = Button::secondary(said).with_tooltip(words::ASK_TOOLTIP);
        let ask = if card.ai.is_asked() || state.is_running {
            ask.disabled()
        } else {
            ask
        };
        if ui.button("ask", right_cell(ui, &mut area, &ask), &ask).clicked {
            intents.push(Intent::Review(ReviewIntent::AskAi(Some(card.key.clone()))));
        }
    }
    if state.layout == Layout::Cards && card.proposal.is_some() {
        if card.shown.is_sides_shown && card.base.is_some() {
            let base = Button::secondary(words::SHOW_BASE).with_tooltip(words::SHOW_BASE_TOOLTIP);
            if ui.button("base", right_cell(ui, &mut area, &base), &base).clicked {
                intents.push(Intent::Review(ReviewIntent::ToggleBase(card.key.clone())));
            }
        }
        let said = if card.shown.is_sides_shown {
            words::HIDE_SIDES
        } else {
            words::COMPARE_SIDES
        };
        let sides = Button::secondary(said);
        if ui.button("sides", right_cell(ui, &mut area, &sides), &sides).clicked {
            intents.push(Intent::Review(ReviewIntent::ToggleSides(card.key.clone())));
        }
    }
    let _ = ui.label(
        area.rest(),
        &Label::new(&origin(card)).kind(TextKind::Small).tone(Tone::Muted),
    );
}

/// The button that asks the AI about a card, with `conflicts.ai`: **Ask again** for a proposal of the AI or once it
/// failed, **Ask the AI** with nothing proposed; none for a proposal of the rules or a choice.
fn ask_words(state: &CardState<'_>) -> Option<&'static str> {
    if !state.can_ask {
        return None;
    }
    let card = state.card;
    match card.proposal.as_ref().map(|proposal| &proposal.source) {
        Some(ProposalSource::Ai { .. }) => Some(words::ASK_AGAIN),
        None if matches!(card.ai, super::CardAi::Failed(_)) => Some(words::ASK_AGAIN),
        None => Some(words::ASK_THE_AI),
        Some(ProposalSource::Rules(_) | ProposalSource::Choice(_)) => None,
    }
}

/// A button's cell at the right of an area, in the middle down.
fn right_cell(ui: &Ui<'_>, area: &mut Area, button: &Button<'_>) -> Rect {
    let (width, height) = ui.button_size(button);
    place(area.right(width), (width, height), (Align::Start, Align::Center))
}

/// Where a proposal comes from, in words: **Rules · Changes to lines next to each other**, **AI · model · high ·
/// reason**, **Your choice · Only the current side**.
pub(super) fn origin(card: &ConflictCard) -> String {
    let Some(proposal) = &card.proposal else {
        return String::new();
    };
    match &proposal.source {
        ProposalSource::Rules(case) => format!("{} \u{b7} {}", words::RULES, words::rule_case(*case)),
        ProposalSource::Ai {
            model,
            reason,
            confidence,
        } => {
            let mut said = format!("{} \u{b7} {model} \u{b7} {}", words::AI, words::confidence(*confidence));
            if !reason.is_empty() {
                said.push_str(" \u{b7} ");
                said.push_str(reason);
            }
            if proposal.counts.new > 0 {
                said.push_str(" \u{b7} ");
                said.push_str(&words::counted(proposal.counts.new, "new line"));
                said.push_str(": ");
                said.push_str(words::WRITTEN_BY_MODEL);
            }
            said
        }
        ProposalSource::Choice(kind) => format!("{} \u{b7} {}", words::YOUR_CHOICE, words::choice_reason(*kind)),
    }
}

/// The card folded to one line: the arrow, the first line of the resolution, its warnings, where it comes from and
/// **Accepted**.
fn folded(state: &CardState<'_>, ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let card = state.card;
    let mut area = Area::with_gap(row, ui.theme().sizes.spacing.small);
    if fold_arrow(ui, &mut area, ("unfold", Icon::ChevronRight, words::UNFOLD)) {
        intents.push(Intent::Review(ReviewIntent::Fold(card.key.clone(), false)));
    }
    intents.extend(ui.clip(row, |ui| accept_view(state, ui)));
    let accept = Button::primary(words::ACCEPTED);
    let _ = area.right(ui.button_size(&accept).0);
    let first = card
        .proposal
        .as_ref()
        .and_then(|proposal| proposal.lines.first())
        .map_or(words::DELETES_LINES, |line| line.line.text.as_str());
    let mut said = first.trim().to_owned();
    if let Some(warned) = card
        .proposal
        .as_ref()
        .and_then(|proposal| words::warnings(&proposal.warnings))
    {
        said.push_str("  \u{b7} ");
        said.push_str(&warned);
    }
    said.push_str("  \u{b7} ");
    said.push_str(&origin(card));
    let _ = ui.label(area.rest(), &Label::new(&said).kind(TextKind::Code));
}

/// **Accept**, or **Accepted** to take it back, at the right of the rectangle its caller clipped it to; nothing when
/// nothing is proposed.
///
/// Feature: review.accept
pub fn accept_view(state: &CardState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let card = state.card;
    if card.proposal.is_none() {
        return Vec::new();
    }
    let rect = ui.current_clip();
    let button = if card.is_accepted {
        Button::secondary(words::ACCEPTED).with_icon(Icon::Check)
    } else {
        Button::primary(words::ACCEPT).with_tooltip(words::ACCEPT_TOOLTIP)
    };
    let button = if state.is_running { button.disabled() } else { button };
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::End, Align::Center));
    if !ui.button("accept", cell, &button).clicked {
        return Vec::new();
    }
    let key = card.key.clone();
    vec![Intent::Review(if card.is_accepted {
        ReviewIntent::TakeBack(key)
    } else {
        ReviewIntent::Accept(key)
    })]
}

/// The choices under what is proposed, over the rectangle its caller clipped it to: **Instead:** (**Choose:** with
/// nothing proposed), the sides and both of them in each order but the shape the proposal has, **Edit**, and **Drop
/// your choice** on a choice.
///
/// Feature: review.choose
pub fn choose_view(state: &CardState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let card = state.card;
    let rect = ui.current_clip();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let mut intents = Vec::new();
    let said = if card.proposal.is_some() {
        words::INSTEAD
    } else {
        words::CHOOSE
    };
    let width = ui.measure(said, ui.text_style(TextKind::Small));
    let _ = ui.label(
        area.left(width),
        &Label::new(said).kind(TextKind::Small).tone(Tone::Muted),
    );
    for kind in &card.choices {
        let button = Button::secondary(words::choice(*kind));
        let button = if state.is_running || *kind == model::review::ChoiceKind::Written {
            button.disabled()
        } else {
            button
        };
        let (width, height) = ui.button_size(&button);
        let cell = place(area.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("choice", words::choice(*kind)), cell, &button).clicked {
            intents.push(Intent::Review(ReviewIntent::Choose(card.key.clone(), *kind)));
        }
    }
    if card
        .proposal
        .as_ref()
        .is_some_and(|proposal| matches!(proposal.source, ProposalSource::Choice(_)))
    {
        let button = Button::outline(words::DROP_CHOICE);
        let (width, height) = ui.button_size(&button);
        let cell = place(area.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button("drop choice", cell, &button).clicked {
            intents.push(Intent::Review(ReviewIntent::DropChoice(card.key.clone())));
        }
    }
    intents
}
