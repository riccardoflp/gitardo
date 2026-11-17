//! Where each thing of the list of the settings page goes: the name of each section, then its settings, each as tall as
//! its title, its description wrapped to the width of the list, its key and what is said under it, and its control.
//! From it, the section marked as the list scrolls (`settings.sections`) and how far the list is scrolled to show a
//! section or a setting (`settings.reveal`).

use ui::{TextKind, Ui};

use super::{SettingRow, SettingsAnchor};
use crate::text::settings::SettingsSection;

/// A thing of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListItem {
    /// The name of a section, over its settings.
    Heading(SettingsSection),
    /// A setting, by its place among the rows of the page.
    Setting(usize),
}

/// A thing of the list and where it is, from the top of the list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedItem {
    /// The thing.
    pub item: ListItem,
    /// Its top.
    pub top: f32,
    /// Its height.
    pub height: f32,
    /// The room left empty under it, at the end of a section, which is part of its height.
    pub room: f32,
}

/// The list of the settings page laid out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListLayout {
    /// Each thing, from the top.
    pub items: Vec<PlacedItem>,
    /// The height of all of them.
    pub height: f32,
}

impl ListLayout {
    /// A list of things of some heights, one under the other, with room under the last thing of each section but the
    /// last.
    #[must_use]
    pub fn of(heights: &[(ListItem, f32)], room: f32) -> ListLayout {
        let mut top = 0.0;
        let mut items: Vec<PlacedItem> = Vec::new();
        for (item, height) in heights {
            if let (ListItem::Heading(_), Some(last)) = (item, items.last_mut()) {
                last.height += room;
                last.room = room;
                top += room;
            }
            items.push(PlacedItem {
                item: *item,
                top,
                height: *height,
                room: 0.0,
            });
            top += height;
        }
        ListLayout { items, height: top }
    }

    /// The section whose name was last scrolled past at an offset: the one whose name is at the top of the list or
    /// above it, else the first. When the list is scrolled to its end, the section asked for last is marked while its
    /// name is in sight, since its name could not reach the top.
    #[must_use]
    pub fn marked_at(
        &self,
        offset: f32,
        (list_height, asked): (f32, Option<SettingsSection>),
    ) -> Option<SettingsSection> {
        let largest = (self.height - list_height).max(0.0);
        let headings = self.items.iter().filter_map(|placed| match placed.item {
            ListItem::Heading(section) => Some((section, placed.top)),
            ListItem::Setting(_) => None,
        });
        let at_end = largest > 0.0 && offset >= largest - 0.5;
        if let Some(asked) = asked.filter(|_| at_end) {
            let is_in_sight = headings
                .clone()
                .any(|(section, top)| section == asked && top >= offset - 0.5 && top < offset + list_height);
            if is_in_sight {
                return Some(asked);
            }
        }
        let mut marked = None;
        for (section, top) in headings {
            if marked.is_none() || top <= offset + 0.5 {
                marked = Some(section);
            }
        }
        marked
    }
}

/// How far the list is scrolled so that the name of a section, or the row of a setting, is at its top, or as near as the
/// end of the list allows; nothing when the list does not show it.
///
/// Feature: settings.reveal
#[must_use]
pub fn scroll_to(layout: &ListLayout, place: SettingsAnchor, list_height: f32, rows: &[SettingRow]) -> Option<f32> {
    let found = layout.items.iter().find(|placed| match (placed.item, place) {
        (ListItem::Heading(section), SettingsAnchor::Section(asked)) => section == asked,
        (ListItem::Setting(index), SettingsAnchor::Setting(key)) => {
            rows.get(index).is_some_and(|row| row.info.key == key)
        }
        _ => false,
    })?;
    let largest = (layout.height - list_height).max(0.0);
    Some(found.top.min(largest))
}

/// The sizes of the parts of a row of the list, from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowMetrics {
    /// The room above and below the lines of a row.
    pub(crate) padding: f32,
    /// The height of a line of the title or of the key.
    pub(crate) line: f32,
    /// The height of a line of the description.
    pub(crate) description_line: f32,
    /// The height of the name of a section.
    pub(crate) heading: f32,
    /// The room between the last setting of a section and the name of the next.
    pub(crate) between: f32,
}

impl RowMetrics {
    /// The sizes in the theme of a frame.
    pub(crate) fn new(ui: &Ui<'_>) -> RowMetrics {
        let sizes = ui.theme().sizes;
        RowMetrics {
            padding: sizes.spacing.medium + sizes.spacing.tiny,
            line: sizes.row_height,
            description_line: ui.line_height(ui.text_style(TextKind::Body)),
            heading: sizes.row_height + sizes.spacing.medium,
            between: sizes.spacing.huge,
        }
    }
}

/// How many lines a text takes wrapped to a width.
pub(crate) fn line_count(ui: &Ui<'_>, text: &str, width: f32) -> usize {
    ui.fonts()
        .wrap(text, ui.text_style(TextKind::Body), width.max(1.0))
        .len()
        .max(1)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    fn layout() -> ListLayout {
        ListLayout::of(
            &[
                (ListItem::Heading(SettingsSection::General), 40.0),
                (ListItem::Setting(0), 100.0),
                (ListItem::Heading(SettingsSection::Timeline), 40.0),
                (ListItem::Setting(1), 100.0),
                (ListItem::Setting(2), 100.0),
                (ListItem::Heading(SettingsSection::Branches), 40.0),
                (ListItem::Setting(3), 60.0),
            ],
            0.0,
        )
    }

    #[test]
    fn the_section_marked_is_the_last_whose_name_was_scrolled_past() {
        let layout = layout();
        assert_eq!(layout.height, 480.0);
        assert_eq!(layout.marked_at(0.0, (200.0, None)), Some(SettingsSection::General));
        assert_eq!(layout.marked_at(139.0, (200.0, None)), Some(SettingsSection::General));
        assert_eq!(layout.marked_at(140.0, (200.0, None)), Some(SettingsSection::Timeline));
        assert_eq!(layout.marked_at(280.0, (200.0, None)), Some(SettingsSection::Timeline));
        let asked = Some(SettingsSection::Branches);
        assert_eq!(layout.marked_at(280.0, (200.0, asked)), Some(SettingsSection::Branches));
    }
}
