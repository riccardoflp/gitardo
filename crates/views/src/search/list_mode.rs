//! The commits found shown as a list (`search.list-mode`), or the history of a commit alone (`search.go-to-hash`): the
//! bar above them that says what they are, as the bar of `filter.bar` does, with **Show all commits**.

use model::filter::BranchesShown;
use model::search::SearchMode;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Icon, Label, Rect, Tone, Ui};

use super::SearchIntent;
use crate::intent::Intent;
use crate::text::search as words;

/// What the timeline is narrowed to, besides the branches shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Narrowed<'a> {
    /// A search shown as a list.
    Search {
        /// How git looked.
        mode: SearchMode,
        /// What it looked for.
        text: &'a str,
    },
    /// The history of a commit on no branch or tag, by its short hash.
    Commit(&'a str),
}

/// The bar of a search shown as a list, or of the history of a commit alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListMode<'a> {
    /// What the timeline is narrowed to.
    pub narrowed: Narrowed<'a>,
    /// The branches shown, which a search is inside; none is everything.
    pub within: Option<&'a BranchesShown>,
    /// How many commits are listed, without the draft.
    pub listed: usize,
    /// Whether there are more commits than those listed.
    pub has_more: bool,
}

/// The bar above the commits of a search shown as a list or of the history of a commit alone, over the rectangle its
/// caller clipped it to: what the commits are and how many, and **Show all commits**. It is as tall as the bar of
/// `filter.bar` ([`crate::filter::bar_height`]), which it takes the place of.
///
/// Feature: search.list-mode
pub fn list_mode_view(state: &ListMode<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let said = words::list_bar(&state.narrowed, state.within, (state.listed, state.has_more));
    let mut intents = Vec::new();
    ui.region("search list bar", (rect, AccessRole::Group, words::LIST_BAR), |ui| {
        ui.fill_rect(rect, theme.colors.card_background);
        let line = Rect::new(
            rect.x,
            rect.bottom() - theme.sizes.border,
            rect.width,
            theme.sizes.border,
        );
        ui.separator(line, Axis::Horizontal);
        let mut parts = Area::with_gap(
            inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
            spacing.medium,
        );
        ui.icon_in(parts.left(theme.sizes.icon), Icon::Filter, Tone::Accent);
        let button = Button::secondary(words::SHOW_ALL_COMMITS).kind(ButtonKind::Outline);
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button("show all commits", cell, &button).clicked {
            intents.push(Intent::Search(SearchIntent::ShowAllCommits));
        }
        let _ = ui.label(parts.rest(), &Label::new(&said));
    });
    intents
}
