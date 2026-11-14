//! The head of the review page (`review.page`): what it is about, the sentence that counts the conflicts, and the
//! buttons that act on them all, or the report of what was written in their place (`review.report`).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use super::{FileCards, Layout, ReviewIntent, ReviewRead, ReviewState};
use crate::intent::Intent;
use crate::text::review as words;

/// The review of the resolutions, over the rectangle its caller clipped it to: the head with its buttons, then the
/// files and the filters beside the cards, or over them in a narrow window.
///
/// Feature: review.page
pub fn page_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.huge, spacing.large));
    let mut intents = Vec::new();
    ui.region("review", (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::with_gap(inner, spacing.medium);
        let head_height = head_height(state, ui, inner.width);
        intents.extend(ui.clip(area.top(head_height), |ui| head(state, ui)));
        let body = area.rest();
        match state.read {
            ReviewRead::Reading if state.files.is_empty() => {
                let _ = ui.label(body, &Label::new(words::READING).tone(Tone::Muted));
            }
            ReviewRead::Failed => {
                let _ = ui.label(body, &Label::new(words::FAILED).tone(Tone::Muted));
            }
            _ => intents.extend(super::body::body(state, ui, body)),
        }
    });
    intents
}

/// The sentence under the head.
fn summary(state: &ReviewState<'_>) -> String {
    if !state.proposers.rules && !state.proposers.ai {
        return String::from(words::NOTHING_SWITCHED_ON);
    }
    let mut counted = words::Summary {
        files: state.files.len(),
        deleted: state.deleted.len(),
        ..words::Summary::default()
    };
    for card in state.conflicts() {
        counted.conflicts += 1;
        match card.proposal.as_ref().map(|proposal| &proposal.source) {
            Some(model::review::ProposalSource::Rules(_)) => counted.by_rules += 1,
            Some(model::review::ProposalSource::Ai { .. }) => counted.by_ai += 1,
            Some(model::review::ProposalSource::Choice(_)) => counted.chosen += 1,
            None => counted.nothing += 1,
        }
    }
    words::summary(&counted)
}

/// How many rows a text takes in a width.
fn rows_of(ui: &Ui<'_>, text: &str, width: f32) -> f32 {
    let measured = ui.measure(text, ui.text_style(TextKind::Body));
    (measured / width.max(1.0)).ceil().max(1.0)
}

/// The height of the head.
fn head_height(state: &ReviewState<'_>, ui: &Ui<'_>, width: f32) -> f32 {
    let sizes = ui.theme().sizes;
    let row = sizes.row_height;
    let mut height = row * 1.5 + row * rows_of(ui, &summary(state), width) + sizes.control_height;
    if state.warned() > 0 {
        height += row;
    }
    if state.report.is_some() {
        height += row * 3.0;
    }
    if state.model.is_some() {
        height += sizes.control_height + sizes.spacing.small;
    }
    height + sizes.spacing.medium
}

/// The head: **Conflicts of the merge**, the sentence that counts, the warning about the proposals with one, and the
/// buttons or the report.
fn head(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let sizes = ui.theme().sizes;
    let row = sizes.row_height;
    let mut area = Area::new(rect);
    let _ = ui.heading(area.top(row * 1.5), words::head(state.operation));
    let said = summary(state);
    let lines = rows_of(ui, &said, rect.width);
    let _ = ui.paragraph(area.top(row * lines), &said, (TextKind::Body, Tone::Muted));
    let warned = state.warned();
    if warned > 0 {
        let _ = ui.label(
            area.top(row),
            &Label::new(&words::with_warnings(warned)).tone(Tone::Warning),
        );
    }
    let mut intents = Vec::new();
    if state.model.is_some() {
        let asking = area.top(sizes.control_height);
        intents.extend(ui.clip(asking, |ui| super::ai::resolve_conflicts_view(state, ui)));
        let _ = area.top(sizes.spacing.small);
    }
    if state.report.is_some() {
        let report = area.top(row * 3.0 + sizes.control_height);
        intents.extend(ui.clip(report, |ui| report_view(state, ui)));
        return intents;
    }
    intents.extend(buttons(state, ui, area.top(sizes.control_height)));
    intents
}

/// A button at the left of an area; whether it was clicked.
fn left_button(ui: &mut Ui<'_>, area: &mut Area, (key, button): (&str, &Button<'_>)) -> bool {
    let (width, height) = ui.button_size(button);
    let cell = place(area.left(width), (width, height), (Align::Start, Align::Center));
    ui.button(key, cell, button).clicked
}

/// The buttons of the head: **Accept all proposed**, **Clear**, **Cards** and **Columns**, **Show the result**, and
/// **Write N resolutions** at the right.
fn buttons(state: &ReviewState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let spacing = ui.theme().sizes.spacing;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, spacing.small);
    let accepted = state.accepted();
    let write_said = words::write(accepted);
    let write = Button::primary(&write_said).with_tooltip(words::WRITE_TOOLTIP);
    let write = if accepted == 0 || state.is_running {
        write.disabled()
    } else {
        write
    };
    let (width, height) = ui.button_size(&write);
    let cell = place(area.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("write", cell, &write).clicked {
        intents.push(Intent::Review(ReviewIntent::Write));
    }
    for (key, view) in [
        (
            "accept all",
            accept_all_view as fn(&ReviewState<'_>, &mut Ui<'_>) -> Vec<Intent>,
        ),
        ("clear", clear_view),
    ] {
        let said = if key == "clear" {
            words::CLEAR
        } else {
            words::ACCEPT_ALL
        };
        let width = ui.button_size(&Button::secondary(said)).0;
        let cell = area.left(width);
        intents.extend(ui.clip(cell, |ui| view(state, ui)));
    }
    let _ = area.left(spacing.medium);
    for (layout, said, tooltip) in [
        (Layout::Cards, words::CARDS, words::CARDS_TOOLTIP),
        (Layout::Columns, words::COLUMNS, words::COLUMNS_TOOLTIP),
    ] {
        let button = Button::secondary(said).with_tooltip(tooltip);
        let button = if state.layout == layout {
            button
        } else {
            button.kind(ButtonKind::Outline)
        };
        if left_button(ui, &mut area, (said, &button)) {
            intents.push(Intent::Review(ReviewIntent::Layout(layout)));
        }
    }
    let _ = area.left(spacing.medium);
    let width = ui.button_size(&Button::secondary(words::SHOW_RESULT)).0;
    let cell = area.left(width);
    intents.extend(ui.clip(cell, |ui| show_result_view(state, ui)));
    intents
}

