//! **Go to commit** (`search.go-to-commit`): Gitardo's input box for anything git takes for a commit, which says while
//! it is typed when git knows no commit by that name; the same box, as **Open commit**, opens the page of the commit
//! (`commit-page.open`).

use ui::layout::Area;
use ui::{Dialog, DialogButton, DialogRole, InputBox, Label, Message, Severity, TextKind, Tone, Ui};

use super::SearchIntent;
use crate::intent::Intent;
use crate::text::search as words;

/// What **Go to commit** shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoToCommitDialog<'a> {
    /// What is typed.
    pub text: &'a str,
    /// What git says of it.
    pub verdict: GoToCommitVerdict,
    /// What the commit is asked for.
    pub purpose: GoToCommitPurpose,
}

/// What the commit of **Go to commit** is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GoToCommitPurpose {
    /// To go to it in the timeline (`search.go-to-commit`).
    #[default]
    GoTo,
    /// To open it as a page, as **Open commit** (`commit-page.open`).
    OpenPage,
}

/// What git says of the name typed in **Go to commit**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GoToCommitVerdict {
    /// Nothing yet: the box is empty, or git is asked.
    #[default]
    Unknown,
    /// A commit.
    Commit,
    /// No commit: [`words::UNKNOWN_COMMIT`] is said under the box.
    NoCommit,
}

/// The buttons of **Go to commit**.
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

/// The buttons of **Open commit**.
const OPEN_BUTTONS: [DialogButton<'static>; 2] = [
    DialogButton {
        label: words::CANCEL,
        role: DialogRole::Cancel,
    },
    DialogButton {
        label: words::OPEN_PAGE,
        role: DialogRole::Primary,
    },
];

/// **Go to commit**, or **Open commit**, over the window: what it asks for, the box, and what git says of what is
/// typed.
pub fn go_to_commit_dialog(state: &GoToCommitDialog<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.medium;
    let row = theme.sizes.row_height;
    let (title, buttons) = match state.purpose {
        GoToCommitPurpose::GoTo => (words::GO_TO_COMMIT_TITLE, &BUTTONS),
        GoToCommitPurpose::OpenPage => (words::OPEN_COMMIT_TITLE, &OPEN_BUTTONS),
    };
    let dialog = Dialog {
        title,
        body_height: row + theme.sizes.control_height + gap,
        buttons,
    };
    let mut intents = Vec::new();
    let response = ui.dialog("go to commit", &dialog, |ui, body| {
        let mut area = Area::with_gap(body.rect, gap);
        let detail = Label::new(words::GO_TO_COMMIT_DETAIL)
            .kind(TextKind::Detail)
            .tone(Tone::Muted);
        let _ = ui.label(area.top(row), &detail);
        let mut input = InputBox::new(state.text, title);
        if state.verdict == GoToCommitVerdict::NoCommit {
            input = input.with_message(Message {
                text: words::UNKNOWN_COMMIT,
                severity: Severity::Error,
            });
        }
        let field = area.top(theme.sizes.control_height);
        if body.is_opening {
            let id = ui.id("revision");
            ui.request_focus(id);
        }
        if let Some(text) = ui.input_box("revision", field, &input).changed_text {
            intents.push(Intent::Search(SearchIntent::GoToCommitTyped(text)));
        }
    });
    if response.is_cancelled {
        intents.push(Intent::Search(SearchIntent::GoToCommitClose));
    } else if response.pressed.is_some() {
        intents.push(Intent::Search(SearchIntent::GoToCommitSubmit));
    }
    intents
}
