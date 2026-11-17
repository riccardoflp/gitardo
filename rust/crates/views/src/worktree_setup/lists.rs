//! The lists of the page: the folders to link and the files and folders to copy, each pattern with what git said it
//! takes, and the commands with the buttons that move them and the commands proposed from what is committed here.

use model::worktree_setup::{PatternTake, SetupPatterns};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, InputBox, Label, Rect, TextKind, Tone, Ui};

use super::rows::Row;
use super::{SetupBox, SetupIntent, SetupPageState};
use crate::icons;
use crate::intent::Intent;
use crate::text::worktree_setup as words;

/// The widest a box that adds is.
const ADD_WIDTH: f32 = 480.0;

/// An intent of the page.
fn page(intent: SetupIntent) -> Intent {
    Intent::WorktreeSetup(intent)
}

/// A row of the lists.
pub(super) fn list_row(state: &SetupPageState<'_>, row: Row, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    match row {
        Row::PatternsTitle => {
            let _ = ui.label(rect, &Label::new(words::PATTERNS).kind(TextKind::Heading));
            Vec::new()
        }
        Row::ListTitle(which) => {
            list_title(which, ui, rect);
            Vec::new()
        }
        Row::Pattern(which, index) => pattern_row(state, (which, index), ui, rect),
        Row::Add(which) => add_row(state, which, ui, rect),
        Row::Command(index) => command_row(state, index, ui, rect),
        Row::Proposals => proposals_row(state, ui, rect),
        _ => Vec::new(),
    }
}

/// The title of a list, with its setting and what it is beside it.
fn list_title(which: SetupBox, ui: &mut Ui<'_>, rect: Rect) {
    let (title, detail) = match which {
        SetupBox::Link => (words::FOLDERS_TO_LINK, words::FOLDERS_TO_LINK_DETAIL),
        SetupBox::Copy => (words::FILES_TO_COPY, words::FILES_TO_COPY_DETAIL),
        SetupBox::Command => (words::COMMANDS, words::COMMANDS_DETAIL),
    };
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
    let width = ui.measure(title, ui.text_style(TextKind::Strong));
    let _ = ui.label(parts.left(width), &Label::new(title).kind(TextKind::Strong));
    let _ = ui.label(
        parts.rest(),
        &Label::new(detail).kind(TextKind::Detail).tone(Tone::Muted),
    );
}

/// The patterns of a list.
fn list_of(patterns: &SetupPatterns, which: SetupBox) -> &[String] {
    match which {
        SetupBox::Link => &patterns.link,
        SetupBox::Copy | SetupBox::Command => &patterns.copy,
    }
}

/// The patterns with one list made otherwise.
fn with_list(patterns: &SetupPatterns, which: SetupBox, list: Vec<String>) -> SetupPatterns {
    match which {
        SetupBox::Link => SetupPatterns {
            link: list,
            copy: patterns.copy.clone(),
        },
        SetupBox::Copy | SetupBox::Command => SetupPatterns {
            link: patterns.link.clone(),
            copy: list,
        },
    }
}

/// What git said a pattern takes, while the patterns are those it was asked about: **3 folders**, or **takes nothing
/// here** as a warning.
fn takes(state: &SetupPageState<'_>, which: SetupBox, pattern: &str) -> Option<(String, Tone)> {
    if state.chosen.patterns != state.chosen.looked {
        return None;
    }
    let preview = state.chosen.preview;
    let taken: &[PatternTake] = match which {
        SetupBox::Link => &preview.link_patterns,
        SetupBox::Copy | SetupBox::Command => &preview.copy_patterns,
    };
    let take = taken.iter().find(|take| take.pattern == pattern)?;
    if take.count == 0 {
        return Some((words::TAKES_NOTHING.to_owned(), Tone::Warning));
    }
    let said = match which {
        SetupBox::Link => words::folders(take.count),
        SetupBox::Copy | SetupBox::Command => words::files_or_folders(take.count),
    };
    Some((said, Tone::Muted))
}

/// A pattern: its text, what it takes, and **Remove**.
fn pattern_row(
    state: &SetupPageState<'_>,
    (which, index): (SetupBox, usize),
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let list = list_of(state.chosen.patterns, which);
    let Some(pattern) = list.get(index) else {
        return Vec::new();
    };
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
    let mut intents = Vec::new();
    let remove = Button::icon(icons::DELETE, words::REMOVE).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&remove);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button(("remove pattern", which, index), cell, &remove).clicked {
        let kept: Vec<String> = list
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .map(|(_, kept)| kept.clone())
            .collect();
        intents.push(page(SetupIntent::SetPatterns(with_list(
            state.chosen.patterns,
            which,
            kept,
        ))));
    }
    let width = ui.measure(pattern, ui.text_style(TextKind::Code));
    let _ = ui.label(parts.left(width), &Label::new(pattern).kind(TextKind::Code));
    if let Some((said, tone)) = takes(state, which, pattern) {
        let _ = ui.label(parts.rest(), &Label::new(&said).kind(TextKind::Detail).tone(tone));
    }
    intents
}

