//! The sections of the settings page (`settings.sections`): a button for each on the left of the page, the one whose
//! name was last scrolled past marked; on a narrow page, a button at the top of the list that names it and opens the
//! menu of the sections.

use ui::layout::{Align, Area, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Label, MenuEntry, MenuItem, Rect, Sense, TextKind, Tone, Ui,
};

use super::{SettingsAnchor, SettingsIntent};
use crate::icons;
use crate::intent::Intent;
use crate::text::settings::SettingsSection;

/// The button of a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionButton {
    /// The section.
    pub section: SettingsSection,
    /// How many settings it has.
    pub count: usize,
}

/// What the column of the sections shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SectionsState {
    /// Where the column is.
    pub rect: Rect,
    /// The button of each section, in their order.
    pub buttons: Vec<SectionButton>,
    /// The section whose name was last scrolled past in the list.
    pub marked: Option<SettingsSection>,
}

/// The icon of a section.
pub(super) fn section_icon(section: SettingsSection) -> Icon {
    match section {
        SettingsSection::General => icons::SETTINGS,
        SettingsSection::Appearance => icons::APPEARANCE,
        SettingsSection::Timeline => icons::TIMELINE,
        SettingsSection::Branches => icons::BRANCH,
        SettingsSection::PullAndCommits => icons::COMMIT,
        SettingsSection::Worktrees => icons::WORKTREE,
        SettingsSection::FilesAndBlame => icons::BLAME,
        SettingsSection::AiAndConflicts => icons::DRAFTED,
        SettingsSection::TerminalAndTools => icons::TERMINAL,
    }
}

/// The column of the sections: a button for each, with its icon and its name, the one marked in the color of the
/// selection; a click scrolls the list to the name of its section.
///
/// Feature: settings.sections
pub fn sections_view(state: &SectionsState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let step = theme.sizes.control_height + theme.sizes.spacing.tiny;
    let height = step * crate::number::float_of_count(state.buttons.len());
    let rect = state.rect;
    let (_, intents) = ui.scroll_area("settings sections", rect, (rect.width, height), |ui, inside| {
        let mut intents = Vec::new();
        let mut column = Area::with_gap(inside, theme.sizes.spacing.tiny);
        for button in &state.buttons {
            let rect = column.top(theme.sizes.control_height);
            let is_marked = state.marked == Some(button.section);
            if section_button(ui, rect, button, is_marked) {
                intents.push(Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
                    button.section,
                ))));
            }
        }
        intents
    });
    intents
}

/// A button of the column of the sections; says whether it was clicked.
fn section_button(ui: &mut Ui<'_>, rect: Rect, button: &SectionButton, is_marked: bool) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("section", button.section));
    let response = ui.interact(id, rect, Sense::CLICK);
    let background = if is_marked {
        Some(theme.colors.list_inactive_selection)
    } else if response.hovered {
        Some(theme.colors.list_hover)
    } else {
        None
    };
    if let Some(color) = background {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, color);
    }
    if is_marked {
        let bar = Rect::new(
            rect.x,
            rect.y + spacing.medium,
            spacing.tiny,
            rect.height - spacing.medium * 2.0,
        );
        ui.fill_rounded_rect(bar, spacing.tiny / 2.0, theme.colors.accent);
    }
    let mut parts = Area::with_gap(
        Rect::new(
            rect.x + spacing.medium,
            rect.y,
            rect.width - spacing.medium * 2.0,
            rect.height,
        ),
        spacing.medium,
    );
    let tone = if is_marked { Tone::Normal } else { Tone::Muted };
    ui.icon_in(parts.left(theme.sizes.icon), section_icon(button.section), tone);
    let name = button.section.name();
    let kind = if is_marked { TextKind::Medium } else { TextKind::Body };
    let _ = ui.label(parts.rest(), &Label::new(name).kind(kind));
    let description = Description {
        is_selected: Some(is_marked),
        ..Description::new(AccessRole::Button, name)
    };
    ui.describe(id, rect, &description);
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    response.clicked
}

/// The button at the top of the list of a narrow page: it names the section marked and opens the menu of the sections.
pub(super) fn sections_menu(ui: &mut Ui<'_>, rect: Rect, marked: Option<SettingsSection>) -> Vec<Intent> {
    let marked = marked.unwrap_or(SettingsSection::General);
    let entries: Vec<MenuEntry<'_, SettingsSection>> = SettingsSection::ALL
        .iter()
        .map(|section| {
            let mut item = MenuItem::new(section.name(), *section);
            item.icon = Some(section_icon(*section));
            MenuEntry::Item(item)
        })
        .collect();
    let button = Button::secondary(marked.name())
        .kind(ButtonKind::Outline)
        .with_icon(section_icon(marked))
        .with_trailing(icons::CHOOSE);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    match ui.menu_button("sections menu", cell, &button, &entries) {
        Some(section) => vec![Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
            section,
        )))],
        None => Vec::new(),
    }
}
