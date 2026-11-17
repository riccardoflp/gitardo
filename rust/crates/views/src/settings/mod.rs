//! The settings page (`settings.edit`), in sections (`settings.sections`), and the picker of the themes
//! (`settings.theme`): every setting with its title, its description, its key and where its value comes from, under the
//! name of its section; a switch or a choice changes it in the file chosen at the top.

mod layout;
mod list;
mod row;
mod sections;
mod theme;

use model::contract::{FeatureId, Target};
use model::worktree_setup::{SettingsPlace, SetupSave};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Availability, Button, ButtonKind, Choice, Label, Rect, TextKind, Tone, Ui};

pub use layout::{ListItem, ListLayout, PlacedItem, scroll_to};
pub use sections::{SectionButton, SectionsState, sections_view};
pub use theme::{ThemePickerState, theme_picker_view};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::settings::{self as words, SettingInfo, SettingsSection, settings};

/// The widest the page is: the lines of the descriptions stay readable on a wide window.
const PAGE_WIDTH: f32 = 960.0;
/// The width of the column of the sections.
const SECTIONS_WIDTH: f32 = 200.0;
/// The narrowest page that has the column of the sections; a narrower one has a menu of them at the top of the list.
const NARROW: f32 = 640.0;

/// The file the settings page writes changes in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsTarget {
    /// The user's `settings.json`.
    #[default]
    User,
    /// `.gitardo/settings.json` of the repository shown, for the settings marked **Repository**.
    Repository,
}

/// A value the page writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingValue {
    /// `true` or `false`.
    Toggle(bool),
    /// One of the words of a choice.
    Choice(&'static str),
}

/// A value to write in a settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SettingChange {
    /// The key: `graph.firstParent`.
    pub key: &'static str,
    /// The value.
    pub value: SettingValue,
    /// The file.
    pub target: SettingsTarget,
}

/// Where the value in force comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingSource {
    /// No file sets it.
    #[default]
    Default,
    /// The user's file.
    User,
    /// The repository's file.
    Repository,
}

/// A setting, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    /// What the card says of it.
    pub info: &'static SettingInfo,
    /// The value in force, as written in a settings file: `true`, `"name"`, `500`.
    pub value: String,
    /// Where it comes from.
    pub source: SettingSource,
    /// Why a value written in a file was not taken, when one was not: `Repository settings: not a number. The default
    /// is used.`
    pub problem: Option<String>,
}

/// A place of the list of the settings page: the name of a section, or the row of a setting by its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsAnchor {
    /// The name of a section.
    Section(SettingsSection),
    /// The row of a setting.
    Setting(&'static str),
}

/// Something asked of the settings page itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsIntent {
    /// Scroll the list to the name of a section, or to the row of a setting (`settings.reveal`).
    Reveal(SettingsAnchor),
}

/// What the window keeps of the settings page between frames: where the list was last asked to go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsPage {
    /// The place the list was last asked to go to, and the number of that request, so that the list goes there once.
    pub reveal: Option<(SettingsAnchor, u64)>,
    /// How many requests were made.
    pub serial: u64,
}

impl SettingsPage {
    /// The page after something asked of it.
    pub fn apply(&mut self, intent: SettingsIntent) {
        match intent {
            SettingsIntent::Reveal(anchor) => {
                self.serial += 1;
                self.reveal = Some((anchor, self.serial));
            }
        }
    }
}

/// What the settings page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsPageState<'a> {
    /// Every setting, in the order of the card.
    pub rows: &'a [SettingRow],
    /// The file changes are written in.
    pub target: SettingsTarget,
    /// Whether a repository is shown, whose file can be chosen.
    pub has_repository: bool,
    /// What the window keeps of the page.
    pub page: SettingsPage,
}

