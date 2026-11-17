//! The sections of the settings page (`settings.sections`): a button for each on the left of the page, the one whose
//! name was last scrolled past marked, with how many of its settings a file sets, or answer the search, and a mark in
//! the color of errors while a value of one was not taken; under them **Changed** and **Experimental**. On a narrow
//! page, a button at the top of the list that names the section marked and opens the menu of them all.

use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Icon, Label, MenuEntry, Rect, Sense, TextKind, Tone, Ui};

use super::{SettingsAnchor, SettingsIntent, SettingsList};
use crate::icons;
use crate::intent::Intent;
use crate::text::settings::{self as words, SettingsSection};

/// The button of a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionButton {
    /// The section.
    pub section: SettingsSection,
    /// How many of its settings a file sets, or answer the search while something is typed.
    pub count: usize,
    /// Whether a value of one of its settings was not taken.
    pub has_problem: bool,
    /// Whether it is dimmed: nothing of it answers the search.
    pub is_dimmed: bool,
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
    /// What the list holds, whose button is marked when it is **Changed** or **Experimental**.
    pub list: SettingsList,
}

/// What a button of the column stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Choice {
    /// A section.
    Section(SettingsSection),
    /// **Changed** or **Experimental**.
    List(SettingsList),
}

impl Choice {
    /// What a click on it asks.
    fn intent(self) -> Intent {
        Intent::Settings(match self {
            Choice::Section(section) => SettingsIntent::Reveal(SettingsAnchor::Section(section)),
            Choice::List(list) => SettingsIntent::Show(list),
        })
    }

    /// Its name and its icon.
    fn look(self) -> (&'static str, Icon) {
        match self {
            Choice::Section(section) => (section.name(), section_icon(section)),
            Choice::List(SettingsList::Experimental) => (words::EXPERIMENTAL, icons::EXPERIMENTAL),
            Choice::List(_) => (words::CHANGED, icons::EDITED),
        }
    }
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

/// The column of the sections: a button for each, with its icon, its name and its count, the one marked in the color
/// of the selection; a click scrolls the list to the name of its section. Under them, after a line, **Changed** and
/// **Experimental**, which list only those settings until clicked again.
///
/// Feature: settings.sections
pub fn sections_view(state: &SectionsState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.tiny;
    let step = theme.sizes.control_height + gap;
    let line = theme.sizes.spacing.large * 2.0;
    let height = step * crate::number::float_of_count(state.buttons.len() + 2) + line;
    let rect = state.rect;
    let (_, intents) = ui.scroll_area("settings sections", rect, (rect.width, height), |ui, inside| {
        let mut intents = Vec::new();
        let mut column = Area::with_gap(inside, gap);
        for button in &state.buttons {
            let rect = column.top(theme.sizes.control_height);
            let is_marked = state.list == SettingsList::Sections && state.marked == Some(button.section);
            if section_button(ui, rect, (Choice::Section(button.section), Some(button)), is_marked) {
                intents.push(Choice::Section(button.section).intent());
            }
        }
        let separator = column.top(line);
        let middle = Rect::new(
            separator.x,
            separator.y + separator.height / 2.0,
            separator.width,
            theme.sizes.border,
        );
        ui.fill_rect(middle, theme.colors.border);
        for list in [SettingsList::Changed, SettingsList::Experimental] {
            let rect = column.top(theme.sizes.control_height);
            if section_button(ui, rect, (Choice::List(list), None), state.list == list) {
                intents.push(Choice::List(list).intent());
            }
        }
        intents
    });
    intents
}

/// A button of the column of the sections, with the count and the mark of a section; says whether it was clicked.
fn section_button(
    ui: &mut Ui<'_>,
    rect: Rect,
    (choice, button): (Choice, Option<&SectionButton>),
    is_marked: bool,
) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("settings column", choice));
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
    let is_dimmed = button.is_some_and(|button| button.is_dimmed);
    let inner = Rect::new(
        rect.x + spacing.medium,
        rect.y,
        rect.width - spacing.medium * 2.0,
        rect.height,
    );
    let mut parts = Area::with_gap(inner, spacing.medium);
    let (name, icon) = choice.look();
    let tone = match (is_dimmed, is_marked) {
        (true, _) => Tone::Disabled,
        (false, true) => Tone::Normal,
        (false, false) => Tone::Muted,
    };
    ui.icon_in(parts.left(theme.sizes.icon), icon, tone);
    let said = button
        .map(|button| count_mark(ui, &mut parts, button))
        .unwrap_or_default();
    let kind = if is_marked { TextKind::Medium } else { TextKind::Body };
    let name_tone = if is_dimmed { Tone::Disabled } else { Tone::Normal };
    let _ = ui.label(parts.rest(), &Label::new(name).kind(kind).tone(name_tone));
    let description = Description {
        is_selected: Some(is_marked),
        is_disabled: is_dimmed,
        ..Description::new(AccessRole::Button, name).value(&said)
    };
    ui.describe(id, rect, &description);
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    response.clicked
}

/// Draws on the right of a button of a section its count, when it has one, and the mark in the color of errors while a
/// value of one of its settings was not taken; returns what they say.
fn count_mark(ui: &mut Ui<'_>, parts: &mut Area, button: &SectionButton) -> String {
    let theme = *ui.theme();
    let mut said = Vec::new();
    if button.has_problem {
        let side = theme.sizes.spacing.medium;
        let cell = parts.right(side);
        let dot = place(cell, (side, side), (Align::Center, Align::Center));
        ui.fill_rounded_rect(dot, side / 2.0, theme.colors.error);
        said.push(words::NOT_TAKEN.to_owned());
    }
    if button.count > 0 || button.is_dimmed {
        let count = button.count.to_string();
        let width = ui.measure(&count, ui.text_style(TextKind::Detail));
        let tone = if button.is_dimmed { Tone::Disabled } else { Tone::Muted };
        let _ = ui.label(
            parts.right(width),
            &Label::new(&count).kind(TextKind::Detail).tone(tone),
        );
        said.insert(0, count);
    }
    said.join(", ")
}

/// The button at the top of the list of a narrow page: it names the section marked, or the list shown, and opens the
/// menu of the sections, **Changed** and **Experimental**.
pub(super) fn sections_menu(
    ui: &mut Ui<'_>,
    rect: Rect,
    (marked, list): (Option<SettingsSection>, SettingsList),
) -> Vec<Intent> {
    let shown = match list {
        SettingsList::Sections => Choice::Section(marked.unwrap_or(SettingsSection::General)),
        other => Choice::List(other),
    };
    let choices: Vec<Choice> = SettingsSection::ALL
        .iter()
        .map(|section| Choice::Section(*section))
        .chain([
            Choice::List(SettingsList::Changed),
            Choice::List(SettingsList::Experimental),
        ])
        .collect();
    let mut entries: Vec<MenuEntry<'_, Choice>> = Vec::new();
    for choice in choices {
        if choice == Choice::List(SettingsList::Changed) {
            entries.push(MenuEntry::Separator);
        }
        let (name, icon) = choice.look();
        entries.push(MenuEntry::Item(icons::action(icon, name, choice)));
    }
    let (name, icon) = shown.look();
    let button = Button::secondary(name)
        .kind(ButtonKind::Outline)
        .with_icon(icon)
        .with_trailing(icons::CHOOSE);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    ui.menu_button("sections menu", cell, &button, &entries)
        .map(Choice::intent)
        .into_iter()
        .collect()
}
