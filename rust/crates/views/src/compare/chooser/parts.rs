//! The parts of the comparison chooser: its two boxes, the field that filters its list and keeps the keyboard, the
//! list of the sides offered, and the comparisons opened before at its foot.

use model::compare::Side;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Border, Button, ButtonKind, InputBox, Key, Label, List, MenuEntry, MenuItem, Message, Modifiers,
    NamedKey, Rect, Reveal, RevealPlace, Row, SelectionMode, Sense, Severity, Steering, TextKind, Tone, Ui,
};

use super::{
    ChooserBox, ChooserGroup, ChooserIntent, ChooserSide, ChooserState, ListRow, TypedCommit, chooser, heading, icon,
    list_rows,
};
use crate::icons;
use crate::intent::Intent;
use crate::text::compare as words;

/// **From**, the arrows that swap the two, and **To**.
pub(super) fn boxes(state: &ChooserState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let spacing = ui.theme().sizes.spacing;
    let swap = Button::icon(icons::SWAP, words::SWAP_TOOLTIP);
    let (swap_width, swap_height) = ui.button_size(&swap);
    let each = ((row.width - swap_width - 2.0 * spacing.medium) * 0.5).max(0.0);
    let mut parts = Area::with_gap(row, spacing.medium);
    if side_box(state, ChooserBox::From, ui, parts.left(each)) {
        intents.push(chooser(ChooserIntent::Focus(ChooserBox::From)));
    }
    let cell = place(
        parts.left(swap_width),
        (swap_width, swap_height),
        (Align::Center, Align::Center),
    );
    if ui.button("chooser swap", cell, &swap).clicked {
        intents.push(chooser(ChooserIntent::Swap));
    }
    if side_box(state, ChooserBox::To, ui, parts.left(each)) {
        intents.push(chooser(ChooserIntent::Focus(ChooserBox::To)));
    }
    intents
}

/// A box: its heading, the icon of its side, its name and its short hash, or its placeholder; the box being chosen
/// has the border of the focus. Whether it was clicked.
fn side_box(state: &ChooserState<'_>, which: ChooserBox, ui: &mut Ui<'_>, rect: Rect) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("chooser box", which));
    let response = ui.interact(id, rect, Sense::CLICK);
    let radius = theme.sizes.radius.medium;
    ui.fill_rounded_rect(rect, radius, theme.colors.input_background);
    let is_chosen = state.choosing == which;
    let (width, color) = if is_chosen {
        (theme.sizes.focus_ring, theme.colors.focus_ring)
    } else {
        (theme.sizes.border, theme.colors.input_border)
    };
    ui.border(rect, Border { width, radius, color });
    let heading = if which == ChooserBox::From {
        words::FROM
    } else {
        words::TO
    };
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.large, 0.0)), spacing.medium);
    let heading_width = ui.measure(heading, ui.text_style(TextKind::Detail));
    let _ = ui.label(
        parts.left(heading_width),
        &Label::new(heading).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let described = box_content(state, (which, heading), ui, &mut parts);
    ui.describe(id, rect, &ui::Description::new(AccessRole::Button, &described));
    response.clicked
}

/// What a box holds, its icon, its name and its short hash, or its placeholder; gives what it says to the screen reader.
fn box_content(
    state: &ChooserState<'_>,
    (which, heading): (ChooserBox, &str),
    ui: &mut Ui<'_>,
    parts: &mut Area,
) -> String {
    let Some(held) = state.held(which) else {
        let placeholder = if which == ChooserBox::From {
            words::CHOOSE_FIRST
        } else {
            words::CHOOSE_SECOND
        };
        let _ = ui.label(parts.rest(), &Label::new(placeholder).tone(Tone::Muted));
        return format!("{heading} {placeholder}");
    };
    ui.icon_in(parts.left(ui.theme().sizes.icon), icon(held), Tone::Normal);
    let hash = match &held.side.side {
        Side::Commit(commit) if commit.short() != held.side.name => commit.short(),
        Side::Commit(_) | Side::WorkingTree => "",
    };
    let name_width = ui.measure(&held.side.name, ui.text_style(TextKind::Strong));
    let _ = ui.label(
        parts.left(name_width),
        &Label::new(&held.side.name).kind(TextKind::Strong),
    );
    let _ = ui.label(parts.rest(), &Label::new(hash).kind(TextKind::Hash).tone(Tone::Muted));
    format!("{heading} {}", held.side.name)
}

