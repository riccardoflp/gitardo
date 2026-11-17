//! A setting of the settings page (`settings.edit`): its title, **Experimental** beside it when it is, its description
//! whole, its key and where its value comes from, its control on the right or its list under the key, and, when the
//! file of the tab shown sets it, a bar on its left and ↺, which takes it back to its default (`settings.reset`).

use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{
    AccessRole, Badge, BadgeFill, Button, ButtonKind, Choice, Description, Label, MenuEntry, MenuItem, Rect, TextKind,
    Tone, Ui,
};

use super::controls::{block_control, block_height, value_control};
use super::layout::{RowMetrics, line_count};
use super::{SettingChange, SettingRow, SettingSource, SettingValue, SettingsPageState, SettingsTarget};
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::number::float_of_count;
use crate::text::settings::{self as words, SettingKind};

/// The widest the column of the controls is.
const CONTROL_WIDTH: f32 = 280.0;

/// The width of the column of the controls of a row of a width.
fn control_width(width: f32) -> f32 {
    CONTROL_WIDTH.min(width * 0.4)
}

/// How far the text of a row is from its left edge, which leaves room for the bar of a setting a file sets.
fn indent(ui: &Ui<'_>) -> f32 {
    ui.theme().sizes.spacing.large
}

/// The width of the column of the text of a row of a width: its title, its description and its key.
fn text_width(ui: &Ui<'_>, width: f32) -> f32 {
    (width - indent(ui) - control_width(width) - ui.theme().sizes.spacing.huge).max(1.0)
}

/// The height of the row of a setting in a list of a width: its title, its description wrapped on as many lines as it
/// needs, its key, and its list under them, with room above and below.
pub(super) fn row_height(ui: &Ui<'_>, row: &SettingRow, width: f32) -> f32 {
    let metrics = RowMetrics::new(ui);
    let description = line_count(ui, row.info.description, text_width(ui, width));
    metrics.padding * 2.0
        + metrics.line * 2.0
        + float_of_count(description) * metrics.description_line
        + block_height(ui, row)
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
    let shifted = row_ground(ui, row, (rect, marked));
    let mut columns = Area::with_gap(shifted, theme.sizes.spacing.huge);
    let control = columns.right(control_width(rect.width));
    let mut lines = Area::new(columns.rest());
    title_line(ui, row, lines.top(metrics.line));
    let description_lines = line_count(ui, row.info.description, lines.rest().width);
    let description = lines.top(float_of_count(description_lines) * metrics.description_line);
    let _ = ui.paragraph(description, row.info.description, (TextKind::Body, Tone::Muted));
    key_line(ui, row, lines.top(metrics.line));
    let mut intents = Vec::new();
    let control_line = Rect::new(control.x, control.y, control.width, theme.sizes.control_height);
    let mut cells = Area::with_gap(control_line, theme.sizes.spacing.small);
    let reset = cells.left(theme.sizes.control_height);
    if row.is_set_here && reset_button(ui, row, reset) {
        intents.push(reset_intent(state, row));
    }
    let below = Rect::new(
        shifted.x,
        lines.rest().y + theme.sizes.spacing.medium,
        shifted.width,
        block_height(ui, row),
    );
    let value = setting_control(row, ui, cells.rest()).or_else(|| block_control(ui, row, below));
    intents.extend(value.map(|value| {
        Intent::SetSetting(SettingChange {
            key: row.info.key,
            value,
            target: state.target,
        })
    }));
    intents
}

