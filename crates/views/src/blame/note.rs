//! The blame of the line under the cursor of a File page (`blame.current-line`): who last changed it, how long ago and
//! the subject of that commit, dimmed after the end of the line, with the hover of the commit.

use ui::{Label, Rect, Sense, TextKind, Tone, Ui, code_columns};

use super::{BlameIntent, FileBlameView, card};
use crate::intent::Intent;

/// How many columns are left between the end of the line and what is said of it.
const GAP_COLUMNS: f32 = 3.0;

/// What the line under the cursor says of who last changed it, after its end, over the text of the editor, and its
/// hover; nothing while the line of the cursor is not in sight.
///
/// Feature: blame.current-line
pub fn current_line_view(state: &FileBlameView<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(note) = state.blame.note else {
        return Vec::new();
    };
    if note.line != state.caret_line || !state.visible.contains(&note.line) {
        return Vec::new();
    }
    let places = state.places;
    // After the end of the line: on its last row, when it wraps.
    let (line_top, line_height) = state.line_place(note.line);
    let top = line_top + line_height - places.line_height;
    let pieces = state.rows.pieces(state.caret_text);
    let last_piece = pieces.last().and_then(|piece| state.caret_text.get(piece.clone()));
    let columns = code_columns(last_piece.unwrap_or(state.caret_text), state.tab_width);
    let left = places.text.x + (columns + GAP_COLUMNS) * places.advance;
    if left >= places.text.right() {
        return Vec::new();
    }
    let style = ui.text_style(TextKind::Code);
    let width = ui.measure(note.text, style).min(places.text.right() - left);
    let rect = Rect::new(left, top, width, places.line_height);
    ui.clip(places.text, |ui| {
        let _ = ui.label(rect, &Label::new(note.text).kind(TextKind::Code).tone(Tone::Muted));
    });
    let id = ui.id(("blame note", state.page));
    let response = ui.interact(id, rect, Sense::HOVER);
    let rests = ui
        .hovered_since(&response)
        .is_some_and(|since| ui.now().saturating_sub(since) >= card::REST);
    let mut intents = Vec::new();
    if response.hovered && !rests {
        ui.request_frame_at(ui.now() + card::REST);
    }
    let is_committed = matches!(note.blame, model::blame::BlameLine::Committed(_));
    if rests && is_committed && state.blame.hovered != Some(note.line) {
        intents.push(Intent::Blame(BlameIntent::Hover(state.page, Some(note.line))));
    }
    if state.blame.hovered == Some(note.line) && state.blame.file.is_none() {
        intents.extend(card::hover_card(ui, (state.page, note.line, false), note.blame, rect));
    }
    intents
}
