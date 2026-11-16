//! The filter of the branch rail (`refs.list-filter`): the button with the funnel at its top, pressed in while the rail
//! lists less than everything, and the list it opens, the one of the timeline's branch filter over the rail alone.

use model::filter::{BranchesShown, label};
use model::refs::References;
use ui::layout::{Align, place};
use ui::{AccessRole, Border, Description, Icon, Rect, Sense, Tone, Ui};

use crate::filter::{PickerFor, Picking, picker_list};
use crate::intent::{Intent, RailIntent};
use crate::text::rail as words;

/// The filter of the branch rail: what it lists, the references to choose from, and its list while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListFilterState<'a> {
    /// What the rail lists; none is everything.
    pub listed: Option<&'a BranchesShown>,
    /// The references of the repository, in the order of `refs.sort`.
    pub references: &'a References,
    /// The list, while it is open.
    pub open: Option<ListFilterOpen<'a>>,
}

/// The list of the filter of the branch rail while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListFilterOpen<'a> {
    /// What is typed in its box.
    pub typed: &'a str,
    /// The size of the window it opened in, in logical pixels: a window resized closes it. None when it was opened
    /// from Quick search, until it is drawn.
    pub window: Option<(f32, f32)>,
}

/// The button of the filter of the branch rail over the rectangle its caller clipped it to, and its list while it is
/// open: the presets, the box, the branches ticked one by one and the kinds of references, which change only what the
/// rail lists (`refs.listed`).
///
/// Feature: refs.list-filter
pub fn list_filter_view(state: &ListFilterState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let clicked = button(state, ui, rect);
    let window = ui.window();
    let size = (window.width, window.height);
    match (state.open, clicked) {
        (None, true) => vec![Intent::Rail(RailIntent::OpenListFilter { window: Some(size) })],
        (Some(_), true) => vec![Intent::Rail(RailIntent::CloseListFilter)],
        (Some(open), false) => {
            let mut intents = Vec::new();
            let opened_in = open.window.unwrap_or_else(|| {
                intents.push(Intent::Rail(RailIntent::OpenListFilter { window: Some(size) }));
                size
            });
            if (opened_in.0 - size.0).abs() > 0.5 || (opened_in.1 - size.1).abs() > 0.5 {
                return vec![Intent::Rail(RailIntent::CloseListFilter)];
            }
            let picking = Picking {
                target: PickerFor::Rail,
                chosen: state.listed,
                references: state.references,
                typed: open.typed,
            };
            intents.extend(picker_list(&picking, ui, rect));
            intents
        }
        (None, false) => Vec::new(),
    }
}

/// Draws the button, its funnel alone, and says whether it was clicked.
fn button(state: &ListFilterState<'_>, ui: &mut Ui<'_>, rect: Rect) -> bool {
    let theme = *ui.theme();
    let id = ui.id("rail filter button");
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    let is_pressed = state.listed.is_some();
    let radius = theme.sizes.radius.medium;
    if is_pressed {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_inactive_selection);
        ui.border(
            rect,
            Border {
                width: theme.sizes.border,
                radius,
                color: theme.colors.border,
            },
        );
    }
    if response.hovered || state.open.is_some() {
        ui.fill_rounded_rect(rect, radius, theme.colors.list_hover);
    }
    let tone = if is_pressed { Tone::Accent } else { Tone::Muted };
    let cell = place(
        rect,
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Center, Align::Center),
    );
    ui.icon_in(cell, Icon::Filter, tone);
    if response.shows_focus {
        ui.focus_ring(rect, radius);
    }
    let tooltip = words::list_filter_tooltip(&label(state.listed));
    ui.tooltip(&response, &tooltip);
    let description = Description::new(AccessRole::Button, &tooltip)
        .on(is_pressed)
        .expanded(state.open.is_some());
    ui.describe(id, rect, &description);
    response.clicked
}
