//! The top of the Search page: the box of what is looked for with the switches of the text, the modes and the
//! scopes, and the patterns and the switches that narrow the files.

use model::file_search::{Mode, Place, Scope};
use ui::layout::{Align, Area, place};
use ui::{Axis, Button, ButtonKind, Choice, FileOrFolder, InputBox, Rect, TextKind, Ui};

use super::{FileSearchIntent, SearchPageState, Switch, results};
use crate::icons;
use crate::text::file_search as words;

/// The least width the box of what is looked for keeps beside the switches of the text; in a row with less room the
/// switches go under it.
const BOX_LEAST: f32 = 260.0;

/// The widest a box of patterns is.
const PATTERNS_MOST: f32 = 320.0;

/// The switches of the text, in their order.
const TEXT_SWITCHES: [(Switch, &str); 3] = [
    (Switch::MatchCase, words::MATCH_CASE),
    (Switch::WholeWord, words::WHOLE_WORD),
    (Switch::Regex, words::REGULAR_EXPRESSION),
];

/// The switches of the files, in their order.
const FILE_SWITCHES: [(Switch, &str); 2] = [(Switch::Gitignore, words::GITIGNORE), (Switch::Hidden, words::HIDDEN)];

/// Whether a row of this width has room for the switches of the text beside the box.
pub(super) fn has_room_for_switches(ui: &Ui<'_>, width: f32) -> bool {
    width >= BOX_LEAST + switches_width(ui, &TEXT_SWITCHES)
}