/// The ground of a row: the color of the selection while it is marked, the line under it and the bar of a setting the
/// file of the tab sets. Returns where its content goes, inside its padding and right of the bar. The row is a group
/// named by its title for the screen reader, selected while it is marked.
fn row_ground(ui: &mut Ui<'_>, row: &SettingRow, (rect, marked): (Rect, f32)) -> Rect {
    let theme = *ui.theme();
    let id = ui.id(("setting row", row.info.key));
    let description = Description::new(AccessRole::Group, row.info.title).selected(marked > 0.0);
    ui.describe(id, rect, &description);
    let padding = RowMetrics::new(ui).padding;
    if marked > 0.0 {
        let color = theme.colors.list_selection.faded(crate::number::opacity(marked));
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, color);
    }
    let border = theme.sizes.border;
    ui.fill_rect(
        Rect::new(rect.x, rect.bottom() - border, rect.width, border),
        theme.colors.border,
    );
    let inner = Rect::new(rect.x, rect.y + padding, rect.width, rect.height - padding * 2.0);
    if row.is_set_here {
        let bar = Rect::new(rect.x, inner.y, theme.sizes.spacing.tiny + 1.0, inner.height);
        ui.fill_rounded_rect(bar, 1.5, theme.colors.accent);
    }
    Rect::new(inner.x + indent(ui), inner.y, inner.width - indent(ui), inner.height)
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

/// The key of a setting, in the text of code, then where its value comes from, or why a value was not taken, in red,
/// or, in **User**, the value of the repository's file when it sets it too.
fn key_line(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let key_width = ui.measure(row.info.key, ui.text_style(TextKind::Code)).min(rect.width);
    let _ = ui.label(
        parts.left(key_width),
        &Label::new(row.info.key).kind(TextKind::Code).tone(Tone::Muted),
    );
    let (said, tone) = match (&row.problem, &row.set_by_repository) {
        (Some(problem), _) => (problem.clone(), Tone::Error),
        (None, Some(value)) => (words::set_by_repository(value), Tone::Warning),
        (None, None) => (source_words(row.source), Tone::Muted),
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

/// ↺ of a setting the file of the tab shown sets; says whether it was clicked.
fn reset_button(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> bool {
    let name = words::reset(row.info.title);
    ui.button(("reset", row.info.key), rect, &Button::icon(icons::UNDO, &name))
        .clicked
}

/// The intent of ↺: `settings.reset` of the key in the file of the tab shown.
fn reset_intent(state: &SettingsPageState<'_>, row: &SettingRow) -> Intent {
    let file = match state.target {
        SettingsTarget::User => "user",
        SettingsTarget::Repository => "repository",
    };
    let answers = Answers::new()
        .with(AnswerKey("setting"), Answer::Choice(row.info.key.to_owned()))
        .with(AnswerKey("settings-file"), Answer::Choice(file.to_owned()));
    Intent::Feature(FeatureIntent::RunWith(
        FeatureId("settings.reset"),
        Target::Head,
        answers,
    ))
}

/// The control of a setting on the right of its row: a switch, a menu of its choices, or a field; returns the value
/// to write.
fn setting_control(row: &SettingRow, ui: &mut Ui<'_>, rect: Rect) -> Option<SettingValue> {
    let theme = *ui.theme();
    match row.info.kind {
        SettingKind::Toggle => {
            let is_on = row.value == "true";
            let side = theme.sizes.icon * 2.5;
            let cell = place(rect, (side, theme.sizes.control_height), (Align::End, Align::Center));
            let response = ui.choice(("setting", row.info.key), cell, &Choice::toggle("", is_on));
            response.clicked.then_some(SettingValue::Toggle(!is_on))
        }
        SettingKind::Choice(choices) => {
            let current = row.text.as_str();
            let items: Vec<MenuEntry<'_, &'static str>> = choices
                .iter()
                .map(|choice| MenuEntry::Item(MenuItem::new(choice, *choice)))
                .collect();
            let shown = if current.is_empty() { words::NOT_SET } else { current };
            let button = Button::secondary(shown)
                .kind(ButtonKind::Outline)
                .with_trailing(icons::CHOOSE);
            let (width, height) = ui.button_size(&button);
            let cell = place(rect, (width.min(rect.width), height), (Align::End, Align::Center));
            match ui.menu_button(("setting", row.info.key), cell, &button, &items) {
                Some(chosen) if chosen != current => Some(SettingValue::Choice(chosen)),
                _ => None,
            }
        }
        _ => value_control(ui, row, rect),
    }
}