/// **Accept all proposed**, over the rectangle its caller clipped it to, disabled while no proposal without a warning
/// is left to accept.
///
/// Feature: review.accept-all
pub fn accept_all_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::secondary(words::ACCEPT_ALL).with_tooltip(words::ACCEPT_ALL_TOOLTIP);
    let can_accept = state.conflicts().any(super::ConflictCard::is_acceptable_with_others);
    let button = if can_accept && !state.is_running {
        button
    } else {
        button.disabled()
    };
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::Start, Align::Center));
    if ui.button("accept all", cell, &button).clicked {
        return vec![Intent::Review(ReviewIntent::AcceptAll)];
    }
    Vec::new()
}

/// **Clear**, over the rectangle its caller clipped it to, disabled while nothing is accepted.
///
/// Feature: review.clear
pub fn clear_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::secondary(words::CLEAR).with_tooltip(words::CLEAR_TOOLTIP);
    let button = if state.accepted() > 0 && !state.is_running {
        button
    } else {
        button.disabled()
    };
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::Start, Align::Center));
    if ui.button("clear", cell, &button).clicked {
        return vec![Intent::Review(ReviewIntent::Clear)];
    }
    Vec::new()
}

/// **Show the result**, over the rectangle its caller clipped it to, disabled while nothing is accepted.
///
/// Feature: review.show-result
pub fn show_result_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::secondary(words::SHOW_RESULT).with_tooltip(words::SHOW_RESULT_TOOLTIP);
    let button = if state.accepted() > 0 {
        button
    } else {
        button.disabled()
    };
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::Start, Align::Center));
    if ui.button("show result", cell, &button).clicked {
        return vec![Intent::Review(ReviewIntent::ShowResult)];
    }
    Vec::new()
}

/// What the last write did, over the rectangle its caller clipped it to: what was written, what was staged, what is
/// left, and **Undo** and, once no file is left in conflict during an operation, **Continue the …**.
///
/// Feature: review.report
pub fn report_view(state: &ReviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(report) = state.report else {
        return Vec::new();
    };
    let rect = ui.current_clip();
    let sizes = ui.theme().sizes;
    let row = sizes.row_height;
    let mut area = Area::new(rect);
    let _ = ui.label(
        area.top(row),
        &Label::new(&words::wrote(report.conflicts, report.files, report.from_ai)).kind(TextKind::Strong),
    );
    if report.staged > 0 {
        let _ = ui.label(area.top(row), &Label::new(&words::staged(report.staged)));
    }
    let left = if report.files_left == 0 {
        words::none_left(state.operation)
    } else if report.conflicts_left > 0 {
        words::conflicts_left(report.conflicts_left)
    } else {
        words::files_left(report.files_left)
    };
    let _ = ui.label(area.top(row), &Label::new(&left));
    if report.staged == 0 {
        let _ = area.top(row);
    }
    let mut buttons = Area::with_gap(area.top(sizes.control_height), sizes.spacing.small);
    let mut intents = Vec::new();
    let undo = Button::secondary(words::UNDO).with_tooltip(words::UNDO_TOOLTIP);
    let undo = if state.is_running { undo.disabled() } else { undo };
    if left_button(ui, &mut buttons, ("undo", &undo)) {
        intents.push(Intent::Review(ReviewIntent::Undo));
    }
    if report.files_left == 0
        && let Some(operation) = state.operation
    {
        let said = words::continue_operation(operation);
        let tooltip = words::continue_tooltip(operation);
        let button = Button::primary(&said).with_tooltip(&tooltip);
        let button = if state.can_continue && !state.is_running {
            button
        } else {
            button.disabled()
        };
        if left_button(ui, &mut buttons, ("continue", &button)) {
            intents.push(Intent::Review(ReviewIntent::Continue));
        }
    }
    intents
}

/// What the header of a file's conflicts shows.
#[derive(Clone, Copy, Debug)]
pub struct FileState<'a> {
    /// The file.
    pub file: &'a FileCards,
    /// Whether an operation runs.
    pub is_running: bool,
}

/// **Accept the file**, over the rectangle its caller clipped it to, offered while the file has a proposal not
/// accepted yet without a warning.
///
/// Feature: review.accept-file
pub fn accept_file_view(state: &FileState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !state
        .file
        .conflicts
        .iter()
        .any(super::ConflictCard::is_acceptable_with_others)
    {
        return Vec::new();
    }
    let rect = ui.current_clip();
    let button = Button::secondary(words::ACCEPT_FILE).with_tooltip(words::ACCEPT_FILE_TOOLTIP);
    let button = if state.is_running { button.disabled() } else { button };
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::End, Align::Center));
    if ui.button(("accept file", &state.file.path), cell, &button).clicked {
        return vec![Intent::Review(ReviewIntent::AcceptFile(state.file.path.clone()))];
    }
    Vec::new()
}
