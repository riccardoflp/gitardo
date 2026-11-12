//! A hash typed with **Loaded** chosen that no loaded commit has (`search.go-to-hash`): `Enter ↵` stands in the place
//! of the count, and `Enter` asks git for the commit wherever it is.

use ui::layout::Align;
use ui::{Label, Sense, TextKind, Tone, Ui};

use super::search_box::SearchBox;
use crate::intent::Intent;
use crate::text::search as words;

/// How wide `Enter ↵` is at the end of the box.
pub(crate) fn go_to_hash_width(ui: &Ui<'_>) -> f32 {
    ui.measure(words::ENTER, ui.text_style(TextKind::Detail)) + 2.0 * ui.theme().sizes.spacing.medium
}

/// `Enter ↵` over the rectangle its caller clipped it to, with its tooltip, while the box offers to go to a hash;
/// `Enter` itself is read by the box.
///
/// Feature: search.go-to-hash
pub fn go_to_hash_view(state: &SearchBox<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !state.is_hash_offered {
        return Vec::new();
    }
    let rect = ui.current_clip();
    let label = Label::new(words::ENTER)
        .kind(TextKind::Detail)
        .tone(Tone::Accent)
        .align(Align::Center);
    let _ = ui.label(rect, &label);
    let id = ui.id("go to hash");
    let response = ui.interact(id, rect, Sense::HOVER);
    ui.tooltip(&response, words::ENTER_TOOLTIP);
    Vec::new()
}