/// The field that filters the list, which keeps the keyboard: the arrows, `Page Up` and `Page Down` light another row,
/// `Home` and `End` the first and the last while nothing is typed, `Tab` chooses the other box, and `Enter` chooses
/// the row lit or opens the comparison.
pub(super) fn field_view(
    state: &ChooserState<'_>,
    offered: &[ChooserSide],
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let mut input = InputBox::new(state.typed, words::FIND_SIDE).with_icon(icons::SEARCH);
    if state.typed_commit == TypedCommit::NoCommit && offered.is_empty() {
        input = input.with_message(Message {
            text: words::UNKNOWN_REVISION,
            severity: Severity::Error,
        });
    }
    let id = ui.id("chooser field");
    ui.request_focus_once(id, state.serial);
    let response = ui.input_box("chooser field", rect, &input);
    let mut intents = Vec::new();
    if let Some(text) = response.changed_text {
        intents.push(chooser(ChooserIntent::Type(text)));
    }
    let last = offered.len().saturating_sub(1);
    let lit = state.lit.min(last);
    for key in ui.unclaimed_keys_of(id) {
        let page = 10;
        let moved = match (key.key, key.modifiers) {
            (Key::Named(NamedKey::ArrowDown), Modifiers::NONE) => (lit + 1).min(last),
            (Key::Named(NamedKey::ArrowUp), Modifiers::NONE) => lit.saturating_sub(1),
            (Key::Named(NamedKey::PageDown), Modifiers::NONE) => (lit + page).min(last),
            (Key::Named(NamedKey::PageUp), Modifiers::NONE) => lit.saturating_sub(page),
            _ => continue,
        };
        ui.claim_key(id, key);
        intents.push(chooser(ChooserIntent::Light(moved)));
    }
    intents.extend(pressed_keys(state, (lit, last), ui));
    if response.is_submitted {
        intents.extend(enter(state, offered, lit));
    }
    intents
}

/// `Home`, `End` while nothing is typed, and `Tab`, which the field and the dialog take before it sees them.
fn pressed_keys(state: &ChooserState<'_>, (lit, last): (usize, usize), ui: &Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    for key in ui.key_presses() {
        let intent = match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Home), Modifiers::NONE) if state.typed.is_empty() && lit != 0 => {
                ChooserIntent::Light(0)
            }
            (Key::Named(NamedKey::End), Modifiers::NONE) if state.typed.is_empty() && lit != last => {
                ChooserIntent::Light(last)
            }
            (Key::Named(NamedKey::Tab), Modifiers::NONE | Modifiers::SHIFT) => {
                ChooserIntent::Focus(state.choosing.other())
            }
            _ => continue,
        };
        intents.push(chooser(intent));
    }
    intents
}

/// What `Enter` in the field does: the comparison opened once both boxes hold a side and nothing is typed, else the
/// row lit chosen.
fn enter(state: &ChooserState<'_>, offered: &[ChooserSide], lit: usize) -> Option<Intent> {
    if state.can_compare() && state.typed.trim().is_empty() {
        return Some(chooser(ChooserIntent::Compare));
    }
    offered
        .get(lit)
        .map(|side| chooser(ChooserIntent::Choose(Box::new(side.clone()))))
}

