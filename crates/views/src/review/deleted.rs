//! A file one side deleted and the other changed (`review.deleted-card`): which side did what, what the side that kept
//! it changed of it, and whether it stays or goes.

use model::conflicts::SideNames;
use model::review::{DeletedFile, KeptChanges, KeptRow, Which};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, Label, Rect, TextKind, Tone, Ui};

use super::lines::{Look, code_line, gutter_width};
use crate::intent::Intent;
use crate::text::review as words;

/// What a card of a file deleted on one side shows.
#[derive(Clone, Copy, Debug)]
pub struct DeletedState<'a> {
    /// The file.
    pub file: &'a DeletedFile,
    /// What the two sides are.
    pub sides: &'a SideNames,
}

/// How many rows the changes take: one per row of the diff, or one for what is said in its place.
fn rows(file: &DeletedFile) -> usize {
    match &file.changes {
        KeptChanges::Rows(rows) => rows.len(),
        KeptChanges::Nothing | KeptChanges::TooMany => 1,
    }
}

/// The height of a card of a file deleted on one side.
pub(super) fn deleted_height(file: &DeletedFile, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.row_height * crate::number::float_of_count(2 + rows(file)) + sizes.control_height + 2.0 * sizes.spacing.medium
}

/// A file one side deleted and the other changed, over the rectangle its caller clipped it to: its path, which side
/// deleted it and which changed it, the changes of the side that kept it with the lines between them left out, and
/// **Keep it, with these changes** and **Delete it**, which stay disabled until `conflicts.take-side` is built.
///
/// Feature: review.deleted-card
pub fn deleted_card_view(state: &DeletedState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
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
    let (deleter, keeper) = match file.deleted_by {
        Which::Current => (&state.sides.current, &state.sides.incoming),
        Which::Incoming => (&state.sides.incoming, &state.sides.current),
    };
    let _ = ui.label(area.top(row), &Label::new(&file.path).kind(TextKind::Strong));
    let _ = ui.label(area.top(row), &Label::new(&words::deleted_by(deleter, keeper)));
    match &file.changes {
        KeptChanges::Nothing => {
            let _ = ui.label(area.top(row), &Label::new(words::CHANGED_NOTHING).tone(Tone::Muted));
        }
        KeptChanges::TooMany => {
            let _ = ui.label(area.top(row), &Label::new(words::TOO_MANY_CHANGES).tone(Tone::Muted));
        }
        KeptChanges::Rows(rows) => {
            let largest = rows
                .iter()
                .filter_map(|row| match row {
                    KeptRow::Same(number, _) | KeptRow::Added(number, _) => Some(*number),
                    KeptRow::Removed(_) | KeptRow::LeftOut => None,
                })
                .max()
                .unwrap_or(1);
            let gutter = gutter_width(ui, largest);
            for kept in rows {
                kept_row(ui, (area.top(row), gutter), kept);
            }
        }
    }
    let mut buttons = Area::with_gap(area.top(theme.sizes.control_height), theme.sizes.spacing.small);
    let keeps_current = file.deleted_by == Which::Incoming;
    for (key, said, tooltip) in [
        ("keep", words::KEEP_IT, words::keep_tooltip(&file.path, keeps_current)),
        (
            "delete",
            words::DELETE_IT,
            words::delete_tooltip(&file.path, !keeps_current),
        ),
    ] {
        let button = Button::secondary(said).with_tooltip(&tooltip).disabled();
        let (width, height) = ui.button_size(&button);
        let cell = place(buttons.left(width), (width, height), (Align::Start, Align::Center));
        let _ = ui.button(key, cell, &button);
    }
    Vec::new()
}

/// A row of the changes of the side that kept the file.
fn kept_row(ui: &mut Ui<'_>, (row, gutter): (Rect, f32), kept: &KeptRow) {
    let colors = ui.theme().colors;
    let plain = Look {
        gutter: "",
        color: colors.foreground,
        bars: [None, None],
        tint: None,
    };
    match kept {
        KeptRow::Same(number, text) => {
            let number = number.to_string();
            code_line(
                ui,
                row,
                (text, gutter),
                &Look {
                    gutter: &number,
                    color: colors.muted_foreground,
                    ..plain
                },
            );
        }
        KeptRow::Removed(text) => code_line(
            ui,
            row,
            (text, gutter),
            &Look {
                gutter: "-",
                tint: Some(colors.removed_background),
                ..plain
            },
        ),
        KeptRow::Added(number, text) => {
            let number = number.to_string();
            code_line(
                ui,
                row,
                (text, gutter),
                &Look {
                    gutter: &number,
                    tint: Some(colors.added_background),
                    ..plain
                },
            );
        }
        KeptRow::LeftOut => {
            let _ = ui.label(
                row,
                &Label::new(words::LINES_LEFT_OUT)
                    .kind(TextKind::Small)
                    .tone(Tone::Muted),
            );
        }
    }
}