/// The box of a list and **Add**: Enter or the button adds what is typed, each of what is typed between commas for the
/// patterns, unless it is in the list already.
fn add_row(state: &SetupPageState<'_>, which: SetupBox, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let placeholder = match which {
        SetupBox::Link => words::LINK_PLACEHOLDER,
        SetupBox::Copy => words::COPY_PLACEHOLDER,
        SetupBox::Command => words::COMMAND_PLACEHOLDER,
    };
    let typed = state.typed.of(which);
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let add = Button::secondary(words::ADD).with_icon(icons::ADD);
    let (add_width, add_height) = ui.button_size(&add);
    let field_width = (ADD_WIDTH - add_width).min(rect.width - add_width).max(0.0);
    let field = parts.left(field_width);
    let answer = ui.input_box(("add", which), field, &InputBox::new(typed, placeholder));
    let mut intents = Vec::new();
    let text = answer.changed_text.clone().unwrap_or_else(|| typed.to_owned());
    if let Some(changed) = answer.changed_text {
        intents.push(page(SetupIntent::Typed(which, changed)));
    }
    let add = if text.trim().is_empty() { add.disabled() } else { add };
    let cell = place(
        parts.left(add_width),
        (add_width, add_height),
        (Align::Start, Align::Center),
    );
    let is_added = ui.button(("add button", which), cell, &add).clicked || answer.is_submitted;
    if is_added && !text.trim().is_empty() {
        intents.extend(added(state, which, &text));
        intents.push(page(SetupIntent::Typed(which, String::new())));
    }
    intents
}

/// What adding a text to a list asks.
fn added(state: &SetupPageState<'_>, which: SetupBox, text: &str) -> Option<Intent> {
    if which == SetupBox::Command {
        let command = text.trim().to_owned();
        if state.commands.contains(&command) {
            return None;
        }
        let mut commands = state.commands.to_vec();
        commands.push(command);
        return Some(page(SetupIntent::SetCommands(commands)));
    }
    let mut list = list_of(state.chosen.patterns, which).to_vec();
    for pattern in text.split(',').map(str::trim).filter(|pattern| !pattern.is_empty()) {
        if !list.iter().any(|kept| kept == pattern) {
            list.push(pattern.to_owned());
        }
    }
    Some(page(SetupIntent::SetPatterns(with_list(
        state.chosen.patterns,
        which,
        list,
    ))))
}

/// A command: **Move up**, **Move down**, **Remove**, its number and its text, edited where it is.
fn command_row(state: &SetupPageState<'_>, index: usize, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let Some(command) = state.commands.get(index) else {
        return Vec::new();
    };
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let mut intents = Vec::new();
    let last = state.commands.len().saturating_sub(1);
    let buttons = [
        (icons::MOVE_UP, words::MOVE_UP, index > 0),
        (icons::MOVE_DOWN, words::MOVE_DOWN, index < last),
        (icons::DELETE, words::REMOVE, true),
    ];
    for (icon, tooltip, is_enabled) in buttons {
        let button = Button::icon(icon, tooltip).kind(ButtonKind::Icon);
        let button = if is_enabled { button } else { button.disabled() };
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("command", index, tooltip), cell, &button).clicked {
            let mut commands = state.commands.to_vec();
            match icon {
                icons::MOVE_UP => commands.swap(index, index - 1),
                icons::MOVE_DOWN => commands.swap(index, index + 1),
                _ => {
                    let _ = commands.remove(index);
                }
            }
            intents.push(page(SetupIntent::SetCommands(commands)));
        }
    }
    let number = format!("{}.", index + 1);
    let number_width = ui.measure("00.", ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(number_width), &Label::new(&number).tone(Tone::Muted));
    let field = parts.left((ADD_WIDTH - number_width).max(0.0));
    let edited = ui.input_box(
        ("command text", index),
        field,
        &InputBox::new(command, words::COMMAND_PLACEHOLDER),
    );
    if let Some(text) = edited.changed_text.filter(|text| !text.trim().is_empty()) {
        let mut commands = state.commands.to_vec();
        if let Some(changed) = commands.get_mut(index) {
            *changed = text;
        }
        intents.push(page(SetupIntent::SetCommands(commands)));
    }
    intents
}

/// **Proposed from what is committed here:** and a button for each command proposed that is not in the list, which adds
/// it; its tooltip names the file it is proposed from.
fn proposals_row(state: &SetupPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let width = ui.measure(words::PROPOSED_HERE, ui.text_style(TextKind::Detail));
    let _ = ui.label(
        parts.left(width),
        &Label::new(words::PROPOSED_HERE)
            .kind(TextKind::Detail)
            .tone(Tone::Muted),
    );
    let mut intents = Vec::new();
    for proposal in state
        .proposals
        .iter()
        .filter(|proposal| !state.commands.contains(&proposal.command))
    {
        let button = Button::secondary(&proposal.command)
            .kind(ButtonKind::Pill)
            .with_tooltip(&proposal.file);
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("proposal", &proposal.command), cell, &button).clicked {
            let mut commands = state.commands.to_vec();
            commands.push(proposal.command.clone());
            intents.push(page(SetupIntent::SetCommands(commands)));
        }
    }
    intents
}
