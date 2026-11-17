//! The list of the settings page: each section under its name, its settings one under the other, or only the settings
//! changed or experimental, kept to those that answer the search (`settings.search`); scrolled to a place once when
//! the page is asked to show it (`settings.reveal`), and the section whose name was last scrolled past marked
//! (`settings.sections`).

use std::time::Duration;

use ui::{Label, Rect, TextKind, Tone, Ui};

use super::layout::{ListItem, ListLayout, RowMetrics};
use super::row::{row_height, setting_row};
use super::{SettingsAnchor, SettingsList, SettingsPageState, scroll_to};
use crate::intent::Intent;
use crate::text::settings::{self as words, SettingsSection};

/// How long a setting the page was opened at stays marked, fading.
const MARKED_FOR: Duration = Duration::from_secs(1);

/// The list laid out for a width: the rows listed, each section under its name when the list holds the sections.
fn list_layout(state: &SettingsPageState<'_>, ui: &Ui<'_>, width: f32) -> ListLayout {
    let metrics = RowMetrics::new(ui);
    let has_headings = state.page.list == SettingsList::Sections;
    let mut heights = Vec::new();
    let mut last = None;
    for index in state.page.listed(state.rows) {
        let Some(row) = state.rows.get(index) else {
            continue;
        };
        if has_headings
            && let Some(section) = row.info.section
            && last != Some(section)
        {
            heights.push((ListItem::Heading(section), metrics.heading));
            last = Some(section);
        }
        heights.push((ListItem::Setting(index), row_height(ui, row, width)));
    }
    ListLayout::of(&heights, metrics.between)
}

/// The key of the scroll of the list: the sections keep where they were while the search or another list is shown,
/// so that `Escape` and a second click bring them back as they were left.
fn scroll_key(state: &SettingsPageState<'_>) -> (SettingsList, bool) {
    (state.page.list, !state.page.query.is_empty())
}

/// Where the list is asked to go, once for each number: the place asked for last, or its top once what is typed
/// changed after it.
fn asked_place(state: &SettingsPageState<'_>, layout: &ListLayout, height: f32) -> Option<(f32, u64)> {
    let page = state.page;
    match page.reveal {
        Some((anchor, serial)) if serial > page.searched => {
            scroll_to(layout, anchor, height, state.rows).map(|offset| (offset, serial))
        }
        _ if !page.query.is_empty() => Some((0.0, page.searched)),
        _ => None,
    }
}

/// Draws the list in a rectangle and returns the section marked, none while the list holds no sections.
pub(super) fn list_view(
    state: &SettingsPageState<'_>,
    ui: &mut Ui<'_>,
    rect: Rect,
    intents: &mut Vec<Intent>,
) -> Option<SettingsSection> {
    let theme = *ui.theme();
    let width = (rect.width - theme.sizes.scrollbar - theme.sizes.spacing.medium).max(0.0);
    let layout = list_layout(state, ui, width);
    if layout.items.is_empty() {
        nothing_found(state, ui, rect);
        return None;
    }
    let key = ("settings list", scroll_key(state));
    let mut since = None;
    if let Some((offset, serial)) = asked_place(state, &layout, rect.height) {
        let went = ui.scroll_to_once(key, serial, offset);
        since = state.page.reveal.map(|(anchor, _)| (anchor, went));
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
    if state.page.list != SettingsList::Sections {
        return None;
    }
    let offset = crate::number::narrow(response.offset.1);
    let asked = state.page.reveal.and_then(|(anchor, _)| match anchor {
        SettingsAnchor::Section(section) => Some(section),
        SettingsAnchor::Setting(_) => None,
    });
    layout.marked_at(offset, (rect.height, asked))
}

/// What the list says when nothing answers the search: `No setting matches "<what was typed>"`.
fn nothing_found(state: &SettingsPageState<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let said = if state.page.query.is_empty() {
        words::NOTHING_LISTED.to_owned()
    } else {
        words::nothing_matches(&state.page.query)
    };
    let line = Rect::new(rect.x, rect.y, rect.width, ui.theme().sizes.row_height);
    let _ = ui.label(line, &Label::new(&said).tone(Tone::Muted));
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
    let line = Rect::new(rect.x, rect.y, rect.width, ui.theme().sizes.row_height);
    let _ = ui.label(line, &Label::new(section.name()).kind(TextKind::Heading));
}
