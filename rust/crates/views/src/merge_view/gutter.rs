//! The buttons on the band of a region in a gutter of a Merge page: on the current side **→** and, on a conflict, **+**
//! and **✦**; on the incoming side **←** and, on a conflict, **+**.

use model::merge_view::Choice;
use ui::layout::{Align, Insets, inset, place};
use ui::{AccessRole, Description, Icon, Label, Rect, Sense, TextKind, Ui};

use super::{MergeIntent, MergePageId, MergePageIntent, Offers};
use crate::intent::Intent;
use crate::text::merge_view as words;

/// The side of the icon of a button of the gutter.
const ICON: f32 = 12.0;

/// The side a gutter is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GutterSide {
    /// The gutter between the current side and the result.
    Current,
    /// The gutter between the result and the incoming side.
    Incoming,
}

/// What the buttons of a band are drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TakeSideState<'a> {
    /// The page.
    pub page: MergePageId,
    /// The region, by its place.
    pub region: usize,
    /// The gutter.
    pub side: GutterSide,
    /// Whether the region is a conflict, where "both" is offered.
    pub is_conflict: bool,
    /// Which choices would change what the result has there.
    pub offers: Offers,
    /// What is proposed for its conflict, by its place, with the tooltip of **✦**.
    pub proposal: Option<(usize, &'a str)>,
}

/// What a button of the gutter does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GutterButton {
    /// Puts a side, or both, in place of the region.
    Take(Choice),
    /// Puts what is proposed in place of the conflict.
    Proposal(usize),
}

/// The buttons of a band, side by side, over the gutter cell its caller clipped it to: a button shows only where it
/// would change what the result has in the region, "both" only on a conflict. Pressing one puts that side, or both in
/// that order, in place of the region in the result, as one edit that `Ctrl+Z` takes back.
///
/// Feature: merge-view.take-side
pub fn take_side_view(state: &TakeSideState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let buttons = offered(state);
    if buttons.is_empty() {
        return Vec::new();
    }
    let theme = *ui.theme();
    let room = inset(rect, Insets::symmetric(theme.sizes.border, 0.0));
    let side = (room.width / 3.0).min(room.height);
    let total = side * crate::number::float_of_count(buttons.len());
    let start = match state.side {
        GutterSide::Current => room.right() - total,
        GutterSide::Incoming => room.x,
    };
    let mut intents = Vec::new();
    for (place_of, (button, name)) in buttons.iter().enumerate() {
        let cell = Rect::new(
            start + side * crate::number::float_of_count(place_of),
            room.y,
            side,
            room.height,
        );
        let square = place(cell, (side, side), (Align::Center, Align::Center));
        if gutter_button(ui, (state, *button), (name, square)) {
            let asked = match button {
                GutterButton::Take(choice) => MergePageIntent::Take(state.region, *choice),
                GutterButton::Proposal(index) => MergePageIntent::AcceptProposal(*index),
            };
            intents.push(Intent::MergeView(MergeIntent::Page(state.page, asked)));
        }
    }
    intents
}

/// The buttons a band offers, with their names, in the order they are drawn from the result outwards.
fn offered<'a>(state: &TakeSideState<'a>) -> Vec<(GutterButton, &'a str)> {
    let mut buttons = Vec::new();
    match state.side {
        GutterSide::Current => {
            if let Some((index, said)) = state.proposal {
                buttons.push((GutterButton::Proposal(index), said));
            }
            if state.is_conflict && state.offers.current_first {
                buttons.push((GutterButton::Take(Choice::CurrentFirst), words::ACCEPT_CURRENT_FIRST));
            }
            if state.offers.current {
                buttons.push((GutterButton::Take(Choice::Current), words::ACCEPT_CURRENT));
            }
        }
        GutterSide::Incoming => {
            if state.offers.incoming {
                buttons.push((GutterButton::Take(Choice::Incoming), words::ACCEPT_INCOMING));
            }
            if state.is_conflict && state.offers.incoming_first {
                buttons.push((GutterButton::Take(Choice::IncomingFirst), words::ACCEPT_INCOMING_FIRST));
            }
        }
    }
    buttons
}

/// A small button of the gutter: its sign, a background while the pointer is on it, its tooltip and its name for the
/// screen reader; **✦** is named **Accept the proposal**, and its tooltip, which says where it comes from, is its value.
/// Whether it was clicked.
fn gutter_button(
    ui: &mut Ui<'_>,
    (state, button): (&TakeSideState<'_>, GutterButton),
    (said, rect): (&str, Rect),
) -> bool {
    let theme = *ui.theme();
    let id = ui.id(("merge gutter button", state.page, state.region, said));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::HOVER);
    let background = if response.hovered {
        theme.colors.list_hover
    } else {
        theme.colors.card_background
    };
    ui.fill_rounded_rect(rect, theme.sizes.radius.small, background);
    let sign = match button {
        GutterButton::Take(Choice::Current) => Some("\u{2192}"),
        GutterButton::Take(Choice::Incoming) => Some("\u{2190}"),
        GutterButton::Take(Choice::CurrentFirst | Choice::IncomingFirst) | GutterButton::Proposal(_) => None,
    };
    match (sign, button) {
        (Some(sign), _) => {
            let _ = ui.label(rect, &Label::new(sign).kind(TextKind::Strong).align(Align::Center));
        }
        (None, GutterButton::Proposal(_)) => icon(ui, rect, Icon::Star),
        (None, GutterButton::Take(_)) => icon(ui, rect, Icon::Plus),
    }
    let description = match button {
        GutterButton::Proposal(_) => Description {
            value: Some(said),
            ..Description::new(AccessRole::Button, words::ACCEPT_PROPOSAL)
        },
        GutterButton::Take(_) => Description::new(AccessRole::Button, said),
    };
    ui.describe(id, rect, &description);
    ui.tooltip(&response, said);
    response.clicked
}

/// An icon in the middle of a button.
fn icon(ui: &mut Ui<'_>, rect: Rect, icon: Icon) {
    let color = ui.theme().colors.foreground;
    let square = place(rect, (ICON, ICON), (Align::Center, Align::Center));
    ui.icon(icon, ui::Point::new(square.x, square.y), ICON, color);
}