/// The box of what is looked for, which takes the keyboard each time the page is asked for, and at its right the
/// switches of the text when the row has room for them. The arrows, `Page Up` and `Page Down` move through the
/// results from the box, and `Enter` opens the one under the keyboard.
pub(super) fn query_row(state: &SearchPageState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<FileSearchIntent> {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    let mut asked = Vec::new();
    if has_room_for_switches(ui, row.width) {
        let switches = parts.right(switches_width(ui, &TEXT_SWITCHES));
        asked.extend(switches_row(state, ui, switches, &TEXT_SWITCHES));
    }
    let id = ui.id("search query");
    ui.focus_input_box_once(id, state.focus);
    let input = InputBox::new(&state.search.query, words::placeholder(state.search.mode)).with_icon(icons::SEARCH);
    let answer = ui.input_box("search query", parts.rest(), &input);
    if let Some(text) = answer.changed_text {
        asked.push(FileSearchIntent::Type(text));
    }
    for key in ui.unclaimed_keys_of(id) {
        if let Some(row) = results::moved_by(state, key) {
            ui.claim_key(id, key);
            asked.push(FileSearchIntent::Light(row));
        }
    }
    if answer.is_submitted
        && let Some(lit) = state.lit
    {
        asked.push(FileSearchIntent::Open(lit));
    }
    asked
}

/// The switches of the text in a row of their own, for a page too narrow to have them beside the box.
pub(super) fn text_switches_row(state: &SearchPageState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<FileSearchIntent> {
    switches_row(state, ui, row, &TEXT_SWITCHES)
}

/// The modes, then the scopes: **Repository**, **Changes**, and what **Search here** named, with the button that
/// takes it away. The one chosen of each stands out.
pub(super) fn choices_row(state: &SearchPageState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<FileSearchIntent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.small);
    let mut asked = Vec::new();
    for mode in Mode::EVERY {
        let is_chosen = mode == state.search.mode;
        if tab(ui, &mut parts, (("search mode", words::mode(mode)), is_chosen, None)) && !is_chosen {
            asked.push(FileSearchIntent::Mode(mode));
        }
    }
    let _ = parts.left(spacing.medium);
    let line = parts.left(theme.sizes.border);
    ui.separator(
        Rect::new(
            line.x,
            row.y + spacing.small,
            line.width,
            (row.height - 2.0 * spacing.small).max(0.0),
        ),
        Axis::Vertical,
    );
    let _ = parts.left(spacing.medium);
    for (scope, name) in [(Scope::Repository, words::REPOSITORY), (Scope::Changes, words::CHANGES)] {
        let is_chosen = scope == state.search.scope;
        if tab(ui, &mut parts, (("search scope", name), is_chosen, None)) && !is_chosen {
            asked.push(FileSearchIntent::Scope(scope));
        }
    }
    if let Some(named) = &state.search.named {
        asked.extend(named_scope(state, ui, &mut parts, named));
    }
    asked
}

/// The scope **Search here** named, by its path with the icon of its file or folder, and the button that takes it
/// away.
fn named_scope(state: &SearchPageState<'_>, ui: &mut Ui<'_>, parts: &mut Area, named: &Place) -> Vec<FileSearchIntent> {
    let mut asked = Vec::new();
    let is_chosen = state.search.scope == Scope::Named;
    let last = named.path.rsplit('/').next().unwrap_or(&named.path);
    let icon = if named.is_file {
        FileOrFolder::File(last)
    } else {
        FileOrFolder::Folder(last)
    };
    if tab(ui, parts, (("search scope", &named.path), is_chosen, Some(icon))) && !is_chosen {
        asked.push(FileSearchIntent::Scope(Scope::Named));
    }
    let side = ui.theme().sizes.control_height;
    let drop = Button::icon(icons::CLOSE, words::DROP_SCOPE).kind(ButtonKind::Icon);
    let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
    if ui.button("search drop scope", cell, &drop).clicked {
        asked.push(FileSearchIntent::DropScope);
    }
    asked
}

/// One of the modes or of the scopes, drawn as a tab: filled when it is the one chosen. Whether it was clicked.
fn tab(
    ui: &mut Ui<'_>,
    parts: &mut Area,
    ((group, name), is_chosen, icon): ((&str, &str), bool, Option<FileOrFolder<'_>>),
) -> bool {
    let kind = if is_chosen {
        ButtonKind::Secondary
    } else {
        ButtonKind::Icon
    };
    let mut button = Button::secondary(name).kind(kind);
    if let Some(icon) = icon {
        button = button.with_file_icon(icon);
    }
    let (width, height) = ui.button_size(&button);
    let width = width.min(parts.rest().width);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    ui.button((group, name), cell, &button).clicked
}

/// **Include** and **Exclude**, then the switches **.gitignore** and **Hidden** at the right.
pub(super) fn filters_row(state: &SearchPageState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<FileSearchIntent> {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    let mut asked = Vec::new();
    let switches = parts.right(switches_width(ui, &FILE_SWITCHES).min(row.width));
    asked.extend(switches_row(state, ui, switches, &FILE_SWITCHES));
    let each = ((parts.rest().width - spacing.medium) * 0.5).clamp(0.0, PATTERNS_MOST);
    let include = InputBox::new(&state.search.include, words::INCLUDE);
    if let Some(text) = ui.input_box("search include", parts.left(each), &include).changed_text {
        asked.push(FileSearchIntent::Include(text));
    }
    let exclude = InputBox::new(&state.search.exclude, words::EXCLUDE);
    if let Some(text) = ui.input_box("search exclude", parts.left(each), &exclude).changed_text {
        asked.push(FileSearchIntent::Exclude(text));
    }
    asked
}

/// The width some switches take, side by side.
fn switches_width(ui: &Ui<'_>, switches: &[(Switch, &str)]) -> f32 {
    switches.iter().map(|(_, label)| switch_width(ui, label)).sum()
}

/// The width a switch takes: its box and its words.
fn switch_width(ui: &Ui<'_>, label: &str) -> f32 {
    ui.measure(label, ui.text_style(TextKind::Body)) + 2.0 * ui.theme().sizes.control_height
}

/// Some switches side by side, from the left; a click turns one.
fn switches_row(
    state: &SearchPageState<'_>,
    ui: &mut Ui<'_>,
    row: Rect,
    switches: &[(Switch, &str)],
) -> Vec<FileSearchIntent> {
    let mut parts = Area::new(row);
    let mut asked = Vec::new();
    for (switch, label) in switches {
        let search = state.search;
        let is_on = match switch {
            Switch::MatchCase => search.match_case,
            Switch::WholeWord => search.whole_word,
            Switch::Regex => search.regex,
            Switch::Gitignore => search.gitignore,
            Switch::Hidden => search.hidden,
        };
        let rect = parts.left(switch_width(ui, label));
        if ui
            .choice(("search switch", *label), rect, &Choice::check_box(label, is_on))
            .clicked
        {
            asked.push(FileSearchIntent::Turn(*switch));
        }
    }
    asked
}
