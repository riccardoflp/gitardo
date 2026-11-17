//! The title row of a page: the button of the side column, the trail, the name of the page and the button that closes
//! it (`repo.side-column`).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use super::PageState;
use crate::graph::{ReferencesShown, side_button};
use crate::icons;
use crate::intent::Intent;
use crate::text::pages as words;

/// The title row above a page: the button of the side column when there is one, at the same place as in the top row
/// of the timeline (`repo.side-column`), the trail with the step of the page it was opened from, a dot after the name
/// of a file with edits not saved, and the button that closes it.
pub(super) fn trail_view(
    page: &PageState<'_>,
    side: Option<ReferencesShown>,
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let (title, is_unsaved) = (page.title(), page.is_unsaved());
    let theme = *ui.theme();
    let mut parts = row_start(side, ui, rect, &mut intents);
    ui.region("trail", (rect, AccessRole::Navigation, words::TRAIL), |ui| {
        let timeline = Button::secondary(words::TIMELINE).kind(ButtonKind::Icon);
        let (width, height) = ui.button_size(&timeline);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button("trail timeline", cell, &timeline).clicked {
            intents.push(Intent::ShowTimeline);
        }
        separator(ui, &mut parts);
        if let Some((name, intent)) = page.step() {
            let step = Button::secondary(name).kind(ButtonKind::Icon);
            let (width, height) = ui.button_size(&step);
            let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
            if ui.button("trail step", cell, &step).clicked {
                intents.push(intent);
            }
            separator(ui, &mut parts);
        }
        let side = theme.sizes.control_height;
        let close_cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        let close = Button::icon(icons::CLOSE, words::CLOSE_PAGE).kind(ButtonKind::Icon);
        if ui.button("close page", close_cell, &close).clicked {
            intents.push(Intent::ClosePage);
        }
        if is_unsaved {
            let dot = crate::text::file_view::UNSAVED_DOT;
            let title_width = ui.measure(title, ui.text_style(TextKind::Strong));
            let _ = ui.label(parts.left(title_width), &Label::new(title).kind(TextKind::Strong));
            let dot_width = ui.measure(dot, ui.text_style(TextKind::Body));
            let _ = ui.label(parts.left(dot_width), &Label::new(dot));
        } else {
            let _ = ui.label(parts.rest(), &Label::new(title).kind(TextKind::Strong));
        }
    });
    intents
}

/// The line under the title row, and its parts after the button of the side column when there is one, which is at the
/// left margin of the top row of the timeline; without it, the trail starts further in, as it always did.
fn row_start(side: Option<ReferencesShown>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) -> Area {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let margin = spacing.large + spacing.small;
    let left = if side.is_some() { spacing.medium } else { margin };
    let inner = inset(
        rect,
        Insets {
            left,
            right: margin,
            ..Insets::all(0.0)
        },
    );
    let mut parts = Area::with_gap(inner, spacing.medium);
    if let Some(side) = side {
        intents.extend(side_button(side, &mut parts, ui));
    }
    parts
}

/// The `›` between two steps of the trail.
fn separator(ui: &mut Ui<'_>, parts: &mut Area) {
    let width = ui.measure(words::TRAIL_SEPARATOR, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(width), &Label::new(words::TRAIL_SEPARATOR).tone(Tone::Muted));
}
