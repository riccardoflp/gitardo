//! The draft of the uncommitted changes, the first card of the timeline (`graph.uncommitted-row`): its message, what
//! it changes, **Review** and **Commit**, and its context menu. The message is the one of the commit box of the Changes
//! page, typed there; the stashes of its menu come with the operations of the stashes, and until then are shown and
//! cannot be chosen.

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, LineCap, MenuEntry, MenuItem, Point, Stroke, TextKind, TextRun, Ui};

use super::state::DraftState;
use crate::changes::{ChangesIntent, CommitBox};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::timeline as words;

/// The dashes and gaps of the edge of the card.
const DASH: (f32, f32) = (6.0, 4.0);

/// The draft, as a row of the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UncommittedRow<'a> {
    /// What it changes and offers.
    pub draft: DraftState,
    /// Whether it is selected.
    pub is_selected: bool,
    /// The commit box of the Changes page, whose message and **Commit** the draft shows.
    pub commit: CommitBox<'a>,
}

/// An entry of the context menu of the draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DraftEntry {
    /// **Stash changes…** (`stash.push`).
    Stash,
    /// **Stash staged changes…** (`stash.push-staged`).
    StashStaged,
    /// **Stage all changes** (`changes.stage-all`).
    StageAll,
    /// **Unstage all changes** (`changes.unstage-all`).
    UnstageAll,
    /// **Discard all changes…** (`changes.discard-all`).
    DiscardAll,
}

/// The entries of the context menu of the draft, each when the card offers it: no stash before the first commit, no
/// staging everything while a file is in conflict, no discarding during an operation stopped half way.
#[must_use]
pub fn draft_entries(draft: &DraftState) -> Vec<DraftEntry> {
    let mut entries = Vec::new();
    if draft.has_commit {
        entries.push(DraftEntry::Stash);
        if draft.staged > 0 {
            entries.push(DraftEntry::StashStaged);
        }
    }
    if draft.unstaged > 0 && draft.conflicted == 0 {
        entries.push(DraftEntry::StageAll);
    }
    if draft.staged > 0 {
        entries.push(DraftEntry::UnstageAll);
    }
    if !draft.is_operation_stopped {
        entries.push(DraftEntry::DiscardAll);
    }
    entries
}

/// The words of an entry.
fn entry_words(entry: DraftEntry) -> &'static str {
    match entry {
        DraftEntry::Stash => words::STASH_CHANGES,
        DraftEntry::StashStaged => words::STASH_STAGED,
        DraftEntry::StageAll => words::STAGE_ALL,
        DraftEntry::UnstageAll => words::UNSTAGE_ALL,
        DraftEntry::DiscardAll => words::DISCARD_ALL,
    }
}

/// The operation of an entry, and what it is invoked on: the whole working tree.
#[must_use]
pub fn entry_operation(entry: DraftEntry) -> Option<&'static str> {
    match entry {
        DraftEntry::Stash => Some("stash.push"),
        DraftEntry::StashStaged => Some("stash.push-staged"),
        DraftEntry::StageAll => Some("changes.stage-all"),
        DraftEntry::UnstageAll => Some("changes.unstage-all"),
        DraftEntry::DiscardAll => Some("changes.discard-all"),
    }
}

/// The context menu of the draft: its entries, those whose operations do not exist yet not to be chosen, and none
/// while an operation runs.
#[must_use]
pub fn draft_menu(draft: &DraftState, is_running: bool) -> Vec<MenuEntry<'static, DraftEntry>> {
    draft_entries(draft)
        .into_iter()
        .map(|entry| {
            let item = MenuItem::new(entry_words(entry), entry);
            let item = if entry_operation(entry).is_none() || is_running {
                item.disabled()
            } else {
                item
            };
            MenuEntry::Item(item)
        })
        .collect()
}

/// What an entry chosen in the context menu of the draft asks: its operation, on the whole working tree.
#[must_use]
pub fn draft_entry_intent(entry: DraftEntry) -> Option<Intent> {
    entry_operation(entry).map(|feature| Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head)))
}

/// The card of the draft over the rectangle of the row its caller clipped it to, beside its dashed ring in the graph:
/// the message, `<n> files changed · <m> staged`, **Review** and **Commit**.
///
/// Feature: graph.uncommitted-row
pub fn uncommitted_row_view(state: &UncommittedRow<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let card = inset(rect, Insets::symmetric(0.0, spacing.small));
    let radius = theme.sizes.radius.large;
    ui.fill_rounded_rect(card, radius, theme.colors.card_background);
    let edge = Stroke {
        width: theme.sizes.border,
        cap: LineCap::Butt,
        color: if state.is_selected {
            theme.colors.accent
        } else {
            theme.colors.muted_foreground
        },
    };
    ui.dashed_border(card, radius, (edge, DASH));
    let mut area = Area::with_gap(inset(card, Insets::symmetric(spacing.large, 0.0)), spacing.medium);
    let intents = buttons(state, ui, &mut area);
    let lines = area.rest();
    let first = ui.text_style(TextKind::Medium);
    let second = ui.text_style(TextKind::Detail);
    let gap = spacing.tiny + 1.0;
    let height = ui.line_height(first) + gap + ui.line_height(second);
    let top = place(lines, (lines.width, height), (Align::Start, Align::Center)).y;
    let typed = state.commit.message.trim();
    let (text, color) = if typed.is_empty() {
        (words::DESCRIBE_NEXT_COMMIT, theme.colors.input_placeholder)
    } else {
        (typed.lines().next().unwrap_or(typed), theme.colors.foreground)
    };
    let message = TextRun {
        text,
        style: first,
        color,
        origin: Point::new(lines.x, top),
        max_width: Some(lines.width),
    };
    let _ = ui.text(&message);
    let said = words::files_changed(state.draft.files, state.draft.staged);
    let changes = TextRun {
        text: &said,
        style: second,
        color: theme.colors.muted_foreground,
        origin: Point::new(lines.x, top + ui.line_height(first) + gap),
        max_width: Some(lines.width),
    };
    let _ = ui.text(&changes);
    intents
}

/// **Commit**, disabled as the one of the Changes page is, and **Review**, at the right of the card.
fn buttons(state: &UncommittedRow<'_>, ui: &mut Ui<'_>, area: &mut Area) -> Vec<Intent> {
    let mut intents = Vec::new();
    let mut commit = Button::primary(state.commit.button_words());
    if !state.commit.can_commit() {
        commit = commit.disabled();
    }
    let review = Button::outline(words::REVIEW);
    for (key, button, intent) in [
        ("commit", commit, ChangesIntent::Commit),
        ("review", review, ChangesIntent::Open),
    ] {
        let (width, height) = ui.button_size(&button);
        let cell = place(area.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button(key, cell, &button).clicked {
            intents.push(Intent::Changes(intent));
        }
    }
    intents
}
