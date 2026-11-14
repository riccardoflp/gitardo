//! The question asked before a very large file in conflict is shown (`merge-view.page`).

use ui::{Dialog, DialogButton, DialogRole, Ui};

use super::MergeIntent;
use crate::intent::Intent;
use crate::text::diff as diff_words;
use crate::text::merge_view as words;

/// The question, while it waits for its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LargeMergeQuestion<'a> {
    /// The title of the page it would open.
    pub title: &'a str,
}

/// **This file is very large and the merge may be slow to show. Continue?** over the window, with **Continue**;
/// anything else opens nothing.
pub fn large_merge_question_view(state: &LargeMergeQuestion<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let buttons = [
        DialogButton {
            label: diff_words::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: diff_words::CONTINUE,
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: words::LARGE_QUESTION,
        body_height: 0.0,
        buttons: &buttons,
    };
    let response = ui.dialog(("large merge question", state.title), &dialog, |_, _| {});
    match response.pressed {
        _ if response.is_cancelled => vec![Intent::MergeView(MergeIntent::AnswerLarge(false))],
        Some(1) => vec![Intent::MergeView(MergeIntent::AnswerLarge(true))],
        Some(_) => vec![Intent::MergeView(MergeIntent::AnswerLarge(false))],
        None => Vec::new(),
    }
}
