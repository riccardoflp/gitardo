//! The branches the timeline shows: the branch filter in the header with its list (`filter.picker`), the eye on a
//! reference of the branch rail the timeline leaves out (`filter.list-too`), the bar above the commits while the
//! timeline is narrowed (`filter.bar`), and the entries of the menus of a reference that narrow the timeline
//! (`filter.show-only`, `filter.show-too`, `filter.hide`, `filter.show-all`).

mod list;
mod menu;
mod picker;
#[cfg(test)]
mod tests;

use model::filter::{BranchesShown, show_too};
use model::refs::References;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Axis, Button, ButtonKind, Icon, Label, Rect, Tone, Ui};

use crate::intent::{FilterIntent, Intent};
use crate::text::filter as words;

pub(crate) use list::{PickerFor, Picking, picker_list};
pub use menu::{FilterEntry, filter_entries, filter_entry_intent, filter_entry_words};
pub use picker::{BranchPicker, PickerOpen, picker_button_width, picker_view};

/// A reference of the branch rail that the timeline leaves out, dimmed there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListToo<'a> {
    /// The branches the timeline shows.
    pub shown: &'a BranchesShown,
    /// The reference, by its full name.
    pub reference: &'a str,
    /// The references of the repository.
    pub references: &'a References,
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

/// The eye at the end of a dimmed row of the branch rail, over the rectangle its caller clipped it to: a click adds the
/// history of its reference to the timeline (`filter.show-too`). The rail lists what its own filter lists, whatever the
/// timeline shows.
///
/// Feature: filter.list-too
pub fn list_too_view(state: &ListToo<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::icon(Icon::Eye, words::SHOW_TOO).kind(ButtonKind::Icon);
    if ui.button(("eye", state.reference), rect, &button).clicked {
        let shown = show_too(state.shown, state.reference, state.references);
        vec![Intent::Filter(FilterIntent::Show(Some(shown)))]
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
