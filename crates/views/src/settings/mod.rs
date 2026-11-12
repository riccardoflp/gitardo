//! The settings page (`settings.edit`) and the picker of the themes (`settings.theme`): every setting with its
//! description, its value and where it comes from; a switch or a choice changes it in the file chosen at the top.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    Availability, Badge, BadgeFill, Button, ButtonKind, Choice, Label, MenuEntry, MenuItem, Picker, PickerAnswer,
    PickerItem, PickerMode, Rect, TextKind, Tone, Ui,
};

use crate::intent::Intent;
use crate::text::settings::{self as words, SETTINGS, SettingInfo, SettingKind};

/// The widest the page is: the lines of the descriptions stay readable on a wide window.
const PAGE_WIDTH: f32 = 880.0;
/// The width of the column of the controls.
const CONTROL_WIDTH: f32 = 200.0;

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

/// What the settings page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsPageState<'a> {
    /// Every setting, in the order of the card.
    pub rows: &'a [SettingRow],
    /// The file changes are written in.
    pub target: SettingsTarget,
    /// Whether a repository is shown, whose file can be chosen.
    pub has_repository: bool,
}

/// The rows of every setting at its default, for a page before any file was read.
#[must_use]
pub fn default_rows() -> Vec<SettingRow> {
    SETTINGS
        .iter()
        .map(|info| SettingRow {
            info,
            value: info.default.to_owned(),
            source: SettingSource::Default,
            problem: None,
        })
        .collect()
}

/// The settings page, over the rectangle its caller clipped it to: the file changes are written in and **Edit in
/// settings.json**, which waits for the File page; then every setting, with its key, **Experimental** beside the name
/// of an experimental one, its description, where its value comes from or why a value was not taken, and a switch or
/// a choice for the settings that hold one.
///
/// Feature: settings.edit
pub fn settings_view(state: &SettingsPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let clip = ui.current_clip();
    let width = PAGE_WIDTH.min(clip.width);
    let page = inset(
        Rect::new(clip.x, clip.y, width, clip.height),
        Insets::all(spacing.large + spacing.small),
    );
    let mut area = Area::with_gap(page, spacing.large);
    target_bar(state, ui, area.top(theme.sizes.control_height), &mut intents);
    let list = area.rest();
    let row_height = row_height(ui);
    let content = (list.width, row_height * count(state.rows.len()));
    let (_, ()) = ui.scroll_area("settings rows", list, content, |ui, inside| {
        for (index, row) in state.rows.iter().enumerate() {
            let top = inside.y + row_height * count(index);
            if top + row_height < list.y || top > list.bottom() {
                continue;
            }
            let rect = Rect::new(inside.x, top, (inside.width - spacing.large * 1.5).max(0.0), row_height);
            intents.extend(setting_row(state, row, ui, rect));
        }
    });
    intents
}

/// A count of rows as a length.
fn count(rows: usize) -> f32 {
    f32::from(u16::try_from(rows).unwrap_or(u16::MAX))
}

/// The height of a setting: its name, its description and where its value comes from.
fn row_height(ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    theme.sizes.row_height * 3.0 + theme.sizes.spacing.large
}

