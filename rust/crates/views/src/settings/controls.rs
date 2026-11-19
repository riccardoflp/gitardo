//! The controls of the settings page that hold more than a switch or a choice (`settings.edit`): a field of numbers,
//! of a string, of a color with its swatch, a menu of the fonts, and the lists of strings and of identities, each
//! written once, when a field loses the focus or `Enter` is pressed, or a row is added or taken out.

use std::hash::Hash;

use ui::layout::{Align, Area, place};
use ui::{Border, Button, ButtonKind, Color, InputBox, Label, MenuEntry, Message, Rect, Severity, TextKind, Tone, Ui};

use super::{Identity, SettingRow, SettingValue};
use crate::icons;
use crate::text::settings::{self as words, SettingKind};

/// The fonts that come with the program, first in the menu of the fonts.
const BUNDLED_FONTS: [&str; 2] = ["Inter", "JetBrains Mono"];

/// The widest a field of numbers is.
const NUMBER_WIDTH: f32 = 120.0;

/// What a field refuses of what is typed, said under it; nothing when it takes it.
type Check<'check> = &'check dyn Fn(&str) -> Option<String>;

/// A field of a setting holding the text of its value: says the text typed once `Enter` is pressed or the field loses
/// the focus, when it differs from the value, or is empty and `is_empty_taken`, and `check` does not refuse it; what
/// `check` refuses is said under the field.
fn field(
    ui: &mut Ui<'_>,
    key: impl Hash + Copy,
    (rect, given, placeholder): (Rect, &str, &str),
    (check, is_empty_taken): (Check<'_>, bool),
) -> Option<String> {
    let id = ui.id(key);
    let typed = ui.typed_text(id).unwrap_or_else(|| given.to_owned());
    let refusal = check(&typed);
    let mut input = InputBox::new(given, placeholder);
    if let Some(said) = &refusal {
        input = input.with_message(Message {
            text: said,
            severity: Severity::Error,
        });
    }
    let response = ui.input_box(key, rect, &input);
    let typed = ui.typed_text(id).unwrap_or_default();
    let is_done = response.is_submitted || response.has_lost_focus;
    let is_new = typed != given || (is_empty_taken && typed.is_empty());
    (is_done && is_new && check(&typed).is_none()).then_some(typed)
}

/// Nothing to refuse.
fn anything(_: &str) -> Option<String> {
    None
}

/// The control in the column on the right of a row, for a number, a string, a color, a font or JSON; returns the value
/// to write.
pub(super) fn value_control(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<SettingValue> {
    let key = ("setting", row.info.key);
    match row.info.kind {
        SettingKind::Number { least, most } => {
            let cell = place(
                rect,
                (NUMBER_WIDTH.min(rect.width), rect.height),
                (Align::End, Align::Center),
            );
            let check = move |typed: &str| match typed.trim().parse::<i64>() {
                Ok(number) if (least..=most).contains(&number) => None,
                _ => Some(words::between(least, most)),
            };
            let placeholder = words::range(least, most);
            let typed = field(ui, key, (cell, &row.text, &placeholder), (&check, false))?;
            typed.trim().parse().ok().map(SettingValue::Number)
        }
        SettingKind::Text { placeholder } => {
            field(ui, key, (rect, &row.text, placeholder), (&anything, false)).map(SettingValue::Text)
        }
        SettingKind::Json => {
            field(ui, key, (rect, &row.value, row.info.default), (&anything, false)).map(SettingValue::Json)
        }
        SettingKind::Color => color_field(ui, row, rect),
        SettingKind::Font => font_menu(ui, row, rect),
        _ => None,
    }
}

/// A color: its swatch, then its hexadecimal value in a field that refuses what is not one.
fn color_field(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<SettingValue> {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let swatch = place(
        parts.left(rect.height),
        (rect.height - 8.0, rect.height - 8.0),
        (Align::Center, Align::Center),
    );
    if let Some(color) = color_of(&row.text) {
        ui.fill_rounded_rect(swatch, theme.sizes.radius.medium, color);
    }
    let edge = Border {
        width: theme.sizes.border,
        radius: theme.sizes.radius.medium,
        color: theme.colors.border,
    };
    ui.border(swatch, edge);
    let check = |typed: &str| color_of(typed).is_none().then(|| words::NOT_A_COLOR.to_owned());
    field(
        ui,
        ("setting", row.info.key),
        (parts.rest(), &row.text, "#e5534b"),
        (&check, false),
    )
    .map(SettingValue::Text)
}

/// The color of a hexadecimal value: `#e5534b`.
fn color_of(text: &str) -> Option<Color> {
    let digits = text.trim().strip_prefix('#')?;
    if digits.len() != 6 || !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let part = |at: usize| {
        digits
            .get(at..at + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
    };
    Some(Color::rgb(part(0)?, part(2)?, part(4)?))
}

/// A font: a menu of the fonts that come with the program, then those of the system.
fn font_menu(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<SettingValue> {
    let system = ui.fonts().families();
    let names: Vec<&str> = BUNDLED_FONTS
        .iter()
        .copied()
        .chain(
            system
                .iter()
                .map(String::as_str)
                .filter(|name| !BUNDLED_FONTS.contains(name)),
        )
        .collect();
    let mut entries: Vec<MenuEntry<'_, usize>> = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if index == BUNDLED_FONTS.len() {
            entries.push(MenuEntry::Separator);
        }
        entries.push(MenuEntry::Item(icons::without_icon(name, index)));
    }
    let button = Button::secondary(&row.text)
        .kind(ButtonKind::Outline)
        .with_trailing(icons::CHOOSE);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width.min(rect.width), height), (Align::End, Align::Center));
    let chosen = ui.menu_button(("setting", row.info.key), cell, &button, &entries)?;
    names
        .get(chosen)
        .filter(|name| **name != row.text)
        .map(|name| SettingValue::Text((*name).to_owned()))
}

/// The height of the list of a row under its key: a line for each entry and one for **Add**, and the names of the
/// columns of `identities`; nothing for a setting that holds one value.
pub(super) fn block_height(ui: &Ui<'_>, row: &SettingRow) -> f32 {
    let sizes = ui.theme().sizes;
    let step = sizes.control_height + sizes.spacing.small;
    let lines = match row.info.kind {
        SettingKind::Strings => row.entries.len() + 1,
        SettingKind::Identities => row.entries.len() + 2,
        _ => return 0.0,
    };
    crate::number::float_of_count(lines) * step + sizes.spacing.medium
}

/// The list of a row under its key, for a list of strings or `identities`: a row for each entry with its fields and
/// ×, and **Add** under them; returns the list to write when one changed.
pub(super) fn block_control(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<SettingValue> {
    match row.info.kind {
        SettingKind::Strings => strings_block(ui, row, rect).map(SettingValue::Strings),
        SettingKind::Identities => identities_block(ui, row, rect).map(SettingValue::Identities),
        _ => None,
    }
}

/// A list of strings: a field and × for each, **Add** under them.
fn strings_block(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<Vec<String>> {
    let sizes = ui.theme().sizes;
    let mut lines = Area::with_gap(rect, sizes.spacing.small);
    let entries: Vec<String> = row
        .entries
        .iter()
        .map(|cells| cells.first().cloned().unwrap_or_default())
        .collect();
    let mut changed = None;
    for (index, entry) in entries.iter().enumerate() {
        let mut parts = Area::with_gap(lines.top(sizes.control_height), sizes.spacing.small);
        let remove = parts.right(sizes.control_height);
        let key = ("setting entry", row.info.key, index);
        if let Some(typed) = field(ui, key, (parts.rest(), entry, words::NEW_ENTRY), (&anything, true)) {
            let mut list = entries.clone();
            if typed.is_empty() {
                let _ = list.remove(index);
            } else if let Some(place) = list.get_mut(index) {
                *place = typed;
            }
            changed = Some(list);
        }
        if remove_button(ui, (row.info.key, index), remove) {
            let mut list = entries.clone();
            let _ = list.remove(index);
            changed = Some(list);
        }
    }
    if add_button(ui, row.info.key, lines.top(sizes.control_height)) {
        let mut list = entries;
        list.push(String::new());
        changed = Some(list);
    }
    changed
}

/// `identities`: the names of the columns, a row of name, email and signing key with × for each identity, and **Add**
/// under them.
fn identities_block(ui: &mut Ui<'_>, row: &SettingRow, rect: Rect) -> Option<Vec<Identity>> {
    let sizes = ui.theme().sizes;
    let mut lines = Area::with_gap(rect, sizes.spacing.small);
    let identities: Vec<Identity> = row.entries.iter().map(|cells| Identity::of(cells)).collect();
    let columns = |line: Rect| {
        let mut parts = Area::with_gap(line, sizes.spacing.small);
        let remove = parts.right(sizes.control_height);
        let fields = parts.rest();
        let width = (fields.width - sizes.spacing.small * 2.0) / 3.0;
        let cell = |at: f32| {
            Rect::new(
                fields.x + at * (width + sizes.spacing.small),
                fields.y,
                width,
                fields.height,
            )
        };
        ([cell(0.0), cell(1.0), cell(2.0)], remove)
    };
    let (heads, _) = columns(lines.top(sizes.control_height));
    for (head, name) in heads.iter().zip(words::IDENTITY_COLUMNS) {
        let _ = ui.label(*head, &Label::new(name).kind(TextKind::Detail).tone(Tone::Muted));
    }
    let mut changed = None;
    for (index, identity) in identities.iter().enumerate() {
        let (cells, remove) = columns(lines.top(sizes.control_height));
        for (column, (cell, placeholder)) in cells.iter().zip(words::IDENTITY_COLUMNS).enumerate() {
            let key = ("setting identity", index, column);
            let given = identity.cell(column);
            if field(ui, key, (*cell, given, placeholder), (&anything, false)).is_some() {
                let mut list = identities.clone();
                if let Some(place) = list.get_mut(index) {
                    *place = typed_identity(ui, index, identity);
                }
                changed = Some(list);
            }
        }
        if remove_button(ui, ("identities", index), remove) {
            let mut list = identities.clone();
            let _ = list.remove(index);
            changed = Some(list);
        }
    }
    if add_button(ui, "identities", lines.top(sizes.control_height)) {
        let mut list = identities;
        list.push(Identity::default());
        changed = Some(list);
    }
    changed
}

/// An identity of the table as its three fields hold it, typed or not.
fn typed_identity(ui: &Ui<'_>, index: usize, identity: &Identity) -> Identity {
    let cells: Vec<String> = (0..3)
        .map(|column| {
            let id = ui.id(("setting identity", index, column));
            ui.typed_text(id).unwrap_or_else(|| identity.cell(column).to_owned())
        })
        .collect();
    Identity::of(&cells)
}

/// × of an entry of a list; says whether it was clicked.
fn remove_button(ui: &mut Ui<'_>, key: impl Hash, rect: Rect) -> bool {
    ui.scope(key, |ui| {
        ui.button("remove", rect, &Button::icon(icons::DELETE, words::REMOVE_ENTRY))
    })
    .clicked
}

/// **Add** under a list; says whether it was clicked.
fn add_button(ui: &mut Ui<'_>, key: impl Hash, rect: Rect) -> bool {
    let button = Button::secondary(words::ADD_ENTRY)
        .kind(ButtonKind::Outline)
        .with_icon(icons::ADD);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    ui.scope(key, |ui| ui.button("add", cell, &button)).clicked
}
