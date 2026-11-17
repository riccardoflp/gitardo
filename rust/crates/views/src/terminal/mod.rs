//! A terminal in the bottom panel (`terminal.view`, D53): the screen of the shell of the repository tab shown, in the
//! font of code and the colors of the theme's terminal palette, taking every key but the window's; the pointer selects,
//! the wheel scrolls back, and a change of size is told to the shell.

mod draw;
pub mod keys;

use model::terminal::Screen;
use ui::layout::{Align, Area, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Key, KeyEvent, Label, Modifiers, NamedKey, PointerButton, Rect, Sense,
    TextKind, Tone, Ui, WidgetEvent,
};

use self::draw::{Cells, draw_screen};
use self::keys::{KeyUse, key_use, paste_bytes};
use crate::intent::Intent;
use crate::text::panel as words;

/// How the shell of the terminal shown stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalStatus<'a> {
    /// No repository is shown: there is no folder to start a shell in.
    NoRepository,
    /// The shell starts.
    Starting,
    /// The shell runs.
    Running,
    /// The shell ended, with its exit code when the system gave one.
    Ended(Option<i32>),
    /// The shell could not start, with what the system said.
    NotStarted(&'a str),
}

/// What the Terminal tab of the bottom panel shows.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalState<'a> {
    /// The names of the terminals of the repository tab shown, in order: `1: pwsh`.
    pub names: Vec<String>,
    /// Which of them is shown.
    pub shown: usize,
    /// What it shows.
    pub screen: Option<&'a Screen>,
    /// How its shell stands.
    pub status: TerminalStatus<'a>,
    /// The size the terminals of the window were last given, in columns and rows.
    pub size: Option<(u16, u16)>,
    /// A number that changes each time the keyboard is to go to the terminal.
    pub focus: u64,
}

/// Something asked of the terminals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalIntent {
    /// **New terminal**, or `` Ctrl+Shift+` ``: another terminal in the repository tab shown, shown.
    New,
    /// **Kill the terminal**: the shell of the terminal shown ended, and the terminal taken away.
    Kill,
    /// A terminal of the list shown.
    Show(usize),
    /// Keys typed, or text pasted, for the shell of the terminal shown.
    Input(Vec<u8>),
    /// The room of the terminal changed: its columns and rows.
    Resize(u16, u16),
    /// The wheel scrolled what is shown by some lines, up into what it printed before for a positive number.
    Scroll(i32),
    /// The main button pressed on a cell, a row and a column: what was selected is no longer.
    Press((usize, usize)),
    /// The pointer dragged to a cell: the cells from the one pressed to it are selected.
    Drag((usize, usize)),
    /// A double click on a cell: its word is selected.
    SelectWord((usize, usize)),
    /// `Enter` once the shell ended: a new shell in its place.
    Restart,
}

/// The terminal shown, over the rectangle given: its screen, or what it says while there is none, and, once its shell
/// ended, the end and how to start another.
///
/// Feature: terminal.view
pub fn terminal_view(state: &TerminalState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let id = ui.id("terminal");
    if state.focus > 0 {
        ui.request_focus_once(id, state.focus);
    }
    let (response, events) = ui.interact_with_events(
        id,
        rect,
        Sense::CLICK | Sense::DRAG | Sense::SCROLL | Sense::FOCUS | Sense::ALL_KEYS,
    );
    let text = state.screen.map(Screen::text).unwrap_or_default();
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::TextInput, words::TERMINAL_AREA).value(&text),
    );
    let style = ui.text_style(TextKind::Code);
    let cells = Cells {
        origin: ui::Point::new(rect.x, rect.y),
        width: (ui.measure("MMMMMMMMMM", style) / 10.0).max(1.0),
        height: ui.line_height(style).max(1.0),
        style,
    };
    let size = grid_size(rect, cells);
    let mut intents = Vec::new();
    if state.size != Some(size) && state.status != TerminalStatus::NoRepository {
        intents.push(terminal(TerminalIntent::Resize(size.0, size.1)));
    }
    intents.extend(events_of(state, ui, (response.id, events), cells));
    if let Some(screen) = state.screen {
        draw_screen(ui, screen, cells, response.has_focus);
    }
    status_lines(state, ui, rect, cells);
    intents
}