/// The choice of the file changes are written in, and **Edit in settings.json**, which cannot be clicked yet.
fn target_bar(state: &SettingsPageState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let edit = Button::secondary(words::EDIT_IN_FILE)
        .kind(ButtonKind::Outline)
        .disabled();
    let (width, height) = ui.button_size(&edit);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    let response = ui.button("edit settings file", cell, &edit);
    ui.tooltip(&response, words::NOT_YET);
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

/// A setting: its key and **Experimental**, its description, where its value comes from, and its control on the right.
fn setting_row(state: &SettingsPageState<'_>, row: &SettingRow, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let separator = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.fill_rect(separator, theme.colors.border);
    let mut columns = Area::with_gap(rect, spacing.large);
    let control = columns.right(CONTROL_WIDTH.min(rect.width * 0.4));
    let mut lines = Area::new(columns.rest());
    let mut name = Area::with_gap(lines.top(theme.sizes.row_height), spacing.medium);
    let key_width = ui.measure(row.info.key, ui.text_style(TextKind::Strong));
    let _ = ui.label(name.left(key_width), &Label::new(row.info.key).kind(TextKind::Strong));
    if row.info.is_experimental {
        let badge = Badge {
            text: words::EXPERIMENTAL,
            icons: &[],
            tone: Tone::Warning,
            fill: BadgeFill::Tint,
        };
        let _ = ui.badge(name.rest(), &badge);
    }
    let description = Label::new(row.info.description).tone(Tone::Muted);
    let _ = ui.label(lines.top(theme.sizes.row_height), &description);
    let (said, tone) = match &row.problem {
        Some(problem) => (problem.clone(), Tone::Error),
        None => (source_words(row.source), Tone::Muted),
    };
    let _ = ui.label(
        lines.top(theme.sizes.row_height),
        &Label::new(&said).kind(TextKind::Detail).tone(tone),
    );
    let control = Rect::new(control.x, rect.y, control.width, theme.sizes.row_height * 2.0);
    setting_control(state, row, ui, control)
}

/// Where a value comes from, in words.
fn source_words(source: SettingSource) -> String {
    match source {
        SettingSource::Default => words::FROM_DEFAULT.to_owned(),
        SettingSource::User => words::source(words::FROM_USER),
        SettingSource::Repository => words::source(words::FROM_REPOSITORY),
    }
}

/// The control of a setting: a switch, a menu of its choices, or its value when the page cannot change it yet.
fn setting_control(state: &SettingsPageState<'_>, row: &SettingRow, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let is_writable = state.target == SettingsTarget::User || row.info.is_repository;
    let change = |value| {
        vec![Intent::SetSetting(SettingChange {
            key: row.info.key,
            value,
            target: state.target,
        })]
    };
    match row.info.kind {
        SettingKind::Toggle => {
            let is_on = row.value == "true";
            let mut toggle = Choice::toggle("", is_on);
            if !is_writable {
                toggle.availability = Availability::Disabled;
            }
            let side = theme.sizes.icon * 2.5;
            let cell = place(rect, (side, theme.sizes.control_height), (Align::End, Align::Center));
            let response = ui.choice(("setting", row.info.key), cell, &toggle);
            if !is_writable {
                ui.tooltip(&response, words::USER_ONLY);
            }
            if response.clicked {
                change(SettingValue::Toggle(!is_on))
            } else {
                Vec::new()
            }
        }
        SettingKind::Choice(choices) => {
            let current = row.value.trim_matches('"');
            let items: Vec<MenuEntry<'_, &'static str>> = choices
                .iter()
                .map(|choice| MenuEntry::Item(MenuItem::new(choice, *choice)))
                .collect();
            let mut button = Button::secondary(current).kind(ButtonKind::Outline);
            if !is_writable {
                button = button.disabled();
            }
            let (width, height) = ui.button_size(&button);
            let cell = place(rect, (width.min(rect.width), height), (Align::End, Align::Center));
            match ui.menu_button(("setting", row.info.key), cell, &button, &items) {
                Some(chosen) if chosen != current => change(SettingValue::Choice(chosen)),
                _ => Vec::new(),
            }
        }
        SettingKind::Number | SettingKind::Text | SettingKind::List => {
            let label = Label::new(&row.value).kind(TextKind::Code).tone(Tone::Muted);
            let width = ui.measure(&row.value, ui.text_style(TextKind::Code)).min(rect.width);
            let _ = ui.label(place(rect, (width, rect.height), (Align::End, Align::Center)), &label);
            Vec::new()
        }
    }
}

/// What the picker of the themes shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemePickerState<'a> {
    /// The value of `theme` in force: `system`.
    pub current: &'a str,
}

/// The picker of the themes: the five of `theme`, the one in force said as current; the one picked is written in the
/// user's settings file, where `theme` is read from, and the window is drawn with it as soon as the file is read again.
pub fn theme_picker_view(state: &ThemePickerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let descriptions: Vec<&str> = words::THEME_VALUES
        .iter()
        .map(|value| {
            if *value == state.current {
                words::CURRENT_THEME
            } else {
                ""
            }
        })
        .collect();
    let items: Vec<PickerItem<'_>> = words::THEME_NAMES
        .iter()
        .zip(&descriptions)
        .map(|(label, description)| PickerItem { label, description })
        .collect();
    let picker = Picker {
        placeholder: words::CHOOSE_THEME,
        items: &items,
        mode: PickerMode::Single,

        query: "",
    };
    match ui.picker("theme picker", &picker) {
        Some(PickerAnswer::Chose(index)) => match words::THEME_VALUES.get(index) {
            Some(value) => vec![
                Intent::SetSetting(SettingChange {
                    key: "theme",
                    value: SettingValue::Choice(value),
                    target: SettingsTarget::User,
                }),
                Intent::CloseThemePicker,
            ],
            None => vec![Intent::CloseThemePicker],
        },
        Some(PickerAnswer::Cancelled | PickerAnswer::ChoseMany(_)) => vec![Intent::CloseThemePicker],
        None => Vec::new(),
    }
}
