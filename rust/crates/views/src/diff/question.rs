//! The question asked before the diff of a very large file is computed (`diff.page`).

use ui::{Dialog, DialogButton, DialogRole, Ui};

use super::DiffIntent;
use crate::intent::Intent;
use crate::text::diff as words;

/// The question, while it waits for its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LargeDiffQuestion<'a> {
    /// The title of the page it would open.
    pub title: &'a str,
}

/// **This file is very large and the diff may be slow. Continue?** over the window, with **Continue**; anything else
/// opens nothing.
pub fn large_diff_question_view(state: &LargeDiffQuestion<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let buttons = [
        DialogButton {
            label: words::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: words::CONTINUE,
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: words::LARGE_QUESTION,
        body_height: 0.0,
        buttons: &buttons,
    };
    let response = ui.dialog(("large diff question", state.title), &dialog, |_, _| {});
    match response.pressed {
        _ if response.is_cancelled => vec![Intent::Diff(DiffIntent::AnswerLarge(false))],
        Some(1) => vec![Intent::Diff(DiffIntent::AnswerLarge(true))],
        Some(_) => vec![Intent::Diff(DiffIntent::AnswerLarge(false))],
        None => Vec::new(),
    }
}
