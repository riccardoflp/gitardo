//! The message of a commit in the inspector: its title in bold, and under it the rest, cut at four lines with **Show
//! more** (`details.show-more`).

use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Rect, TextKind, Tone, Ui};

use crate::details::DetailsIntent;
use crate::intent::Intent;
use crate::text::details as words;

/// How many lines of the text under the title the inspector shows until **Show more** is clicked.
pub const SHOWN_LINES: usize = 4;

/// The text under the title of a message, and whether it is shown whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageState<'a> {
    /// The text under the title, unwrapped (`details.message-unwrap`).
    pub text: &'a str,
    /// Whether **Show more** was clicked.
    pub is_expanded: bool,
}

/// How many lines a message's text has at a width, and how many of them are shown.
fn line_counts(ui: &Ui<'_>, state: &MessageState<'_>, width: f32) -> (usize, usize) {
    if state.text.is_empty() {
        return (0, 0);
    }
    let lines = ui.fonts().wrap(state.text, ui.text_style(TextKind::Body), width).len();
    let shown = if state.is_expanded {
        lines
    } else {
        lines.min(SHOWN_LINES)
    };
    (lines, shown)
}

/// Whether the text is longer than four lines at a width, so that **Show more** or **Show less** is offered.
fn has_link(ui: &Ui<'_>, state: &MessageState<'_>, width: f32) -> bool {
    line_counts(ui, state, width).0 > SHOWN_LINES
}

/// The height the text and its link take at a width.
pub(crate) fn text_height(ui: &Ui<'_>, state: &MessageState<'_>, width: f32) -> f32 {
    let (_, shown) = line_counts(ui, state, width);
    let lines = crate::number::float_of_count(shown) * ui.line_height(ui.text_style(TextKind::Body));
    let link = if has_link(ui, state, width) {
        ui.theme().sizes.control_height
    } else {
        0.0
    };
    lines + link
}

/// The text under the title, over the rectangle its caller clipped it to: four lines at most, then **Show more** when
/// more is hidden; once clicked, the whole text in place, without a scroll bar of its own, and **Show less**. Whether
/// the link is there follows the width of the inspector.
///
/// Feature: details.show-more
pub fn show_more(state: &MessageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let (_, shown) = line_counts(ui, state, rect.width);
    let line = ui.line_height(ui.text_style(TextKind::Body));
    let mut area = Area::new(rect);
    let lines = area.top(crate::number::float_of_count(shown) * line);
    let _ = ui.paragraph(lines, state.text, (TextKind::Body, Tone::Muted));
    if !has_link(ui, state, rect.width) {
        return Vec::new();
    }
    let said = if state.is_expanded {
        words::SHOW_LESS
    } else {
        words::SHOW_MORE
    };
    let button = Button::secondary(said).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&button);
    let cell: Rect = place(
        super::outdent(ui, area.rest()),
        (width, height),
        (Align::Start, Align::Center),
    );
    if ui.button("show more", cell, &button).clicked {
        return vec![Intent::Details(DetailsIntent::ShowMore(!state.is_expanded))];
    }
    Vec::new()
}
