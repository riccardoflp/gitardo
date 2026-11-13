//! **Go to line** (`file-view.go-to-line`): a number between 1 and the number of lines, the current line at first;
//! what is not a number is refused while it is typed, and a line past the end is the last line.

use ui::{Dialog, DialogButton, DialogRole, InputBox, Message, Severity, Ui};

use super::{FilePageIntent, FilePageState, page_intent};
use crate::intent::Intent;
use crate::text::file_view as words;

/// **Go to line**, while it is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoToLineState<'a> {
    /// What is typed: the current line at first.
    pub typed: &'a str,
    /// How many lines the file has.
    pub line_count: usize,
}

/// The line a text names, from 1, when it is a whole number from 1: a line past the end is the last line.
#[must_use]
pub fn parse_line(typed: &str, line_count: usize) -> Option<usize> {
    let line: usize = typed.trim().parse().ok()?;
    (line >= 1).then(|| line.min(line_count.max(1)))
}

/// The buttons of the dialog.
const BUTTONS: [DialogButton<'static>; 2] = [
    DialogButton {
        label: words::CANCEL,
        role: DialogRole::Cancel,
    },
    DialogButton {
        label: words::GO,
        role: DialogRole::Primary,
    },
];

/// **Go to line** over the window: the box, and what it says of what is typed that is not a line of the file; **Go**
/// goes there only when it is one.
///
/// Feature: file-view.go-to-line
pub fn go_to_line_view(state: &FilePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(asked) = state.go_to_line else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let line = parse_line(asked.typed, asked.line_count);
    let dialog = Dialog {
        title: words::GO_TO_LINE,
        body_height: theme.sizes.control_height + theme.sizes.row_height,
        buttons: &BUTTONS,
    };
    let refusal = words::line_between(asked.line_count);
    let mut intents = Vec::new();
    let response = ui.dialog(("go to line", state.id), &dialog, |ui, body| {
        let mut input = InputBox::new(asked.typed, words::GO_TO_LINE);
        if line.is_none() {
            input = input.with_message(Message {
                text: &refusal,
                severity: Severity::Error,
            });
        }
        let field = ui::Rect::new(body.rect.x, body.rect.y, body.rect.width, theme.sizes.control_height);
        if body.is_opening {
            let id = ui.id("line");
            ui.focus_input_box(id);
        }
        if let Some(text) = ui.input_box("line", field, &input).changed_text {
            intents.push(page_intent(state, FilePageIntent::GoToLineTyped(text)));
        }
    });
    if response.is_cancelled {
        intents.push(page_intent(state, FilePageIntent::CloseGoToLine));
    } else if response.pressed.is_some()
        && let Some(line) = line
    {
        intents.push(page_intent(state, FilePageIntent::GoToLine(line)));
    }
    intents
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_line;

    #[test]
    fn a_line_is_a_number_from_1_to_the_number_of_lines() {
        assert_eq!(parse_line("120", 300), Some(120));
        assert_eq!(parse_line(" 7 ", 300), Some(7));
        assert_eq!(parse_line("0", 300), None);
        assert_eq!(parse_line("301", 300), Some(300));
        assert_eq!(parse_line("abc", 300), None);
    }
}
