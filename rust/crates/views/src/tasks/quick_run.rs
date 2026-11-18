//! **Run Task…** (`tasks.quick-run`): the picker of every group and every task of the repository shown, opened from
//! Quick search or with `Ctrl+Shift+T` wherever the window is.

use model::tasks::TaskReference;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Description, InputBox, Key, KeyEvent, Label, Modifiers, NamedKey, PickList, Popover, Rect, Sense,
};
use ui::{TextKind, Tone, Ui};

use super::{TasksIntent, tasks};
use crate::icons;
use crate::intent::Intent;
use crate::line::rows_of;
use crate::text::tasks_tab as words;

/// How many entries the picker shows with room enough.
const LISTED: usize = 7;

/// What the picker of the tasks shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct QuickRunState<'a> {
    /// Whether the picker is open.
    pub is_open: bool,
    /// What is typed in its box.
    pub typed: &'a str,
    /// The groups, then the tasks by source, that hold what is typed.
    pub entries: &'a [QuickRunEntry],
    /// Whether the tasks of the worktree shown are still being read.
    pub is_reading: bool,
}

/// A group or a task of the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickRunEntry {
    /// Its name.
    pub name: String,
    /// What is under its name: the command of a task, the tasks of a group.
    pub under: String,
    /// Where it is from, at the right of its row: **Groups**, **Workspace**, **npm** and the name of a package.
    pub source: String,
    /// Whether it runs, or a task of the group does.
    pub is_running: bool,
    /// What choosing it runs.
    pub choice: QuickRunChoice,
}

/// What an entry of the picker runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuickRunChoice {
    /// A task.
    Task(TaskReference),
    /// The tasks of a group, by its name.
    Group(String),
}

/// Something asked of the picker of the tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuickRunIntent {
    /// **Run Task…**, or `Ctrl+Shift+T`: the picker opens with nothing typed.
    Open,
    /// The text of its box changed.
    Type(String),
    /// `Escape`, or a press outside it: it closes and nothing runs.
    Close,
    /// `Enter` or a click on an entry: the task or the group runs, and the Tasks tab shows it.
    Choose(QuickRunChoice),
}

/// An intent of the picker.
fn asks(intent: QuickRunIntent) -> Intent {
    tasks(TasksIntent::QuickRun(intent))
}

/// Whether a key opens the picker: `Ctrl+Shift+T`, or `Cmd+Shift+T` on macOS.
#[must_use]
pub fn is_quick_run_key(key: KeyEvent) -> bool {
    matches!(key.key, Key::Character('t' | 'T')) && key.modifiers == Modifiers::CONTROL | Modifiers::SHIFT
}

/// The picker of the tasks, over the window while it is open: a box, and under it the groups of the repository first,
/// then the tasks by source as the Tasks tab orders them, each with its command under its name, where it is from at
/// its right and a spinner while it runs; what is typed leaves the entries whose words hold it. `Enter` or a click
/// chooses the entry lit, `Escape` or a press outside closes the picker. While it is closed, `Ctrl+Shift+T`
/// (`Cmd+Shift+T` on macOS) opens it, wherever the window is.
///
/// Feature: tasks.quick-run
pub fn quick_run_view(state: &QuickRunState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !state.is_open {
        let is_asked = ui.unclaimed_keys().into_iter().any(is_quick_run_key);
        return if is_asked {
            vec![asks(QuickRunIntent::Open)]
        } else {
            Vec::new()
        };
    }
    let sizes = ui.theme().sizes;
    let window = ui.window();
    let width = (sizes.row_height * 28.0)
        .min(window.width - 2.0 * sizes.spacing.huge)
        .max(0.0);
    let listed = state.entries.len().clamp(1, LISTED);
    let height = sizes.control_height + 2.0 * sizes.row_height * rows_of(listed) + 3.0 * sizes.spacing.medium;
    let popover = Popover {
        anchor: Rect::new(
            window.x + (window.width - width) * 0.5,
            window.y + sizes.spacing.huge,
            width,
            0.0,
        ),
        size: (width, height),
        name: words::PICKER,
        side: ui::PopoverSide::Below,
    };
    let response = ui.popover("run task picker", &popover, |ui, rect, is_new| {
        body(state, ui, rect, is_new)
    });
    let mut intents = response.inner;
    if response.is_dismissed {
        intents.push(asks(QuickRunIntent::Close));
    }
    intents
}

