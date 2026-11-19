//! The files git resolved as before, where the files in conflict are listed (`rerere.resolved-as-before`): the words
//! on their rows, and what their menus have in place of the two sides.

use model::rerere::ResolvedAsBefore;
use ui::{Label, TextKind, Tone, Ui};

use super::ConflictsIntent;
use crate::intent::Intent;
use crate::text::rerere as words;

/// What the words of a file git resolved as before are drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedAsBeforeState<'a> {
    /// The file.
    pub file: &'a ResolvedAsBefore,
}

/// The width the words of a file git resolved as before take.
pub(crate) fn words_width(ui: &Ui<'_>) -> f32 {
    ui.measure(words::RESOLVED_AS_BEFORE, ui.text_style(TextKind::Detail))
}

/// **Resolved as before**, in the small text of details, muted, over the rectangle its caller clipped it to, at the
/// right of the row of a file git resolved as before; its menu has **Show what git wrote**, **Mark as resolved** and
/// **Forget this resolution…** in place of the two sides (`changes.row-menu`), and the banner and the message of the
/// stop count these files (`conflicts.banner`, `conflicts.stopped`).
///
/// Feature: rerere.resolved-as-before
pub fn resolved_as_before(state: &ResolvedAsBeforeState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let label = Label::new(words::RESOLVED_AS_BEFORE)
        .kind(TextKind::Detail)
        .tone(Tone::Muted);
    let _ = ui.label(rect, &label);
    let _ = state.file;
    Vec::new()
}

/// What **Show what git wrote** asks for a file git resolved as before.
#[must_use]
pub fn show_what_git_wrote(file: &ResolvedAsBefore) -> Intent {
    Intent::Conflicts(ConflictsIntent::ShowWhatGitWrote {
        path: file.path.clone(),
        is_staged: file.is_staged,
    })
}