/// An intent of the terminals.
fn terminal(intent: TerminalIntent) -> Intent {
    Intent::Terminal(intent)
}

/// How many columns and rows fit in a rectangle.
fn grid_size(rect: Rect, cells: Cells) -> (u16, u16) {
    let count = |length: f32, size: f32| to_u16((length / size).floor().clamp(1.0, f32::from(u16::MAX)));
    (count(rect.width, cells.width).max(2), count(rect.height, cells.height))
}

/// A count of cells, already whole and within the range of a u16, as one.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "whole and within the range of a u16"
)]
fn to_u16(count: f32) -> u16 {
    count as u16
}

/// What the events given to the terminal ask.
fn events_of(
    state: &TerminalState<'_>,
    ui: &mut Ui<'_>,
    (id, events): (ui::WidgetId, Vec<WidgetEvent>),
    cells: Cells,
) -> Vec<Intent> {
    let screen = state.screen;
    let extent = screen.map_or((1, 1), |screen| (screen.columns.max(1), screen.rows.len().max(1)));
    let has_ended = matches!(state.status, TerminalStatus::Ended(_));
    let mut intents = Vec::new();
    for event in events {
        let asked = match event {
            WidgetEvent::Pressed {
                button: PointerButton::Primary,
                position,
                click_count,
                ..
            } => {
                let cell = cells.cell_at(position, extent);
                Some(if click_count >= 2 {
                    TerminalIntent::SelectWord(cell)
                } else {
                    TerminalIntent::Press(cell)
                })
            }
            WidgetEvent::Dragged { position, .. } => Some(TerminalIntent::Drag(cells.cell_at(position, extent))),
            WidgetEvent::Wheel { delta } => wheel(delta.y, cells.height, screen),
            WidgetEvent::Key(key) => key_intent(ui, id, key, (screen, has_ended)),
            WidgetEvent::Text(text) if !has_ended => Some(TerminalIntent::Input(text.into_bytes())),
            WidgetEvent::Paste(text) if !has_ended => Some(TerminalIntent::Input(paste_bytes(&text, screen))),
            _ => None,
        };
        intents.extend(asked.map(terminal));
    }
    intents
}

/// What a key asks, claimed when the terminal uses it; copying is done here.
fn key_intent(
    ui: &mut Ui<'_>,
    id: ui::WidgetId,
    key: KeyEvent,
    (screen, has_ended): (Option<&Screen>, bool),
) -> Option<TerminalIntent> {
    if has_ended {
        let is_enter = key.key == Key::Named(NamedKey::Enter) && key.modifiers == Modifiers::NONE;
        if is_enter {
            ui.claim_key(id, key);
        }
        return is_enter.then_some(TerminalIntent::Restart);
    }
    match key_use(key, screen) {
        KeyUse::Send(bytes) => {
            ui.claim_key(id, key);
            Some(TerminalIntent::Input(bytes))
        }
        KeyUse::Copy => {
            ui.claim_key(id, key);
            if let Some(text) = screen.and_then(|screen| screen.selected_text.clone()) {
                ui.copy_to_clipboard(text);
            }
            None
        }
        KeyUse::Typed => {
            ui.claim_key(id, key);
            None
        }
        KeyUse::Window => None,
    }
}

/// What the wheel asks: what was printed before scrolled into view, or, for a program that draws on the second screen
/// such as a pager, the arrows it reads to scroll.
fn wheel(down: f32, row: f32, screen: Option<&Screen>) -> Option<TerminalIntent> {
    let lines = (down / row).round();
    let lines = if lines == 0.0 && down != 0.0 {
        down.signum()
    } else {
        lines
    };
    #[allow(clippy::cast_possible_truncation, reason = "a turn of the wheel is a few lines")]
    let lines = lines.clamp(-1000.0, 1000.0) as i32;
    if lines == 0 {
        return None;
    }
    if screen.is_some_and(|screen| screen.is_alternate_screen) {
        let arrow: &[u8] = if lines > 0 { b"\x1b[B" } else { b"\x1b[A" };
        let arrow = if screen.is_some_and(|screen| screen.is_application_cursor) {
            if lines > 0 {
                b"\x1bOB".as_slice()
            } else {
                b"\x1bOA".as_slice()
            }
        } else {
            arrow
        };
        return Some(TerminalIntent::Input(arrow.repeat(lines.unsigned_abs() as usize)));
    }
    Some(TerminalIntent::Scroll(-lines))
}

