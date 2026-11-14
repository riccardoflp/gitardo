//! The box **Edit** opens on a card (`review.choose`): the lines written in place of a conflict, from what is proposed
//! or from the two sides one after the other; once used, they are a choice like the others, accepted at once, kept by
//! file and fingerprint of the conflict while the window is open.

use ui::editor::{Command, Cursor, Document, apply};
use views::review::{ConflictKey, WritingBox};

use super::Fingerprinted;
use crate::store::state::State;

/// The lines being written on a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinesBox {
    /// The conflict.
    pub key: ConflictKey,
    /// The text.
    pub document: Document,
    /// The caret and the selection.
    pub cursor: Cursor,
    /// The number of the box, which gives it the keyboard once.
    pub focus: u64,
}

/// **Edit**: the box opens on a card with the lines proposed, or the two sides one after the other when nothing is.
pub(super) fn open_box(state: &mut State, key: &ConflictKey) {
    let Some(page) = state.reviews.page.as_ref() else {
        return;
    };
    let Some(card) = page
        .cards
        .iter()
        .flat_map(|file| &file.conflicts)
        .find(|card| card.key == *key)
    else {
        return;
    };
    let lines: Vec<&str> = card.proposal.as_ref().map_or_else(
        || {
            card.current
                .lines
                .iter()
                .chain(&card.incoming.lines)
                .map(|line| line.text.as_str())
                .collect()
        },
        |proposal| proposal.lines.iter().map(|line| line.line.text.as_str()).collect(),
    );
    let Ok(document) = Document::from_bytes(lines.join("\n").as_bytes()) else {
        return;
    };
    state.reviews.opened += 1;
    state.reviews.lines = Some(LinesBox {
        key: key.clone(),
        document,
        cursor: Cursor::default(),
        focus: state.reviews.opened,
    });
}

/// What the keys and the pointer asked of the lines.
pub(super) fn write_lines(state: &mut State, commands: &[Command]) {
    let Some(lines) = state.reviews.lines.as_mut() else {
        return;
    };
    for command in commands {
        let _changes = apply(&mut lines.document, &mut lines.cursor, command);
    }
}

/// **Use these lines** or `Ctrl+Enter`: the lines written, each a line of the text, become the choice of the conflict;
/// gives the conflict they were written for.
pub(super) fn use_lines(state: &mut State) -> Option<ConflictKey> {
    let lines = state.reviews.lines.take()?;
    let folder = state.reviews.page.as_ref()?.folder.clone();
    let text = lines.document.buffer().text();
    let written: Vec<String> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').map(str::to_owned).collect()
    };
    let known: Fingerprinted = (lines.key.path.clone(), lines.key.fingerprint.clone());
    state.reviews.written.retain(|(_, other, _)| *other != known);
    state.reviews.written.push((folder, known, written));
    Some(lines.key)
}

/// The lines written for a conflict, when they were.
#[must_use]
pub fn written_of<'a>(state: &'a State, known: &Fingerprinted) -> &'a [String] {
    state
        .reviews
        .written
        .iter()
        .find(|(_, other, _)| other == known)
        .map_or(&[], |(_, _, lines)| lines.as_slice())
}

/// The box as the card draws it.
#[must_use]
pub fn box_state(state: &State) -> Option<WritingBox<'_>> {
    let lines = state.reviews.lines.as_ref()?;
    Some(WritingBox {
        key: &lines.key,
        buffer: lines.document.buffer(),
        cursor: lines.cursor,
        focus: Some(lines.focus),
    })
}
