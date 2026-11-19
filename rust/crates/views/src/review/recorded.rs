//! A file git resolved from a resolution it recorded (`review.recorded`): what it wrote in place of the conflicts,
//! to accept, which stages it with the resolutions written, or to forget, which marks its conflicts again.

use model::review::{KeptChanges, RecordedFile};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, Label, Rect, TextKind, Tone, Ui};

use super::ReviewIntent;
use super::deleted::{kept_row, largest_number, rows};
use super::lines::gutter_width;
use crate::icons;
use crate::intent::Intent;
use crate::text::{rerere as words, review};

/// What a card of a file git resolved as before shows.
#[derive(Clone, Copy, Debug)]
pub struct RecordedState<'a> {
    /// The file.
    pub file: &'a RecordedFile,
    /// Whether it is accepted.
    pub is_accepted: bool,
    /// Whether an operation runs, which disables the buttons.
    pub is_running: bool,
}

/// The height of a card of a file git resolved as before: its path, its heading, what it says, the changes and the
/// buttons.
pub(super) fn recorded_height(file: &RecordedFile, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.row_height * crate::number::float_of_count(3 + rows(&file.changes))
        + sizes.control_height
        + 2.0 * sizes.spacing.medium
}

/// A file git resolved as before, over the rectangle its caller clipped it to: its path, **Resolved by git as
/// before** with what that means, the lines git wrote as a diff of the file as git marked it against the file on disk,
/// and **Accept**, which **Write** stages, and **Forget it…**, which runs `rerere.forget`.
///
/// Feature: review.recorded
pub fn recorded_card_view(state: &RecordedState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let row = theme.sizes.row_height;
    ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.card_background);
    let inner = inset(
        rect,
        Insets::symmetric(theme.sizes.spacing.medium, theme.sizes.spacing.medium),
    );
    let mut area = Area::new(inner);
    let file = state.file;
    let _ = ui.label(area.top(row), &Label::new(&file.path).kind(TextKind::Strong));
    let _ = ui.label(area.top(row), &Label::new(words::RESOLVED_BY_GIT));
    let _ = ui.label(area.top(row), &Label::new(words::RECORDED_NOTE).tone(Tone::Muted));
    match &file.changes {
        KeptChanges::Nothing => {
            let _ = ui.label(area.top(row), &Label::new(words::WROTE_NOTHING).tone(Tone::Muted));
        }
        KeptChanges::TooMany => {
            let _ = ui.label(area.top(row), &Label::new(review::TOO_MANY_CHANGES).tone(Tone::Muted));
        }
        KeptChanges::Rows(rows) => {
            let gutter = gutter_width(ui, largest_number(rows));
            for kept in rows {
                kept_row(ui, (area.top(row), gutter), kept);
            }
        }
    }
    let buttons = area.top(theme.sizes.control_height);
    decide(state, ui, buttons)
}

/// **Forget it…** from the left of a row, and **Accept**, or **Accepted** to take it back, at its right.
fn decide(state: &RecordedState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let path = &state.file.path;
    let mut buttons = Area::with_gap(row, ui.theme().sizes.spacing.small);
    let mut intents = Vec::new();
    let accept = if state.is_accepted {
        Button::secondary(review::ACCEPTED).with_icon(icons::CHOSEN)
    } else {
        Button::primary(review::ACCEPT).with_tooltip(words::ACCEPT_TOOLTIP)
    };
    let forget = Button::secondary(words::FORGET_IT).with_tooltip(words::FORGET_IT_TOOLTIP);
    let (accept, forget) = if state.is_running {
        (accept.disabled(), forget.disabled())
    } else {
        (accept, forget)
    };
    let (width, height) = ui.button_size(&forget);
    let cell = place(buttons.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button("forget", cell, &forget).clicked {
        intents.push(Intent::Review(ReviewIntent::ForgetRecorded(path.clone())));
    }
    let (width, height) = ui.button_size(&accept);
    let cell = place(buttons.right(width), (width, height), (Align::End, Align::Center));
    if ui.button("accept", cell, &accept).clicked {
        intents.push(Intent::Review(ReviewIntent::AcceptRecorded(
            path.clone(),
            !state.is_accepted,
        )));
    }
    intents
}