/// What the terminal says besides its screen: why there is none, or, under what the shell printed, that it ended.
fn status_lines(state: &TerminalState<'_>, ui: &mut Ui<'_>, rect: Rect, cells: Cells) {
    let (first, second) = match state.status {
        TerminalStatus::NoRepository => (words::NO_REPOSITORY.to_owned(), None),
        TerminalStatus::NotStarted(reason) => (format!("{}: {reason}", words::NOT_STARTED), None),
        TerminalStatus::Ended(code) => (words::shell_ended(code), Some(words::PRESS_ENTER)),
        TerminalStatus::Starting | TerminalStatus::Running => return,
    };
    let used = state.screen.map_or(0, |screen| {
        let last = screen
            .rows
            .iter()
            .rposition(|row| !row.is_empty())
            .map_or(0, |row| row + 1);
        screen.cursor.map_or(last, |cursor| last.max(cursor.row + 1))
    });
    let rows = (rect.height / cells.height).floor().max(2.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "at least 2, the rows of a panel"
    )]
    let rows = rows as usize;
    let row = used.min(rows.saturating_sub(2));
    let height = ui.theme().sizes.row_height.max(cells.height);
    let top = cells.rect(row, 0, 1).y;
    let label = Label::new(&first).kind(TextKind::Detail).tone(Tone::Muted);
    let _ = ui.label(Rect::new(rect.x, top, rect.width, height), &label);
    if let Some(second) = second {
        let label = Label::new(second).kind(TextKind::Detail).tone(Tone::Muted);
        let _ = ui.label(Rect::new(rect.x, top + height, rect.width, height), &label);
    }
}

/// The buttons of the terminals at the top of the panel, from its right: **Kill the terminal**, the list of the
/// terminals when there are more than one, **New terminal**.
pub(crate) fn terminal_buttons(state: &TerminalState<'_>, ui: &mut Ui<'_>, parts: &mut Area) -> Vec<Intent> {
    let mut intents = Vec::new();
    if state.status == TerminalStatus::NoRepository {
        return intents;
    }
    let height = ui.theme().sizes.control_height;
    if !state.names.is_empty() {
        let kill = Button::icon(crate::icons::DELETE, words::KILL_TERMINAL).kind(ButtonKind::Icon);
        if ui.button("kill the terminal", parts.right(height), &kill).clicked {
            intents.push(terminal(TerminalIntent::Kill));
        }
    }
    if state.names.len() > 1 {
        for (index, name) in state.names.iter().enumerate().rev() {
            let kind = if index == state.shown {
                ButtonKind::Secondary
            } else {
                ButtonKind::Icon
            };
            let button = Button::secondary(name).kind(kind);
            let (width, button_height) = ui.button_size(&button);
            let cell = place(parts.right(width), (width, button_height), (Align::End, Align::Center));
            if ui.button(("terminal", index), cell, &button).clicked && index != state.shown {
                intents.push(terminal(TerminalIntent::Show(index)));
            }
        }
    }
    let new = Button::icon(crate::icons::ADD, words::NEW_TERMINAL).kind(ButtonKind::Icon);
    if ui.button("new terminal", parts.right(height), &new).clicked {
        intents.push(terminal(TerminalIntent::New));
    }
    intents
}

/// Whether a key is `` Ctrl+Shift+` ``, which starts a new terminal (`` Cmd+Shift+` `` on macOS).
#[must_use]
pub fn is_new_terminal_key(key: Key, modifiers: Modifiers) -> bool {
    key == Key::Character('`') && modifiers.command() && modifiers.shift() && !modifiers.contains(Modifiers::ALT)
}

/// What `` Ctrl+Shift+` `` asks when it was pressed in this frame and no widget used it.
#[must_use]
pub(crate) fn new_terminal_shortcut(ui: &Ui<'_>) -> Option<Intent> {
    ui.unclaimed_keys()
        .iter()
        .any(|key| is_new_terminal_key(key.key, key.modifiers))
        .then_some(terminal(TerminalIntent::New))
}
