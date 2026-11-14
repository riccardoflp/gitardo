//! The question asked before closing the page of a file with edits not saved (`file-view.edit`): **Save changes to
//! app.rs?** with **Save**, **Don't save** and **Cancel**.

use ui::{Dialog, DialogButton, DialogRole, Ui};

use super::{CloseAnswer, FileViewIntent};
use crate::intent::Intent;
use crate::text::file_view as words;

/// The question, while it waits for its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseQuestion<'a> {
    /// The name of the file.
    pub name: &'a str,
}

/// The buttons of the question, in the order they are drawn.
const BUTTONS: [DialogButton<'static>; 3] = [
    DialogButton {
        label: words::CANCEL,
        role: DialogRole::Cancel,
    },
    DialogButton {
        label: words::DONT_SAVE,
        role: DialogRole::Secondary,
    },
    DialogButton {
        label: words::SAVE,
        role: DialogRole::Primary,
    },
];

/// **Save changes to app.rs?** over the window; Escape is **Cancel**.
pub fn close_question_view(state: &CloseQuestion<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let title = words::save_changes(state.name);
    let dialog = Dialog {
        title: &title,
        body_height: 0.0,
        buttons: &BUTTONS,
    };
    let response = ui.dialog(("save changes question", state.name), &dialog, |_, _| {});
    let answer = match response.pressed {
        _ if response.is_cancelled => Some(CloseAnswer::Cancel),
        Some(2) => Some(CloseAnswer::Save),
        Some(1) => Some(CloseAnswer::DontSave),
        Some(_) => Some(CloseAnswer::Cancel),
        None => None,
    };
    answer.map_or_default(|answer| vec![Intent::FileView(FileViewIntent::AnswerClose(answer))])
}
