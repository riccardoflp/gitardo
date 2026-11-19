//! The note of a commit under its message, in the inspector and in the page of the commit (`note.show`): a block
//! titled **Note**, with the pencil of **Edit note…** and **…** with **Remove note** on its title, and its text as it
//! was written, its lines kept.

use model::CommitId;
use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Label, MenuEntry, Rect, TextKind, Tone, Ui};

use crate::details::DetailsIntent;
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::details as words;

/// How many lines of a note the inspector shows until **Show more** is clicked, as it cuts a long message.
pub const NOTE_SHOWN_LINES: usize = 12;

/// How much of a note is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLength {
    /// All of it, as the page of a commit shows it.
    Whole,
    /// Cut at [`NOTE_SHOWN_LINES`] with **Show more**, as the inspector shows it; whole once that was clicked.
    Cut {
        /// Whether **Show more** was clicked.
        is_expanded: bool,
    },
}

/// The note of a commit, as the block shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteState<'a> {
    /// The commit the note is of.
    pub commit: &'a CommitId,
    /// Its text as it was written.
    pub text: &'a str,
    /// How much of it is shown.
    pub length: NoteLength,
}

/// How many lines the text has at a width, and how many of them are shown.
fn line_counts(ui: &Ui<'_>, state: &NoteState<'_>, width: f32) -> (usize, usize) {
    let lines = ui.fonts().wrap(state.text, ui.text_style(TextKind::Body), width).len();
    let shown = match state.length {
        NoteLength::Cut { is_expanded: false } => lines.min(NOTE_SHOWN_LINES),
        NoteLength::Whole | NoteLength::Cut { is_expanded: true } => lines,
    };
    (lines, shown)
}

/// Whether **Show more** or **Show less** is offered under the text.
fn has_link(ui: &Ui<'_>, state: &NoteState<'_>, width: f32) -> bool {
    matches!(state.length, NoteLength::Cut { .. }) && line_counts(ui, state, width).0 > NOTE_SHOWN_LINES
}

/// The height of the block at a width: its title, its text and the link that shows more of it.
#[must_use]
pub fn note_height(ui: &Ui<'_>, state: &NoteState<'_>, width: f32) -> f32 {
    let sizes = ui.theme().sizes;
    let (_, shown) = line_counts(ui, state, width);
    let text = crate::number::float_of_count(shown) * ui.line_height(ui.text_style(TextKind::Body));
    let link = if has_link(ui, state, width) {
        sizes.control_height
    } else {
        0.0
    };
    sizes.control_height + sizes.spacing.small + text + link
}

/// The note of a commit, over the rectangle its caller clipped it to: **Note** with the pencil of **Edit note…** and
/// **…** with **Remove note** at its right, then its text in the font of a message, its lines as they were written
/// and not joined (Q1212), cut with **Show more** in the inspector.
///
/// Feature: note.show
pub fn note_view(state: &NoteState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let sizes = ui.theme().sizes;
    let mut intents = Vec::new();
    let mut area = Area::new(rect);
    title_line(ui, area.top(sizes.control_height), state, &mut intents);
    let _gap = area.top(sizes.spacing.small);
    let (_, shown) = line_counts(ui, state, rect.width);
    let line = ui.line_height(ui.text_style(TextKind::Body));
    let text = area.top(crate::number::float_of_count(shown) * line);
    let _ = ui.paragraph(text, state.text, (TextKind::Body, Tone::Normal));
    if let NoteLength::Cut { is_expanded } = state.length
        && has_link(ui, state, rect.width)
    {
        let said = if is_expanded {
            words::SHOW_LESS
        } else {
            words::SHOW_MORE
        };
        let button = Button::secondary(said).kind(ButtonKind::Icon);
        let size = ui.button_size(&button);
        let cell = place(super::outdent(ui, area.rest()), size, (Align::Start, Align::Center));
        if ui.button("show more of the note", cell, &button).clicked {
            intents.push(Intent::Details(DetailsIntent::ShowMoreNote(!is_expanded)));
        }
    }
    intents
}

/// **Note**, with the pencil and the **…** of the note at its right.
fn title_line(ui: &mut Ui<'_>, rect: Rect, state: &NoteState<'_>, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let more_cell = place(area.right(side), (side, side), (Align::Start, Align::Center));
    let edit_cell = place(area.right(side), (side, side), (Align::Start, Align::Center));
    let _ = ui.label(area.rest(), &Label::new(words::NOTE).kind(TextKind::Strong));
    let run = |feature: &'static str| {
        Intent::Feature(FeatureIntent::Run(
            FeatureId(feature),
            Target::Commit(state.commit.clone()),
        ))
    };
    let edit = Button::icon(icons::EDITED, words::EDIT_NOTE).kind(ButtonKind::Icon);
    if ui.button("edit note", edit_cell, &edit).clicked {
        intents.push(run("note.edit"));
    }
    let more = Button::icon(icons::MORE, words::MORE_FOR_NOTE).kind(ButtonKind::Icon);
    let remove = icons::action(icons::DELETE, words::REMOVE_NOTE, ());
    if ui
        .menu_button("note menu", more_cell, &more, &[MenuEntry::Item(remove)])
        .is_some()
    {
        intents.push(run("note.remove"));
    }
}
