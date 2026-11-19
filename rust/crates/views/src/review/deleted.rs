//! A file one side deleted and the other changed (`review.deleted-card`): which side did what, what the side that kept
//! it changed of it, and whether it stays or goes.

use model::conflicts::SideNames;
use model::review::{DeletedFile, KeptChanges, KeptRow, Which};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use super::ReviewIntent;
use super::lines::{Look, code_line, gutter_width};
use crate::conflicts::ConflictsIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::review as words;

/// What a card of a file deleted on one side shows.
#[derive(Clone, Copy, Debug)]
pub struct DeletedState<'a> {
    /// The file.
    pub file: &'a DeletedFile,
    /// What the two sides are.
    pub sides: &'a SideNames,
    /// Whether an operation runs, which disables the buttons that change the file.
    pub is_running: bool,
}

/// How many rows the changes take: one per row of the diff, or one for what is said in its place.
pub(super) fn rows(changes: &KeptChanges) -> usize {
    match changes {
        KeptChanges::Rows(rows) => rows.len(),
        KeptChanges::Nothing | KeptChanges::TooMany => 1,
    }
}

/// The largest number of a line of the changes, which sets the width of their gutter.
pub(super) fn largest_number(rows: &[KeptRow]) -> usize {
    rows.iter()
        .filter_map(|row| match row {
            KeptRow::Same(number, _) | KeptRow::Added(number, _) => Some(*number),
            KeptRow::Removed(_) | KeptRow::LeftOut => None,
        })
        .max()
        .unwrap_or(1)
}

/// The height of a card of a file deleted on one side.
pub(super) fn deleted_height(file: &DeletedFile, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.row_height * crate::number::float_of_count(2 + rows(&file.changes))
        + sizes.control_height
        + 2.0 * sizes.spacing.medium
}

/// A file one side deleted and the other changed, over the rectangle its caller clipped it to: its path, which side
/// deleted it and which changed it, the changes of the side that kept it with the lines between them left out, and
/// **Keep it, with these changes** and **Delete it**, which take that side of it (`conflicts.take-side`), and the button
/// that shows its base beside it (`conflicts.open-file`).
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
            let gutter = gutter_width(ui, largest_number(rows));
            for kept in rows {
                kept_row(ui, (area.top(row), gutter), kept);
            }
        }
    }
    let buttons = area.top(theme.sizes.control_height);
    decide(state, ui, buttons)
}

/// **Keep it, with these changes** and **Delete it**, which take that side of the file, and the eye that shows its base
/// beside it, from the left of a row.
fn decide(state: &DeletedState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let file = state.file;
    let mut buttons = Area::with_gap(row, ui.theme().sizes.spacing.small);
    let keeps_current = file.deleted_by == Which::Incoming;
    let keeper = if keeps_current { Which::Current } else { Which::Incoming };
    let mut intents = Vec::new();
    for (key, said, tooltip, taken) in [
        (
            "keep",
            words::KEEP_IT,
            words::keep_tooltip(&file.path, keeps_current),
            keeper,
        ),
        (
            "delete",
            words::DELETE_IT,
            words::delete_tooltip(&file.path, !keeps_current),
            file.deleted_by,
        ),
    ] {
        let button = Button::secondary(said).with_tooltip(&tooltip);
        let button = if state.is_running { button.disabled() } else { button };
        let (width, height) = ui.button_size(&button);
        let cell = place(buttons.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(key, cell, &button).clicked {
            intents.push(Intent::Review(ReviewIntent::TakeSide(file.path.clone(), taken)));
        }
    }
    let shown = Button::icon(icons::SHOW, words::SHOW_WHAT_IT_WAS).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&shown);
    let cell = place(buttons.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button("what it was", cell, &shown).clicked {
        intents.push(Intent::Conflicts(ConflictsIntent::Open(file.path.clone())));
    }
    intents
}

/// A row of the changes of the side that kept the file.
pub(super) fn kept_row(ui: &mut Ui<'_>, (row, gutter): (Rect, f32), kept: &KeptRow) {
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
