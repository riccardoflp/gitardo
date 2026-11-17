//! Folding the commits a merge brought in (`graph.fold-merge`) and every merge at once (`graph.fold-all`).

use model::CommitId;
use ui::layout::Area;
use ui::{Button, MenuEntry, Rect, Sense, Tone, Ui};

use super::intent::GraphIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::timeline as words;

/// The arrow after the message of a merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldsState<'a> {
    /// The merge.
    pub merge: &'a CommitId,
    /// Whether it brought in commits among those loaded, which can be folded.
    pub is_foldable: bool,
    /// Whether it is folded.
    pub is_folded: bool,
    /// Whether the arrow shows: while the pointer is on the row or the merge is selected.
    pub is_shown: bool,
}

/// The arrow after the message of a merge, over the rectangle its caller clipped it to: it folds the commits the merge
/// brought in, or shows them again. A merge that brought nothing in has no arrow.
///
/// Feature: graph.fold-merge
pub fn fold_merge_view(state: &FoldsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    if !state.is_foldable || !(state.is_shown || state.is_folded) {
        return Vec::new();
    }
    let rect = ui.current_clip();
    let (icon, tooltip, intent) = if state.is_folded {
        (
            icons::FOLDED,
            words::UNFOLD_MERGE,
            GraphIntent::Unfold(state.merge.clone()),
        )
    } else {
        (
            icons::UNFOLDED,
            words::FOLD_MERGE,
            GraphIntent::Fold(state.merge.clone()),
        )
    };
    let button = Button::icon(icon, tooltip);
    if ui.button(("fold arrow", state.merge.as_str()), rect, &button).clicked {
        return vec![Intent::Graph(intent)];
    }
    Vec::new()
}

/// What **Fold all merges** and **Unfold all merges** in **…** stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldAllState {
    /// The entry chosen.
    pub chosen: FoldAllEntry,
    /// Whether every merge is folded: **Fold all merges** again unfolds them.
    pub is_all_folded: bool,
}

/// An entry of **…** about every merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FoldAllEntry {
    /// **Fold all merges**.
    Fold,
    /// **Unfold all merges**.
    Unfold,
}

/// What choosing **Fold all merges** or **Unfold all merges** in **…** asks: **Fold all merges** checked, or
/// **Unfold all merges**, shows every commit again and forgets the folds.
///
/// Feature: graph.fold-all
pub fn fold_all_view(state: &FoldAllState, _ui: &mut Ui<'_>) -> Vec<Intent> {
    let intent = match (state.chosen, state.is_all_folded) {
        (FoldAllEntry::Fold, false) => GraphIntent::FoldAll,
        (FoldAllEntry::Fold, true) | (FoldAllEntry::Unfold, _) => GraphIntent::UnfoldAll,
    };
    vec![Intent::Graph(intent)]
}

/// An entry of the context menu of a merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MergeEntry {
    /// **Fold the commits it merged into one row**.
    Fold,
    /// **Show the commits it merged again**.
    Unfold,
}

/// The context menu of a commit: for a merge that brought commits in, its fold; nothing else yet.
#[must_use]
pub fn merge_menu(is_foldable: bool, is_folded: bool) -> Vec<MenuEntry<'static, MergeEntry>> {
    match (is_foldable, is_folded) {
        (_, true) => vec![MenuEntry::Item(icons::action(
            icons::UNFOLD,
            words::UNFOLD_MERGE,
            MergeEntry::Unfold,
        ))],
        (true, false) => vec![MenuEntry::Item(icons::action(
            icons::FOLD,
            words::FOLD_MERGE,
            MergeEntry::Fold,
        ))],
        (false, false) => Vec::new(),
    }
}

/// A folded row: the icon of unfolding and what it holds, `3 commits of feature`.
pub(super) fn fold_row(ui: &mut Ui<'_>, rect: Rect, title: &str) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    ui.icon_in(area.left(theme.sizes.icon), crate::icons::FOLDED, Tone::Muted);
    let _ = ui.label(area.rest(), &ui::Label::new(title).tone(Tone::Muted));
    let id = ui.id(("fold row", rect.y.to_bits()));
    let response = ui.interact(id, rect, Sense::HOVER);
    ui.tooltip(&response, words::CLICK_TO_SHOW);
}
