//! The foot of the Ask Gitardo page: the box the question is written in, which grows to 6 lines, with **Ask**, or
//! **Stop** while an answer comes. `Enter` asks; `Shift+Enter` makes a new line.

use ui::editor::Command;
use ui::layout::{Align, place};
use ui::{Border, Button, CodeEditor, Key, Label, LineWrap, NamedKey, Point, Rect, TextKind, Tone, Ui};

use super::{HelpIntent, HelpState};
use crate::intent::Intent;
use crate::text::help as words;

/// The most lines the box shows before it scrolls.
const MOST_LINES: usize = 6;

/// The height of a line of the box.
fn line_height(ui: &Ui<'_>) -> f32 {
    ui.line_height(ui.text_style(TextKind::Code)) + 4.0
}

/// The height of the foot: the lines of the box, from 1 to 6, and its margins, at least as high as a button.
pub(super) fn height(state: &HelpState<'_>, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    let lines = state.question.line_count().clamp(1, MOST_LINES);
    (line_height(ui) * crate::number::float_of_count(lines) + 2.0 * sizes.spacing.small).max(sizes.control_height)
}

/// The box and its button over a rectangle: the box disabled while an answer comes and with no AI provider.
pub(super) fn foot_view(state: &HelpState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let button = if state.is_asking {
        Button::secondary(words::STOP).with_icon(crate::icons::STOP)
    } else if state.has_provider && !state.question.text().trim().is_empty() {
        Button::primary(words::ASK)
    } else {
        Button::primary(words::ASK).disabled()
    };
    let (width, button_height) = ui.button_size(&button);
    let button_cell = place(
        Rect::new(rect.right() - width, rect.y, width, rect.height),
        (width, button_height),
        (Align::Start, Align::End),
    );
    let field = Rect::new(
        rect.x,
        rect.y,
        (rect.width - width - spacing.medium).max(0.0),
        rect.height,
    );
    let mut intents = question_box(state, ui, field);
    if ui.button("help ask", button_cell, &button).clicked {
        intents.push(Intent::Help(if state.is_asking {
            HelpIntent::Stop
        } else {
            HelpIntent::Ask
        }));
    }
    intents
}

/// The box the question is written in, with what it says while it is empty: `Enter` asks, `Shift+Enter` makes a new
/// line.
fn question_box(state: &HelpState<'_>, ui: &mut Ui<'_>, field: Rect) -> Vec<Intent> {
    let spacing = ui.theme().sizes.spacing;
    let is_disabled = state.is_asking || !state.has_provider;
    draw_frame(ui, field);
    let inner = Rect::new(
        field.x,
        field.y + spacing.small,
        field.width,
        (field.height - 2.0 * spacing.small).max(0.0),
    );
    let editor = CodeEditor {
        name: words::PLACEHOLDER,
        buffer: state.question,
        cursor: state.cursor,
        colors: &[],
        marks: &[],
        current_mark: None,
        lit_line: None,
        is_read_only: is_disabled,
        reveal: None,
        focus: (!is_disabled).then_some(state.focus),
        gutter: 0,
        tab_width: 4,
        wrap: LineWrap::Words,
        has_line_numbers: false,
    };
    let answer = ui.code_editor("help question", inner, &editor);
    if state.question.text().is_empty() {
        let at = Point::new(answer.places.text.x, inner.y);
        let line = Rect::new(at.x, at.y, answer.places.text.width, line_height(ui));
        let placeholder = Label::new(words::PLACEHOLDER).kind(TextKind::Code).tone(Tone::Muted);
        let _ = ui.label(line, &placeholder);
    }
    let has_focus = answer.response.has_focus;
    keys(ui, answer.commands, (has_focus, is_disabled))
}

/// The ground and the border of the box.
fn draw_frame(ui: &mut Ui<'_>, field: Rect) {
    let theme = *ui.theme();
    ui.fill_rounded_rect(field, theme.sizes.radius.medium, theme.colors.input_background);
    ui.border(
        field,
        Border {
            width: theme.sizes.border,
            color: theme.colors.input_border,
            radius: theme.sizes.radius.medium,
        },
    );
}

/// What the keys ask of the box: `Enter`, which the editor makes a new line, asks; `Shift+Enter`, which it leaves,
/// makes the new line; the rest edits the text.
fn keys(ui: &Ui<'_>, asked: Vec<Command>, (has_focus, is_disabled): (bool, bool)) -> Vec<Intent> {
    let asks = asked.contains(&Command::NewLine);
    let mut commands: Vec<Command> = asked
        .into_iter()
        .filter(|command| *command != Command::NewLine)
        .collect();
    let new_line = has_focus
        && ui
            .unclaimed_keys()
            .iter()
            .any(|key| key.key == Key::Named(NamedKey::Enter) && key.modifiers.shift() && !key.modifiers.command());
    if new_line && !is_disabled {
        commands.push(Command::NewLine);
    }
    let mut intents = Vec::new();
    if !commands.is_empty() {
        intents.push(Intent::Help(HelpIntent::Write(commands)));
    }
    if asks && !is_disabled {
        intents.push(Intent::Help(HelpIntent::Ask));
    }
    intents
}
