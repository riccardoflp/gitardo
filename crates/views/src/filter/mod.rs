//! The branches the timeline shows: the branch filter in the header with its list (`filter.picker`), **Filter the list
//! of branches too** (`filter.list-too`), the bar above the commits while the timeline is narrowed (`filter.bar`), and
//! the entries of the menus of a reference that narrow the timeline (`filter.show-only`, `filter.show-too`,
//! `filter.hide`, `filter.show-all`).

mod menu;
mod picker;
#[cfg(test)]
mod tests;

use model::filter::BranchesShown;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Axis, Button, ButtonKind, Choice, Icon, Label, Rect, Tone, Ui};

use crate::intent::{FilterIntent, Intent};
use crate::text::filter as words;

pub use menu::{FilterEntry, filter_entries, filter_entry_intent, filter_entry_words};
pub use picker::{BranchPicker, PickerOpen, picker_button_width, picker_view};

/// **Filter the list of branches too**, as it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListToo {
    /// Whether the branch rail lists only the branches the timeline shows: `branches.followGraphFilter`.
    pub is_on: bool,
}

/// What the bar above the commits says while the timeline is narrowed to some branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NarrowedBar<'a> {
    /// The branches shown.
    pub shown: &'a BranchesShown,
    /// How many commits are listed, without the draft.
    pub listed: usize,
    /// Whether there are more commits than those listed.
    pub has_more: bool,
}

/// The height of the bar above the commits.
#[must_use]
pub fn bar_height(ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.control_height + 2.0 * sizes.spacing.small
}

/// **Filter the list of branches too**, a check box over the rectangle its caller clipped it to: a click turns
/// `branches.followGraphFilter` the other way.
///
/// Feature: filter.list-too
pub fn list_too_view(state: &ListToo, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let response = ui.choice("list too", rect, &Choice::check_box(words::LIST_TOO, state.is_on));
    ui.tooltip(&response, words::LIST_TOO_TOOLTIP);
    if response.clicked {
        vec![Intent::Filter(FilterIntent::FollowGraphFilter(!state.is_on))]
    } else {
        Vec::new()
    }
}

/// The bar above the commits while the timeline is narrowed to some branches, over the rectangle its caller clipped it
/// to: `Only the history of main, feature and 2 more · 1000+ commits`, and **Show all branches**.
///
/// Feature cards that fill it further: `search.list-mode`, `file-history.file` (what it says of a search shown as a
/// list and of the history of a file, and **Show all commits**).
///
/// Feature: filter.bar
pub fn bar_view(state: &NarrowedBar<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let said = words::bar_words(state.shown, (state.listed, state.has_more));
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
        spacing.medium,
    );
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Filter, Tone::Accent);
    let button = Button::secondary(words::SHOW_ALL_BRANCHES).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&button);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    let clicked = ui.button("show all branches", cell, &button).clicked;
    let _ = ui.label(parts.rest(), &Label::new(&said));
    if clicked {
        vec![Intent::Filter(FilterIntent::Show(None))]
    } else {
        Vec::new()
    }
}
