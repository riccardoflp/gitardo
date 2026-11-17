//! The list of the settings page: each section under its name, its settings one under the other, scrolled to a place
//! once when the page is asked to show it (`settings.reveal`), and the section whose name was last scrolled past marked
//! (`settings.sections`).

use std::time::Duration;

use ui::{Label, Rect, TextKind, Ui};

use super::layout::{ListItem, ListLayout, RowMetrics};
use super::row::{row_height, setting_row};
use super::{SettingsAnchor, SettingsPageState, scroll_to};
use crate::intent::Intent;
use crate::text::settings::SettingsSection;

/// How long a setting the page was opened at stays marked, fading.
const MARKED_FOR: Duration = Duration::from_secs(1);

/// The rows the list shows, by their place among the rows of the page, in the order of the sections, then of the card.
fn listed(state: &SettingsPageState<'_>) -> Vec<(SettingsSection, usize)> {
    let mut listed = Vec::new();
    for section in SettingsSection::ALL {
        for (index, row) in state.rows.iter().enumerate() {
            if row.info.section == Some(section) {
                listed.push((section, index));
            }
        }
    }
    listed
}

/// The list laid out for a width: the name of each section that has rows listed, then its rows.
fn list_layout(state: &SettingsPageState<'_>, ui: &Ui<'_>, width: f32) -> ListLayout {
    let metrics = RowMetrics::new(ui);
    let mut heights = Vec::new();
    let mut last = None;
    for (section, index) in listed(state) {
        if last != Some(section) {
            heights.push((ListItem::Heading(section), metrics.heading));
            last = Some(section);
        }
        if let Some(row) = state.rows.get(index) {
            heights.push((ListItem::Setting(index), row_height(ui, row, width)));
        }
    }
    ListLayout::of(&heights, metrics.between)
}

/// Draws the list in a rectangle and returns the section marked.
pub(super) fn list_view(
    state: &SettingsPageState<'_>,
    ui: &mut Ui<'_>,
    rect: Rect,
    intents: &mut Vec<Intent>,
) -> Option<SettingsSection> {
    let theme = *ui.theme();
    let width = (rect.width - theme.sizes.scrollbar - theme.sizes.spacing.medium).max(0.0);
    let layout = list_layout(state, ui, width);
    let key = "settings list";
    let mut since = None;
    if let Some((anchor, serial)) = state.page.reveal
        && let Some(offset) = scroll_to(&layout, anchor, rect.height, state.rows)
    {
        since = Some((anchor, ui.scroll_to_once(key, serial, offset)));
    }
    let (response, ()) = ui.scroll_area(key, rect, (rect.width, layout.height), |ui, inside| {
        for placed in &layout.items {
            let top = inside.y + placed.top;
            if top + placed.height < rect.y || top > rect.bottom() {
                continue;
            }
            let item = Rect::new(inside.x, top, width, placed.height - placed.room);
            match placed.item {
                ListItem::Heading(section) => heading(ui, item, section),
                ListItem::Setting(index) => {
                    if let Some(row) = state.rows.get(index) {
                        let fade = marked_fade(ui, since, row.info.key);
                        intents.extend(setting_row(state, row, ui, (item, fade)));
                    }
                }
            }
        }
    });
    let offset = crate::number::narrow(response.offset.1);
    let asked = state.page.reveal.and_then(|(anchor, _)| match anchor {
        SettingsAnchor::Section(section) => Some(section),
        SettingsAnchor::Setting(_) => None,
    });
    layout.marked_at(offset, (rect.height, asked))
}

/// How strongly the row of a setting is marked: fully when the page was just opened at it, then less and less for a
/// second; nothing for another row.
fn marked_fade(ui: &mut Ui<'_>, since: Option<(SettingsAnchor, Duration)>, key: &str) -> f32 {
    let Some((SettingsAnchor::Setting(asked), since)) = since else {
        return 0.0;
    };
    if asked != key {
        return 0.0;
    }
    let elapsed = ui.now().saturating_sub(since);
    if elapsed >= MARKED_FOR {
        return 0.0;
    }
    let frame = ui.now() + Duration::from_millis(16);
    ui.request_frame_at(frame);
    1.0 - elapsed.as_secs_f32() / MARKED_FOR.as_secs_f32()
}

/// The name of a section, over its settings.
fn heading(ui: &mut Ui<'_>, rect: Rect, section: SettingsSection) {
    let theme = *ui.theme();
    let line = Rect::new(rect.x, rect.y, rect.width, theme.sizes.row_height);
    let _ = ui.label(line, &Label::new(section.name()).kind(TextKind::Heading));
}
