//! The inspector, beside the timeline or over it: the card of the selected commit (`details.show`). For now it shows
//! what the row of the timeline holds: the subject, the hash, the author and the date; the draft says what it changes.

use model::commit::Commit;
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Label, TextKind, Tone, Ui};

use crate::graph::DraftState;
use crate::intent::Intent;
use crate::text::dates::full_date;
use crate::text::timeline as words;

/// What the inspector shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DetailsState<'a> {
    /// The row selected in the timeline; none while nothing is.
    pub selected: Option<SelectedDetails<'a>>,
}

/// The row selected, as the timeline read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedDetails<'a> {
    /// The draft of the uncommitted changes.
    Draft(DraftState),
    /// A commit.
    Commit(&'a Commit),
    /// A stash.
    Stash {
        /// The commit git keeps it as.
        commit: &'a Commit,
        /// Its place among the stashes.
        index: usize,
    },
}

/// The inspector, along the rectangle its caller clipped it to: a card with the row selected.
///
/// Feature cards that fill it: `details.show` (the message, the author, the date, the hash and parents, the files of
/// the selected commit and its actions), `details.show-more`, `details.show-all-files`, `details.stash` (a stash
/// selected), `compare.in-place`; in the narrow layout it opens over the timeline (`graph.view`).
pub fn show(state: &DetailsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let column = ui.current_clip();
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.large + theme.sizes.spacing.small;
    let card = inset(
        column,
        Insets {
            left: 0.0,
            top: padding,
            right: padding,
            bottom: padding,
        },
    );
    ui.card(card, theme.sizes.radius.large + theme.sizes.spacing.small);
    ui.region(
        "inspector region",
        (card, AccessRole::Complementary, words::INSPECTOR),
        |ui| {
            if let Some(selected) = state.selected {
                selected_card(ui, card, selected);
            }
        },
    );
    Vec::new()
}

/// The lines of the card: a title, then what is known of the row.
fn selected_card(ui: &mut Ui<'_>, card: ui::Rect, selected: SelectedDetails<'_>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(
        card,
        Insets::symmetric(spacing.large + spacing.small, spacing.large + spacing.small),
    );
    let mut area = Area::with_gap(inner, spacing.medium);
    let line = theme.sizes.row_height;
    let (title, commit) = match selected {
        SelectedDetails::Draft(draft) => {
            let _ = ui.label(
                area.top(line),
                &Label::new(words::UNCOMMITTED_CHANGES).kind(TextKind::Strong),
            );
            let said = words::files_changed(draft.files, draft.staged);
            let _ = ui.label(area.top(line), &Label::new(&said).tone(Tone::Muted));
            return;
        }
        SelectedDetails::Commit(commit) => (commit.subject.clone(), commit),
        SelectedDetails::Stash { commit, index } => (format!("stash@{{{index}}} \u{b7} {}", commit.subject), commit),
    };
    let _ = ui.label(area.top(line), &Label::new(&title).kind(TextKind::Strong));
    let _ = ui.label(
        area.top(line),
        &Label::new(commit.id.as_str()).kind(TextKind::Hash).tone(Tone::Muted),
    );
    let _ = ui.label(area.top(line), &Label::new(&commit.author_name));
    let _ = ui.label(
        area.top(line),
        &Label::new(&full_date(commit.author_date.0)).tone(Tone::Muted),
    );
}
