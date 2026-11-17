//! A setting of the settings page (`settings.edit`): its title, **Experimental** beside it when it is, its description
//! whole, its key and where its value comes from, and its control on the right.

use ui::layout::{Align, Area, place};
use ui::{
    Availability, Badge, BadgeFill, Button, ButtonKind, Choice, Label, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui,
};

use super::layout::{RowMetrics, line_count};
use super::{SettingChange, SettingRow, SettingSource, SettingValue, SettingsPageState, SettingsTarget};
use crate::intent::Intent;
use crate::number::float_of_count;
use crate::text::settings::{self as words, SettingKind};

/// The widest the column of the controls is.
const CONTROL_WIDTH: f32 = 240.0;

/// The width of the column of the controls of a row of a width.
fn control_width(width: f32) -> f32 {
    CONTROL_WIDTH.min(width * 0.4)
}

/// The width of the column of the text of a row of a width: its title, its description and its key.
fn text_width(ui: &Ui<'_>, width: f32) -> f32 {
    (width - control_width(width) - ui.theme().sizes.spacing.huge).max(1.0)
}

/// The height of the row of a setting in a list of a width: its title, its description wrapped on as many lines as it
/// needs, and its key, with room above and below.
pub(super) fn row_height(ui: &Ui<'_>, row: &SettingRow, width: f32) -> f32 {
    let metrics = RowMetrics::new(ui);
    let description = line_count(ui, row.info.description, text_width(ui, width));
    metrics.padding * 2.0 + metrics.line * 2.0 + float_of_count(description) * metrics.description_line
}

/// Draws a setting, marked in the color of the selection as strongly as `marked` says, and returns what its control
/// asked.
pub(super) fn setting_row(
    state: &SettingsPageState<'_>,
    row: &SettingRow,
    ui: &mut Ui<'_>,
    (rect, marked): (Rect, f32),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let metrics = RowMetrics::new(ui);
    if marked > 0.0 {
        let color = theme.colors.list_selection.faded(crate::number::opacity(marked));
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, color);
    }
    let separator = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.fill_rect(separator, theme.colors.border);
    let inner = Rect::new(
        rect.x,
        rect.y + metrics.padding,
        rect.width,
        rect.height - metrics.padding * 2.0,
    );
    let mut columns = Area::with_gap(inner, theme.sizes.spacing.huge);
    let control = columns.right(control_width(rect.width));
    let mut lines = Area::new(columns.rest());
    title_line(ui, row, lines.top(metrics.line));
    let description_height =
        float_of_count(line_count(ui, row.info.description, lines.rest().width)) * metrics.description_line;
    let _ = ui.paragraph(
        lines.top(description_height),
        row.info.description,
        (TextKind::Body, Tone::Muted),
    );
    key_line(ui, row, lines.top(metrics.line));
    let control = Rect::new(control.x, control.y, control.width, theme.sizes.control_height);
    setting_control(state, row, ui, control)
}

/// The title of a setting, in strong text, with **Experimental** beside it when it is.
fn title_line(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) {
    let theme = *ui.theme();
    let mut name = Area::with_gap(rect, theme.sizes.spacing.medium);
    let title_width = ui
        .measure(row.info.title, ui.text_style(TextKind::Strong))
        .min(rect.width);
    let _ = ui.label(
        name.left(title_width),
        &Label::new(row.info.title).kind(TextKind::Strong),
    );
    if row.info.is_experimental {
        let badge = Badge {
            text: words::EXPERIMENTAL,
            icons: &[],
            tone: Tone::Warning,
            fill: BadgeFill::Tint,
        };
        let height = ui.badge_height();
        let cell = name.rest();
        let _ = ui.badge(
            Rect::new(cell.x, cell.y + (cell.height - height) / 2.0, cell.width, height),
            &badge,
        );
    }
}

/// The key of a setting, in the text of code, then where its value comes from, or why a value was not taken, in red.
fn key_line(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let key_width = ui.measure(row.info.key, ui.text_style(TextKind::Code)).min(rect.width);
    let _ = ui.label(
        parts.left(key_width),
        &Label::new(row.info.key).kind(TextKind::Code).tone(Tone::Muted),
    );
    let (said, tone) = match &row.problem {
        Some(problem) => (problem.clone(), Tone::Error),
        None => (source_words(row.source), Tone::Muted),
    };
    let _ = ui.label(parts.rest(), &Label::new(&said).kind(TextKind::Detail).tone(tone));
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
        _ => {
            let label = Label::new(&row.value)
                .kind(TextKind::Code)
                .tone(Tone::Muted)
                .align(Align::End);
            let _ = ui.label(rect, &label);
            Vec::new()
        }
    }
}
