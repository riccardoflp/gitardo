//! The bar at the top of a Diff page: the arrows that go from change to change with where the current one is among
//! them (`diff.navigate`), the hint that `F7` goes on to the next file, the fold button (`diff.collapse-unchanged`), and
//! the title buttons that stage, unstage or discard the file (`diff.stage-file`, `diff.unstage-file`,
//! `diff.discard-file`).

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Key, Label, Modifiers, NamedKey, TextKind, Tone, Ui};

use super::{DiffIntent, DiffPageIntent, DiffPageState, DiffVersion};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::diff as words;

/// The bar, over the rectangle its caller clipped it to: the arrows and the count at the start, the fold button at the
/// end, and the hint between them.
pub(crate) fn bar_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, spacing.medium));
    let mut parts = Area::with_gap(inner, spacing.medium);
    let side = theme.sizes.control_height;
    let mut intents = Vec::new();
    let fold = parts.right(side);
    intents.extend(ui.clip(fold, |ui| collapse_unchanged_view(state, ui)));
    for (feature, icon, said) in file_buttons(state) {
        let cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        let button = Button::icon(icon, said).kind(ButtonKind::Icon);
        if ui
            .button(("diff file button", state.id, feature), cell, &button)
            .clicked
            && let Some(file) = state.files.first()
        {
            let target = Target::File(file.right.clone());
            intents.push(Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
        }
    }
    let navigation_width = 2.0 * side + spacing.medium + ui.measure("000 / 000", ui.text_style(TextKind::Detail));
    let navigation = parts.left(navigation_width.min(parts.rest().width));
    intents.extend(ui.clip(navigation, |ui| navigate_view(state, ui)));
    if let Some((hint, until)) = state.hint
        && ui.now() < until
    {
        ui.request_frame_at(until);
        let _ = ui.label(
            parts.rest(),
            &Label::new(hint.words()).kind(TextKind::Detail).tone(Tone::Accent),
        );
    }
    let separator = ui::Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(separator, ui::Axis::Horizontal);
    intents
}

/// The title buttons of a page of one file, from the right: **Discard changes** and **Stage file** when the right side
/// is the working tree and the left `HEAD` or the index (`diff.discard-file`, `diff.stage-file`), **Unstage file** when
/// the right side is the index (`diff.unstage-file`). A page of several files has none yet.
fn file_buttons(state: &DiffPageState<'_>) -> Vec<(&'static str, Icon, &'static str)> {
    if state.files.len() != 1 {
        return Vec::new();
    }
    match state.versions {
        (DiffVersion::Head | DiffVersion::Index, DiffVersion::WorkingTree) => vec![
            ("diff.discard-file", Icon::Close, words::DISCARD_FILE),
            ("diff.stage-file", Icon::Plus, words::STAGE_FILE),
        ],
        (_, DiffVersion::Index) => vec![("diff.unstage-file", Icon::Minus, words::UNSTAGE_FILE)],
        _ => Vec::new(),
    }
}

/// The arrows ▲ and ▼ and **3 / 7** beside them, over the rectangle its caller clipped it to, and the keys `F7` and
/// `Shift+F7` in either side: from one change to the next, the current one centered on both sides; past the last change
/// of a diff between `HEAD`, the index and the working tree, on to the next changed file after a second press; against
/// a commit, round to the first.
///
/// Feature: diff.navigate
pub fn navigate_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let side = theme.sizes.control_height;
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.small);
    let at = ui.now();
    let mut asked = Vec::new();
    let previous = Button::icon(Icon::ArrowUp, words::PREVIOUS_CHANGE).kind(ButtonKind::Icon);
    let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
    if ui.button(("diff previous", state.id), cell, &previous).clicked {
        asked.push(DiffPageIntent::Previous(at));
    }
    let next = Button::icon(Icon::ArrowDown, words::NEXT_CHANGE).kind(ButtonKind::Icon);
    let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
    if ui.button(("diff next", state.id), cell, &next).clicked {
        asked.push(DiffPageIntent::Next(at));
    }
    let said = if state.changes.is_empty() {
        words::NO_CHANGES.to_owned()
    } else {
        words::position(state.current, state.changes.len())
    };
    let _ = ui.label(
        parts.rest(),
        &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted),
    );
    for key in ui.unclaimed_keys() {
        match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Function(7)), Modifiers::NONE) => asked.push(DiffPageIntent::Next(at)),
            (Key::Named(NamedKey::Function(7)), Modifiers::SHIFT) => asked.push(DiffPageIntent::Previous(at)),
            _ => {}
        }
    }
    asked
        .into_iter()
        .map(|intent| Intent::Diff(DiffIntent::Page(state.id, intent)))
        .collect()
}

/// The fold button **Collapse unchanged regions**, over the rectangle its caller clipped it to, pressed while the
/// unchanged rows are folded: it folds them, or shows them all.
///
/// Feature: diff.collapse-unchanged
pub fn collapse_unchanged_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let side = ui.theme().sizes.control_height;
    let kind = if state.is_collapsed {
        ButtonKind::Outline
    } else {
        ButtonKind::Icon
    };
    let button = Button::icon(Icon::Minus, words::COLLAPSE_UNCHANGED).kind(kind);
    let cell = place(rect, (side, side), (Align::Start, Align::Center));
    if ui.button(("diff collapse", state.id), cell, &button).clicked {
        let intent = DiffPageIntent::Collapse(!state.is_collapsed);
        return vec![Intent::Diff(DiffIntent::Page(state.id, intent))];
    }
    Vec::new()
}
