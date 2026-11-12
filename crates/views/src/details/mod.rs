//! The inspector, beside the timeline: the card of the selected commit (`details.show`).

use ui::layout::{Insets, inset};
use ui::{AccessRole, Ui};

use crate::intent::Intent;
use crate::text::timeline as words;

/// What the inspector shows. Nothing can be selected yet, so it is an empty card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DetailsState;

/// The inspector, along the rectangle its caller clipped it to: a card, empty until a commit can be selected.
///
/// Feature cards that fill it: `details.show` (the message, the author, the date, the hash and parents, the files of
/// the selected commit and its actions), `details.show-more`, `details.show-all-files`, `details.stash` (a stash
/// selected), `compare.in-place`; in the narrow layout it opens over the timeline (`graph.view`).
pub fn show(_state: &DetailsState, ui: &mut Ui<'_>) -> Vec<Intent> {
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
        |_| {},
    );
    Vec::new()
}