/// The inside of the picker: its box, with the keyboard, and its entries.
fn body(state: &QuickRunState<'_>, ui: &mut Ui<'_>, rect: Rect, is_new: bool) -> Vec<Intent> {
    let sizes = ui.theme().sizes;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(inset(rect, Insets::all(sizes.spacing.medium)), sizes.spacing.medium);
    let box_rect = area.top(sizes.control_height);
    if is_new {
        let input = ui.id("run task box");
        ui.request_focus(input);
    }
    // The list reads the keys of the box before the box does, so that the arrows move through the list.
    intents.extend(entries(state, ui, area.rest()));
    let input = InputBox::new(state.typed, words::PICKER_BOX).with_icon(icons::SEARCH);
    if let Some(text) = ui.input_box("run task box", box_rect, &input).changed_text {
        intents.push(asks(QuickRunIntent::Type(text)));
    }
    let escape = Key::Named(NamedKey::Escape);
    if ui.unclaimed_keys().iter().any(|key| key.key == escape) {
        intents.push(asks(QuickRunIntent::Close));
    }
    intents
}

/// The entries, in a list that scrolls, whose row lit the keys of the box move and `Enter` chooses; or what the
/// picker says with none.
fn entries(state: &QuickRunState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    if state.entries.is_empty() {
        let said = match (state.is_reading, state.typed.is_empty()) {
            (true, _) => words::READING,
            (false, true) => words::NOTHING_TO_RUN,
            (false, false) => words::NO_MATCH,
        };
        let line = Rect::new(rect.x, rect.y, rect.width, ui.theme().sizes.row_height);
        let _ = ui.label(line, &Label::new(said).tone(Tone::Muted));
        return None;
    }
    let headings = vec![false; state.entries.len()];
    let list = PickList {
        headings: &headings,
        typed: state.typed,
        keys_from: ui.id("run task box"),
        is_tall: true,
    };
    let response = ui.pick_list("tasks to run", rect, &list);
    let mut chosen = response.chosen;
    ui.clip(rect, |ui| {
        for (index, row) in &response.rows {
            if let Some(entry) = state.entries.get(*index)
                && entry_row(ui, *row, (*index, entry))
            {
                chosen = Some(*index);
            }
        }
    });
    let entry = state.entries.get(chosen?)?;
    Some(asks(QuickRunIntent::Choose(entry.choice.clone())))
}

/// A row of the picker: a spinner while the task or a task of the group runs, its name with its command or its tasks
/// under it, and where it is from at its right. Says whether it was clicked.
fn entry_row(ui: &mut Ui<'_>, rect: Rect, (index, entry): (usize, &QuickRunEntry)) -> bool {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("task to run", index));
    let response = ui.interact(id, rect, Sense::CLICK);
    if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let mut line = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
    let source = ui.measure(&entry.source, ui.text_style(TextKind::Detail));
    let _ = ui.label(
        line.right(source),
        &Label::new(&entry.source).kind(TextKind::Detail).tone(Tone::Muted),
    );
    if entry.is_running {
        let icon = (theme.sizes.icon, theme.sizes.icon);
        ui.spinner(place(line.left(theme.sizes.icon), icon, (Align::Start, Align::Center)));
    }
    let words = line.rest();
    let half = words.height * 0.5;
    let gap = spacing.tiny;
    let name = Rect::new(words.x, words.y + gap, words.width, half - gap);
    let under = Rect::new(words.x, words.y + half, words.width, half - gap);
    let _ = ui.label(name, &Label::new(&entry.name));
    let _ = ui.label(
        under,
        &Label::new(&entry.under).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let said = words::row_name(&entry.name, &entry.under, &entry.source);
    ui.describe(id, rect, &Description::new(AccessRole::ListItem, &said));
    response.clicked
}
