//! The selection of several commits drawn: what a click asks of it, its bar over the commits and its menu.

use ui::layout::{Align, Area, place};
use ui::{Button, Label, MenuAnchor, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui};

use super::{Click, Several, bar_entries, clicked, entry_intents, menu_entries};
use crate::graph::HistoryState;
use crate::intent::Intent;
use crate::text::history as words;

/// The key of the bar of a selection of several commits.
const BAR_KEY: &str = "selection bar";
/// The key of the menu of a selection of several commits.
const MENU_KEY: &str = "selection menu";

/// What a click on a row shown asks of the selection, by the keys held now: none for a plain click, which selects the
/// row as the timeline always does.
///
/// Feature: selection.select
pub(crate) fn select_view(state: &HistoryState<'_>, ui: &Ui<'_>, place: usize) -> Option<Intent> {
    let click = Click::of(ui.modifiers(), cfg!(target_os = "macos"));
    clicked(state, place, click).map(Intent::Graph)
}

/// The height of the bar of a selection.
pub(crate) fn bar_height(ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    theme.sizes.control_height + 2.0 * theme.sizes.spacing.small
}

/// The bar over the commits while two or more are selected: how many, the hint, and the buttons of what is done most
/// with them, those that run an operation disabled while one runs.
///
/// Feature: selection.bar
pub(crate) fn bar_view(state: &HistoryState<'_>, several: &Several, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let mut intents = Vec::new();
    ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_inactive_selection);
    let inner = Rect::new(
        rect.x + theme.sizes.spacing.medium,
        rect.y,
        (rect.width - 2.0 * theme.sizes.spacing.medium).max(0.0),
        rect.height,
    );
    let mut area = Area::with_gap(inner, theme.sizes.spacing.small);
    // The count comes first, so that buttons that do not all fit never hide it.
    let count = words::commits_selected(several.commits.len());
    let count_width = ui.measure(&count, ui.text_style(TextKind::Body));
    let _ = ui.label(area.left(count_width), &Label::new(&count));
    let entries = bar_entries(state, several);
    let buttons: Vec<Button<'_>> = entries
        .iter()
        .map(|entry| {
            let mut button = Button::secondary(&entry.label);
            if let Some(tooltip) = entry.tooltip {
                button = button.with_tooltip(tooltip);
            }
            if entry.runs_an_operation() && state.commit_box.is_running {
                button = button.disabled();
            }
            button
        })
        .collect();
    let sizes: Vec<(f32, f32)> = buttons.iter().map(|button| ui.button_size(button)).collect();
    let room = inner.width - count_width - theme.sizes.spacing.small;
    let shown = fitting(&sizes, room, theme.sizes.spacing.small);
    for index in (0..entries.len()).rev().filter(|index| shown.contains(index)) {
        let (Some(entry), Some(button), Some(&(width, height))) =
            (entries.get(index), buttons.get(index), sizes.get(index))
        else {
            continue;
        };
        let cell = place(area.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button((BAR_KEY, index), cell, button).clicked {
            intents.extend(entry_intents(entry, several));
        }
    }
    let _ = ui.label(area.rest(), &Label::new(words::SELECTION_HINT).tone(Tone::Muted));
    intents
}

/// The buttons of the bar that fit in the room beside the count, by their places: the last, **Clear selection**,
/// always; then the others in their order, each that still fits, so that those on the right are left out first. The menu of
/// the selection has them all.
fn fitting(sizes: &[(f32, f32)], room: f32, gap: f32) -> Vec<usize> {
    let Some(last) = sizes.len().checked_sub(1) else {
        return Vec::new();
    };
    let mut left = room - sizes.get(last).map_or(0.0, |size| size.0);
    let mut shown = Vec::new();
    for (index, (width, _)) in sizes.iter().enumerate().take(last) {
        if width + gap <= left {
            left -= width + gap;
            shown.push(index);
        }
    }
    shown.push(last);
    shown
}

/// Opens the menu of the selection where a commit of it was right clicked.
pub(crate) fn open_menu(ui: &mut Ui<'_>) {
    ui.open_menu(MENU_KEY, MenuAnchor::Pointer);
}

/// The menu of all the commits selected while it is open, and what the entry chosen asks.
///
/// Feature: selection.menu
pub(crate) fn menu_view(state: &HistoryState<'_>, several: &Several, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !ui.is_menu_open(MENU_KEY) {
        return Vec::new();
    }
    let (entries, separator) = menu_entries(state, several);
    let mut items = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if index == separator && index > 0 {
            items.push(MenuEntry::Separator);
        }
        let item = MenuItem::new(entry.label.as_str(), index);
        items.push(MenuEntry::Item(
            if entry.runs_an_operation() && state.commit_box.is_running {
                item.disabled()
            } else {
                item
            },
        ));
    }
    ui.menu(MENU_KEY, &items)
        .and_then(|index| entries.get(index))
        .map(|entry| entry_intents(entry, several))
        .unwrap_or_default()
}