/// The list of the sides offered under their headings, with its bar, the row lit brought into sight once each time it
/// changes; a click on a row chooses it.
pub(super) fn list_view(state: &ChooserState<'_>, offered: &[ChooserSide], ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let rows = list_rows(offered);
    let lit = state.lit.min(offered.len().saturating_sub(1));
    let lit_row = rows.iter().position(|row| *row == ListRow::Side(lit));
    let shown = List {
        name: words::SIDES,
        row_count: rows.len(),
        row_height: ui.theme().sizes.row_height + ui.theme().sizes.spacing.small,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let steering = Steering {
        reveal: lit_row.map(|row| Reveal {
            serial: state.serial,
            row,
            place: RevealPlace::Nearest,
        }),
        ..Steering::default()
    };
    let mut intents = Vec::new();
    let _ = ui.steered_list("chooser list", (rect, &shown), &steering, |ui, row| {
        match rows.get(row.index) {
            Some(ListRow::Heading(group)) => {
                let _ = ui.label(
                    row.rect,
                    &Label::new(heading(*group)).kind(TextKind::Detail).tone(Tone::Muted),
                );
            }
            Some(ListRow::Side(index)) => {
                if let Some(side) = offered.get(*index) {
                    intents.extend(side_row(state, (side, *index == lit), ui, row));
                }
            }
            None => {}
        }
    });
    intents
}

/// A side offered: its icon, its name, its short hash, the subject of a recent commit, and how long ago its commit was
/// made; lit while the arrows are on it, and chosen by a click.
fn side_row(
    state: &ChooserState<'_>,
    (side, is_lit): (&ChooserSide, bool),
    ui: &mut Ui<'_>,
    row: &Row,
) -> Option<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("chooser row", &side.side.name, side.group));
    let response = ui.interact(id, row.rect, Sense::CLICK | Sense::HOVER);
    if is_lit {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let mut parts = Area::with_gap(inset(row.rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
    ui.icon_in(parts.left(theme.sizes.icon), icon(side), Tone::Muted);
    let name = if side.group == ChooserGroup::Typed {
        words::use_revision(&side.side.name)
    } else {
        side.side.name.clone()
    };
    if let Some(date) = side.date {
        let ago = crate::text::dates::relative_date(date, state.now);
        let width = ui.measure(&ago, ui.text_style(TextKind::Detail));
        let _ = ui.label(
            parts.right(width),
            &Label::new(&ago).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    let name_width = ui.measure(&name, ui.text_style(TextKind::Body)).min(parts.rest().width);
    let _ = ui.label(parts.left(name_width), &Label::new(&name));
    if let Side::Commit(commit) = &side.side.side
        && commit.short() != side.side.name
    {
        let width = ui.measure(commit.short(), ui.text_style(TextKind::Hash));
        let _ = ui.label(
            parts.left(width),
            &Label::new(commit.short()).kind(TextKind::Hash).tone(Tone::Muted),
        );
    }
    let _ = ui.label(parts.rest(), &Label::new(&side.subject).tone(Tone::Muted));
    ui.describe(id, row.rect, &ui::Description::new(AccessRole::Button, &name));
    response
        .clicked
        .then(|| chooser(ChooserIntent::Choose(Box::new(side.clone()))))
}

/// **Opened before** and the comparisons opened before, as many as fit on the line, the rest under **More…**; a click
/// opens one as it was.
pub(super) fn recent_line(state: &ChooserState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let spacing = ui.theme().sizes.spacing;
    let mut intents = Vec::new();
    let mut parts = Area::with_gap(row, spacing.medium);
    let heading_width = ui.measure(words::OPENED_BEFORE, ui.text_style(TextKind::Detail));
    let _ = ui.label(
        parts.left(heading_width),
        &Label::new(words::OPENED_BEFORE)
            .kind(TextKind::Detail)
            .tone(Tone::Muted),
    );
    let more = Button::secondary(words::MORE).kind(ButtonKind::Icon);
    let (more_width, _) = ui.button_size(&more);
    let said: Vec<String> = state
        .recent
        .iter()
        .map(|recent| words::opened_before(&recent.first.name, &recent.second.name, recent.path.as_deref()))
        .collect();
    let mut room = parts.rest().width;
    let mut shown = 0;
    for (index, label) in said.iter().enumerate() {
        let button = Button::secondary(label).kind(ButtonKind::Outline);
        let (width, height) = ui.button_size(&button);
        let needs = if index + 1 < said.len() {
            width + more_width + spacing.medium
        } else {
            width
        };
        if needs > room {
            break;
        }
        room -= width + spacing.medium;
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("chooser recent", index), cell, &button).clicked {
            intents.push(chooser(ChooserIntent::OpenRecent(index)));
        }
        shown += 1;
    }
    if shown < said.len() {
        let entries: Vec<MenuEntry<'_, usize>> = said
            .iter()
            .enumerate()
            .skip(shown)
            .map(|(index, label)| MenuEntry::Item(MenuItem::new(label, index)))
            .collect();
        let (_, height) = ui.button_size(&more);
        let cell = place(
            parts.left(more_width),
            (more_width, height),
            (Align::Start, Align::Center),
        );
        if let Some(index) = ui.menu_button("chooser more", cell, &more, &entries) {
            intents.push(chooser(ChooserIntent::OpenRecent(index)));
        }
    }
    intents
}
