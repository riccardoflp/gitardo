//! The right side of the Tasks tab (`tasks.view`): the terminal of the task selected, with what it printed and, once
//! it ended, how it ended and how long it ran; or, for a task never run in the session, its command, its folder and
//! **Run**.

use ui::layout::{Align, Area, place};
use ui::{Button, Label, Rect, TextKind, Tone, Ui};

use super::{Detail, TasksIntent, TasksTabState, tasks};
use crate::intent::Intent;
use crate::terminal::{TerminalIntent, terminal_area};
use crate::text::tasks_tab as words;

/// The right side over the rectangle its caller clipped it to.
pub(super) fn detail_view(state: &TasksTabState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let detail = &state.list.detail;
    let Some(reference) = &detail.reference else {
        super::list::say(ui, rect, words::NOTHING_SELECTED);
        return Vec::new();
    };
    if !detail.has_terminal {
        return never_run(state, ui, rect);
    }
    let ended = detail
        .ended
        .as_ref()
        .map(|(first, second)| (first.as_str(), second.as_str()));
    let asked = terminal_area(&state.terminal, (words::TERMINAL_AREA, ended), ui);
    asked
        .into_iter()
        .map(|asked| match asked {
            // `Enter` in the terminal of a task that ended runs it again.
            TerminalIntent::Restart => tasks(TasksIntent::Run(reference.clone())),
            other => tasks(TasksIntent::Terminal(other)),
        })
        .collect()
}

/// A task never run in the session: its name, its command, its folder, and **Run**, or why it cannot run or did not.
fn never_run(state: &TasksTabState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let detail: &Detail = &state.list.detail;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.small);
    let row = theme.sizes.row_height;
    let _ = ui.label(area.top(row), &Label::new(&detail.name).kind(TextKind::Strong));
    if !detail.command.is_empty() {
        let _ = ui.label(area.top(row), &Label::new(&detail.command).kind(TextKind::Code));
    }
    let folder = words::folder_line(&detail.folder);
    let _ = ui.label(
        area.top(row),
        &Label::new(&folder).kind(TextKind::Detail).tone(Tone::Muted),
    );
    if let Some(said) = &detail.said {
        let _ = ui.paragraph(area.top(row * 2.0), said, (TextKind::Detail, Tone::Warning));
    }
    let Some(reference) = &detail.reference else {
        return intents;
    };
    let mut run = Button::primary(words::RUN);
    if !state.list.is_trusted || detail.cannot_run {
        run = run.disabled();
    }
    let (width, height) = ui.button_size(&run);
    let cell = place(area.top(height), (width, height), (Align::Start, Align::Center));
    if ui.button("run the task selected", cell, &run).clicked {
        intents.push(tasks(TasksIntent::Run(reference.clone())));
    }
    intents
}
