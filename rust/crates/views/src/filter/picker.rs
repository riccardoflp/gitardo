//! The branch filter (`filter.picker`): the button before Quick search that says what the timeline shows, pressed in
//! while it is narrowed, and the list it opens: the presets, the box, and every reference with a tick.

use model::filter::{BranchesShown, label, label_parts};
use model::refs::References;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Border, Description, Label, Rect, Sense, TextKind, Tone, Ui};

use super::list::{PickerFor, Picking, picker_list};
use crate::icons;
use crate::intent::{FilterIntent, Intent};
use crate::line::is_narrow;
use crate::text::filter as words;

/// How wide the name on the button is at most, before it is cut.
const NAME_MOST: f32 = 160.0;

/// The branch filter: what the timeline shows, the references to choose from, and the list while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BranchPicker<'a> {
    /// The branches shown; none is everything.
    pub shown: Option<&'a BranchesShown>,
    /// The references of the repository, in the order of `refs.sort`.
    pub references: &'a References,
    /// The list, while it is open.
    pub open: Option<PickerOpen<'a>>,
}

/// The list of the branch filter while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerOpen<'a> {
    /// What is typed in its box.
    pub typed: &'a str,
    /// The size of the window it opened in, in logical pixels: a window resized closes it.
    pub window: (f32, f32),
}

/// How wide the button is: the icon alone in the narrow layout, otherwise the icon, the name and the count.
#[must_use]
pub fn picker_button_width(state: &BranchPicker<'_>, ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    let spacing = theme.sizes.spacing;
    if is_narrow(ui) {
        return theme.sizes.control_height;
    }
    let (name, count) = label_parts(state.shown);
    let style = ui.text_style(TextKind::Body);
    let name_width = ui.measure(&name, style).min(NAME_MOST);
    let count_width = count.map_or(0.0, |count| ui.measure(&format!("+{count}"), style) + spacing.small);
    2.0 * spacing.medium + theme.sizes.icon + spacing.small + name_width + count_width
}

/// The button of the branch filter over the rectangle its caller clipped it to, and its list while it is open.
///
/// Feature: filter.picker
pub fn picker_view(state: &BranchPicker<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let clicked = button(state, ui, rect);
    let window = ui.window();
    match (&state.open, clicked) {
        (None, true) => vec![Intent::Filter(FilterIntent::OpenPicker {
            window: (window.width, window.height),
        })],
        (Some(_), true) => vec![PickerFor::Timeline.close()],
        (Some(open), false) => {
            let is_resized = (open.window.0 - window.width).abs() > 0.5 || (open.window.1 - window.height).abs() > 0.5;
            if is_resized {
                vec![PickerFor::Timeline.close()]
            } else {
                let picking = Picking {
                    target: PickerFor::Timeline,
                    chosen: state.shown,
                    references: state.references,
                    typed: open.typed,
                };
                picker_list(&picking, ui, rect)
            }
        }
        (None, false) => Vec::new(),
    }
}

/// Draws the button, and says whether it was clicked.
fn button(state: &BranchPicker<'_>, ui: &mut Ui<'_>, rect: Rect) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id("branch filter button");
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    let is_pressed = state.shown.is_some();
    let radius = theme.sizes.radius.large;
    if is_pressed {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_inactive_selection);
        let width = theme.sizes.border;
        ui.border(
            rect,
            Border {
                width,
                radius,
                color: theme.colors.border,
            },
        );
    }
    if response.hovered || state.open.is_some() {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_hover);
    }
    let tone = if is_pressed { Tone::Accent } else { Tone::Normal };
    let what = label(state.shown);
    // Its icon alone in a narrow window, or where its caller gave it only the room of its icon.
    if is_narrow(ui) || rect.width <= theme.sizes.control_height {
        let cell = place(
            rect,
            (theme.sizes.icon, theme.sizes.icon),
            (Align::Center, Align::Center),
        );
        ui.icon_in(cell, icons::FILTER, tone);
    } else {
        let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
        ui.icon_in(parts.left(theme.sizes.icon), icons::FILTER, tone);
        let (name, count) = label_parts(state.shown);
        if let Some(count) = count {
            let count = format!("+{count}");
            let width = ui.measure(&count, ui.text_style(TextKind::Body));
            let _ = ui.label(parts.right(width), &Label::new(&count).tone(Tone::Muted));
        }
        let _ = ui.label(parts.rest(), &Label::new(&name));
    }
    if response.shows_focus {
        ui.focus_ring(rect, radius);
    }
    let tooltip = words::button_tooltip(&what);
    ui.tooltip(&response, &tooltip);
    let description = Description::new(AccessRole::Button, &tooltip)
        .on(is_pressed)
        .expanded(state.open.is_some());
    ui.describe(id, rect, &description);
    response.clicked
}
