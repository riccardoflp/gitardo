//! The box **Edit** opens on a card (`review.choose`): the lines to put in place of the conflict, written in an editor
//! of code, from what is proposed or from the two sides one after the other; **Use these lines** or `Ctrl+Enter`
//! takes them, **Cancel** or `Escape` leaves them.

use ui::editor::{Buffer, Cursor};
use ui::layout::{Align, place};
use ui::{Border, Button, CodeEditor, Key, LineWrap, Modifiers, NamedKey, Rect, TextKind, Ui};

use super::card::CardState;
use super::{ConflictKey, ReviewIntent};
use crate::intent::Intent;
use crate::text::review as words;

/// How many lines the box shows before it scrolls.
const MOST_LINES: usize = 12;
/// How many lines it shows at least.
const FEWEST_LINES: usize = 3;

/// The lines being written on a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WritingBox<'a> {
    /// The conflict they are written for.
    pub key: &'a ConflictKey,
    /// Their text.
    pub buffer: &'a Buffer,
    /// The caret and the selection.
    pub cursor: Cursor,
    /// A number that changes when the box is to take the keyboard.
    pub focus: Option<u64>,
}

/// The height of the box in a card: its lines, up to 12, and the row of its buttons.
pub(super) fn height(writing: &WritingBox<'_>, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    let lines = writing.buffer.line_count().clamp(FEWEST_LINES, MOST_LINES);
    line_height(ui) * crate::number::float_of_count(lines) + sizes.spacing.small + sizes.control_height
}

/// The height of a line of the editor.
fn line_height(ui: &Ui<'_>) -> f32 {
    ui.line_height(ui.text_style(TextKind::Code)) + 4.0
}

/// The box over the rectangle its caller clipped it to: the editor, then **Use these lines** and **Cancel**.
pub(super) fn write_box_view(state: &CardState<'_>, writing: &WritingBox<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let sizes = ui.theme().sizes;
    let editor_height = (rect.height - sizes.spacing.small - sizes.control_height).max(line_height(ui));
    let editor_rect = Rect::new(rect.x, rect.y, rect.width, editor_height);
    let colors = ui.theme().colors;
    ui.border(
        editor_rect,
        Border {
            width: sizes.border,
            radius: 0.0,
            color: colors.border,
        },
    );
    let editor = CodeEditor {
        name: words::LINES_BOX,
        buffer: writing.buffer,
        cursor: writing.cursor,
        colors: &[],
        marks: &[],
        current_mark: None,
        lit_line: None,
        is_read_only: state.is_running,
        reveal: None,
        focus: writing.focus,
        gutter: 0,
        tab_width: 4,
        wrap: LineWrap::Cut,
    };
    let answer = ui.code_editor(("review lines", writing.key), editor_rect, &editor);
    let mut intents = Vec::new();
    if !answer.commands.is_empty() {
        intents.push(Intent::Review(ReviewIntent::WriteLines(answer.commands)));
    }
    let keys = ui.unclaimed_keys();
    let has_focus = answer.response.has_focus;
    let uses = has_focus
        && keys
            .iter()
            .any(|key| key.key == Key::Named(NamedKey::Enter) && key.modifiers.command() && !key.modifiers.shift());
    let leaves = has_focus
        && keys
            .iter()
            .any(|key| key.key == Key::Named(NamedKey::Escape) && key.modifiers == Modifiers::NONE);
    let row = Rect::new(
        rect.x,
        editor_rect.bottom() + sizes.spacing.small,
        rect.width,
        sizes.control_height,
    );
    let (use_clicked, cancel_clicked) = buttons(state, row, ui);
    if (use_clicked || uses) && !state.is_running {
        intents.push(Intent::Review(ReviewIntent::UseLines));
    }
    if cancel_clicked || leaves {
        intents.push(Intent::Review(ReviewIntent::CancelLines));
    }
    intents
}

/// **Use these lines** and **Cancel** in a row, from its left: whether each was clicked.
fn buttons(state: &CardState<'_>, row: Rect, ui: &mut Ui<'_>) -> (bool, bool) {
    let gap = ui.theme().sizes.spacing.small;
    let mut left = row.x;
    let mut button = |ui: &mut Ui<'_>, key: &str, button: &Button<'_>| {
        let (width, height) = ui.button_size(button);
        let cell = place(
            Rect::new(left, row.y, width, row.height),
            (width, height),
            (Align::Start, Align::Center),
        );
        left += width + gap;
        ui.button(("review lines button", key), cell, button).clicked
    };
    let use_button = Button::primary(words::USE_LINES);
    let use_button = if state.is_running {
        use_button.disabled()
    } else {
        use_button
    };
    let use_clicked = button(ui, "use", &use_button);
    let cancel_clicked = button(ui, "cancel", &Button::secondary(words::CANCEL));
    (use_clicked, cancel_clicked)
}
