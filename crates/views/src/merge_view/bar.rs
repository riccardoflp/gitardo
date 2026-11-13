//! The bar at the top of a Merge page: the arrows that go from region to region, where the current one is among them
//! and how many conflicts are left (`merge-view.navigate`); and, at its end, the button with two ticks
//! (`merge-view.accept-simple`), the sparkle (`merge-view.accept-proposals`) and the clock (`merge-view.show-base`).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Key, Label, Modifiers, NamedKey, TextKind, Tone, Ui};

use super::{MergeIntent, MergePageIntent, MergePageState};
use crate::intent::Intent;
use crate::text::diff as diff_words;
use crate::text::merge_view as words;

/// The bar, over the rectangle its caller clipped it to.
pub(crate) fn bar_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, spacing.medium));
    let mut parts = Area::with_gap(inner, spacing.medium);
    let side = theme.sizes.control_height;
    let mut intents = Vec::new();
    let clock = parts.right(side);
    intents.extend(ui.clip(clock, |ui| show_base_view(state, ui)));
    let sparkle = parts.right(side);
    intents.extend(ui.clip(sparkle, |ui| accept_proposals_view(state, ui)));
    let ticks = parts.right(side);
    intents.extend(ui.clip(ticks, |ui| accept_simple_view(state, ui)));
    let arrows = parts.rest();
    intents.extend(ui.clip(arrows, |ui| navigate_view(state, ui)));
    let separator = ui::Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(separator, ui::Axis::Horizontal);
    intents
}

/// An intent of a page.
fn asked(state: &MergePageState<'_>, intent: MergePageIntent) -> Intent {
    Intent::MergeView(MergeIntent::Page(state.id, intent))
}

/// The arrows ▲ and ▼ with **2 / 5** beside them and how many conflicts are left, over the rectangle its caller clipped
/// it to, and the keys `F7` and `Shift+F7` in any column: every region is a stop, the current one outlined in every
/// column and centered; past the last, `F7` goes round to the first.
///
/// Feature: merge-view.navigate
pub fn navigate_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let side = theme.sizes.control_height;
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.small);
    let mut intents = Vec::new();
    let previous = Button::icon(Icon::ArrowUp, diff_words::PREVIOUS_CHANGE).kind(ButtonKind::Icon);
    let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
    if ui.button(("merge previous", state.id), cell, &previous).clicked {
        intents.push(asked(state, MergePageIntent::Previous));
    }
    let next = Button::icon(Icon::ArrowDown, diff_words::NEXT_CHANGE).kind(ButtonKind::Icon);
    let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
    if ui.button(("merge next", state.id), cell, &next).clicked {
        intents.push(asked(state, MergePageIntent::Next));
    }
    let regions = state.text.map_or(0, |text| text.regions.len());
    let position = if regions == 0 {
        diff_words::NO_CHANGES.to_owned()
    } else {
        diff_words::position(state.current, regions)
    };
    let style = ui.text_style(TextKind::Detail);
    let width = ui.measure("000 / 000", style).max(ui.measure(&position, style));
    let _ = ui.label(
        parts.left(width),
        &Label::new(&position).kind(TextKind::Detail).tone(Tone::Muted),
    );
    if let Some(text) = state.text {
        let left = text.conflicts_left();
        let said = if left > 0 {
            words::conflicts_left(left)
        } else if model::merge_view::has_conflicts(&text.regions) {
            words::NO_CONFLICTS_LEFT.to_owned()
        } else {
            words::NO_CONFLICTS.to_owned()
        };
        let _ = ui.label(parts.rest(), &Label::new(&said).kind(TextKind::Detail));
    }
    for key in ui.unclaimed_keys() {
        match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Function(7)), Modifiers::NONE) => intents.push(asked(state, MergePageIntent::Next)),
            (Key::Named(NamedKey::Function(7)), Modifiers::SHIFT) => {
                intents.push(asked(state, MergePageIntent::Previous));
            }
            _ => {}
        }
    }
    intents
}

/// The button with two ticks, over the rectangle its caller clipped it to: it accepts at once every change that needs
/// no choice, as one edit; disabled when none would change the result.
///
/// Feature: merge-view.accept-simple
pub fn accept_simple_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let side = ui.theme().sizes.control_height;
    let count = state.text.map_or(0, |text| text.simple.len());
    let tooltip = if count == 0 {
        words::NO_SIMPLE.to_owned()
    } else {
        words::accept_simple(count)
    };
    let button = Button::icon(Icon::Check, &tooltip).kind(ButtonKind::Icon);
    let button = if count == 0 { button.disabled() } else { button };
    let cell = place(rect, (side, side), (Align::Start, Align::Center));
    if ui.button(("merge accept simple", state.id), cell, &button).clicked {
        return vec![asked(state, MergePageIntent::AcceptSimple)];
    }
    Vec::new()
}

/// The sparkle, over the rectangle its caller clipped it to, while anything is proposed: it accepts every proposal at
/// once, as one edit; disabled when none is for a conflict still marked.
///
/// Feature: merge-view.accept-proposals
pub fn accept_proposals_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(text) = state.text.filter(|text| !text.proposals.is_empty()) else {
        return Vec::new();
    };
    let rect = ui.current_clip();
    let side = ui.theme().sizes.control_height;
    let marked = text
        .proposals
        .iter()
        .filter(|proposal| {
            text.regions
                .get(proposal.region)
                .is_some_and(model::merge_view::Region::is_unresolved)
        })
        .count();
    let tooltip = if marked == 0 {
        words::NOTHING_PROPOSED.to_owned()
    } else {
        words::accept_proposals(marked)
    };
    let button = Button::icon(Icon::Star, &tooltip).kind(ButtonKind::Icon);
    let button = if marked == 0 { button.disabled() } else { button };
    let cell = place(rect, (side, side), (Align::Start, Align::Center));
    if ui.button(("merge accept proposals", state.id), cell, &button).clicked {
        return vec![asked(state, MergePageIntent::AcceptProposals)];
    }
    Vec::new()
}

/// The clock, over the rectangle its caller clipped it to: **Show the base** adds a fourth column at the left of the
/// current side, **Hide the base** takes it away; the next Merge page opens the same way.
///
/// Feature: merge-view.show-base
pub fn show_base_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let side = ui.theme().sizes.control_height;
    let (said, kind) = if state.is_base_shown {
        (words::HIDE_BASE, ButtonKind::Outline)
    } else {
        (words::SHOW_BASE, ButtonKind::Icon)
    };
    let button = Button::icon(Icon::Clock, said).kind(kind);
    let cell = place(rect, (side, side), (Align::Start, Align::Center));
    if ui.button(("merge show base", state.id), cell, &button).clicked {
        return vec![asked(state, MergePageIntent::ShowBase(!state.is_base_shown))];
    }
    Vec::new()
}