/// The rows of every setting at its default, for a page before any file was read.
#[must_use]
pub fn default_rows() -> Vec<SettingRow> {
    settings()
        .map(|info| SettingRow {
            info,
            value: info.default.to_owned(),
            source: SettingSource::Default,
            problem: None,
        })
        .collect()
}

/// The settings page, centered in the rectangle its caller clipped it to and at most 960 logical pixels wide: the file
/// changes are written in and **Edit in settings.json**, which opens it in a File page; then the column of the sections
/// on the left and the list of the settings on the right, each setting with its title, **Experimental** beside the
/// title of an experimental one, its description, its key, where its value comes from or why a value was not taken,
/// and a switch or a choice for the settings that hold one.
///
/// Feature: settings.edit
pub fn settings_view(state: &SettingsPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let clip = ui.current_clip();
    let width = PAGE_WIDTH.min(clip.width);
    let page = Rect::new(clip.x + (clip.width - width) / 2.0, clip.y, width, clip.height);
    ui.region("settings page", (page, AccessRole::Region, words::PAGE), |ui| {
        let mut area = Area::with_gap(inset(page, Insets::all(spacing.large + spacing.small)), spacing.large);
        target_bar(state, ui, area.top(theme.sizes.control_height), &mut intents);
        let body = area.rest();
        if page.width < NARROW {
            let mut column = Area::with_gap(body, spacing.medium);
            let menu = column.top(theme.sizes.control_height);
            let marked = list::list_view(state, ui, column.rest(), &mut intents);
            intents.extend(sections::sections_menu(ui, menu, marked));
        } else {
            let mut columns = Area::with_gap(body, spacing.huge);
            let column = columns.left(SECTIONS_WIDTH);
            let marked = list::list_view(state, ui, columns.rest(), &mut intents);
            let sections = SectionsState {
                rect: column,
                buttons: section_buttons(state.rows),
                marked,
            };
            intents.extend(sections_view(&sections, ui));
        }
    });
    intents
}

/// The button of each section.
fn section_buttons(rows: &[SettingRow]) -> Vec<SectionButton> {
    SettingsSection::ALL
        .iter()
        .map(|section| SectionButton {
            section: *section,
            count: rows.iter().filter(|row| row.info.section == Some(*section)).count(),
        })
        .collect()
}

/// The choice of the file changes are written in, and **Edit in settings.json**, which opens that file in a File page
/// (`worktree-setup.edit-settings-file`).
fn target_bar(state: &SettingsPageState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let edit = Button::secondary(words::EDIT_IN_FILE).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&edit);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("edit settings file", cell, &edit).clicked {
        intents.push(edit_file(state.target));
    }
    let label_width = ui.measure(words::WRITE_IN, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(label_width), &Label::new(words::WRITE_IN).tone(Tone::Muted));
    let targets = [
        (SettingsTarget::User, words::FROM_USER, true),
        (SettingsTarget::Repository, words::FROM_REPOSITORY, state.has_repository),
    ];
    for (target, label, is_enabled) in targets {
        let mut choice = Choice::radio(label, state.target == target);
        if !is_enabled {
            choice.availability = Availability::Disabled;
        }
        let choice_width = ui.measure(label, ui.text_style(TextKind::Body)) + theme.sizes.icon * 2.0;
        if ui
            .choice(("settings target", label), parts.left(choice_width), &choice)
            .clicked
        {
            intents.push(Intent::ChooseSettingsFile(target));
        }
    }
}

/// The intent of **Edit in settings.json**: the operation that opens the file chosen, made first when the repository
/// has none.
fn edit_file(target: SettingsTarget) -> Intent {
    let place = match target {
        SettingsTarget::User => SettingsPlace::User,
        SettingsTarget::Repository => SettingsPlace::Repository,
    };
    let save = SetupSave {
        place,
        ..SetupSave::default()
    };
    let feature = FeatureId("worktree-setup.edit-settings-file");
    Intent::Feature(FeatureIntent::Run(feature, Target::SetupSettings(Box::new(save))))
}
