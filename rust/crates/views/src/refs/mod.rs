//! The branch rail, on the left of the window (`refs.tree`): the legend and the filter of the timeline. It lists what
//! its own filter chooses (`refs.list-filter`, `refs.listed`), whatever the timeline shows. At its top the filter box
//! (`refs.filter`) and, while its filter lists less than everything, the line of what it lists (`refs.scope-line`);
//! under them **Favorites** and **Recent**, the local branches each with the dot of its lane, the branches of each
//! remote under its name, and **Kept aside**: the stashes, the worktrees, the tags and the submodules, in lists that
//! open and close. The branches the timeline leaves out are dimmed (`filter.list-too`). Narrower than the side column
//! can be dragged, it shows only the button of its filter, the dots and the icons.

mod draw;
mod entry;
mod list_filter;
mod menu;
#[cfg(test)]
mod notes_tests;
mod rows;
mod stashes;
#[cfg(test)]
mod stashes_tests;
mod submodules;
#[cfg(test)]
mod submodules_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod worktree_tests;

use std::collections::HashMap;
use std::time::Duration;

use model::CommitId;
use model::contract::OpenIn;
use model::filter::{BranchesShown, label};
use model::refs::{References, SortOrder};
use model::state::RepositoryState;
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Axis, Button, ButtonKind, InputBox, Key, Label, NamedKey, Rect, TextKind, Tone, Ui};

use crate::graph::HoverTarget;
use crate::icons;
use crate::intent::{Intent, RailIntent, RailLists};
use crate::line::is_icon_rail;
use crate::text::rail as words;
use draw::{Context, RailReport, draw_row, row_height};
use rows::{is_filter_offered, rail_rows};

pub use list_filter::{ListFilterOpen, ListFilterState, list_filter_view};
#[cfg(test)]
pub(crate) use stashes::drop_stash_icon;
pub use stashes::{StashFilesRead, StashesShown};

/// What the branch rail lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferencesState<'a> {
    /// The repository shown, as last read: its remotes, stashes, worktrees and submodules.
    pub repository: &'a RepositoryState,
    /// Its branches, remote branches and tags, in the order of `refs.sort`.
    pub references: &'a References,
    /// The lists turned from how they start, and those listed in full.
    pub lists: &'a RailLists,
    /// The color of the line of each local branch in the timeline, by its name; a branch whose commit is not shown has
    /// none, and its dot is muted.
    pub branch_colors: &'a HashMap<String, usize>,
    /// The same for each remote branch, by its name, as `origin/main`.
    pub remote_colors: &'a HashMap<String, usize>,
    /// The favorites of the repository, by their full names, in the order they were added (`refs.favorite`).
    pub favorites: &'a [String],
    /// The local branches checked out last, the latest first (`refs.recent`).
    pub recent: &'a [String],
    /// What is typed in the filter box.
    pub typed: &'a str,
    /// The branches the timeline shows; none is everything. Those it leaves out are dimmed.
    pub shown: Option<&'a BranchesShown>,
    /// The filter of the rail: what it lists, apart from what the timeline shows.
    pub list_filter: ListFilterState<'a>,
    /// The commit selected in the timeline, whose references are marked.
    pub selected: Option<&'a CommitId>,
    /// The order of the branches and tags: `branches.sortOrder`.
    pub sort: SortOrder,
    /// What a double click on a branch does, and which entry comes first in its menu: `branches.openIn`.
    pub open_in: OpenIn,
    /// What the menus and the list of the worktrees are told of the window.
    pub window: crate::worktree::WorktreeWindow<'a>,
    /// The stashes unfolded and what was read of their files, for the Stashes list (`stash.pane`).
    pub stashes: StashesShown<'a>,
    /// The repository on GitHub and its pull requests, in the tooltips and the menus of the branches.
    pub github: crate::github::GitHubShown<'a>,
}

/// The filter box at the top of the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferencesFilter<'a> {
    /// What is typed in it.
    pub typed: &'a str,
}

/// The line at the top of the rail while it lists less than everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeLine<'a> {
    /// What the rail lists.
    pub listed: &'a BranchesShown,
}

/// **Add to favorites** or **Remove from favorites**, chosen on a branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Favorites<'a> {
    /// The full name of the branch.
    pub reference: &'a str,
    /// Whether it is a favorite now.
    pub is_favorite: bool,
}

/// The branch rail, over the rectangle its caller clipped it to.
///
/// A click on a branch or a remote branch narrows the timeline to its history and selects its tip, a second click
/// shows everything again; a click on a tag, a stash or a worktree goes to its commit. The menus of each reference
/// and of each list, **Favorites** and **Recent**, **Show all `<n>`…** past 100 entries. Feature cards that fill it
/// further: `branch.open` (a double click), `drag.drop`, `graph.pull-request-badge`, the operations of `branch.`,
/// `tag.`, `stash.`, `worktree.` and `submodule.` in their menus.
///
/// Feature: refs.tree
pub fn tree_view(state: &ReferencesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    rail_view(state, ui).0
}

/// The branch rail, and the reference the pointer rests on with since when, whose history the timeline brings out
/// (`graph.hover-history`).
pub(crate) fn rail_view(
    state: &ReferencesState<'_>,
    ui: &mut Ui<'_>,
) -> (Vec<Intent>, Option<(HoverTarget, Duration)>) {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.rail_background);
    let edge = Rect::new(
        rect.right() - theme.sizes.border,
        rect.y,
        theme.sizes.border,
        rect.height,
    );
    ui.separator(edge, Axis::Vertical);
    let is_icons = is_icon_rail(rect.width);
    let spacing = theme.sizes.spacing;
    let across = if is_icons {
        spacing.small + spacing.tiny
    } else {
        spacing.large
    };
    let mut report = RailReport::default();
    ui.region("rail region", (rect, AccessRole::Navigation, words::RAIL), |ui| {
        let mut area = Area::with_gap(inset(rect, Insets::symmetric(across, spacing.small)), spacing.small);
        if is_icons {
            let button = area.top(theme.sizes.control_height);
            report
                .intents
                .extend(ui.clip(button, |ui| list_filter_view(&state.list_filter, ui)));
        }
        if !is_icons && is_filter_offered(state) {
            let filter = ReferencesFilter { typed: state.typed };
            report
                .intents
                .extend(ui.clip(area.top(theme.sizes.control_height), |ui| filter_view(&filter, ui)));
        }
        if let (Some(listed), false) = (state.list_filter.listed, is_icons) {
            let line = ScopeLine { listed };
            report
                .intents
                .extend(ui.clip(area.top(theme.sizes.control_height), |ui| scope_line_view(&line, ui)));
        }
        lists((state, is_icons), ui, area.rest(), &mut report);
    });
    (report.intents, report.hover)
}

/// The lists of the rail, in an area that scrolls.
fn lists((state, is_icons): (&ReferencesState<'_>, bool), ui: &mut Ui<'_>, rect: Rect, report: &mut RailReport) {
    let rows = rail_rows(state, is_icons);
    let heights: Vec<f32> = rows.iter().map(|row| row_height(ui, row, rect.width)).collect();
    let height: f32 = heights.iter().sum();
    let context = Context { state, is_icons };
    let _ = ui.scroll_area("rail", rect, (rect.width, height), |ui, content| {
        let mut y = content.y;
        for (row, &row_height) in rows.iter().zip(&heights) {
            let row_rect = Rect::new(content.x, y, content.width, row_height);
            y += row_height;
            if row_rect.bottom() < rect.y || row_rect.y > rect.bottom() {
                continue;
            }
            draw_row(ui, context, row, row_rect, report);
        }
    });
}

/// The filter box at the top of the rail, over the rectangle its caller clipped it to: what is typed leaves in each
/// list the entries whose names have it, the best first, and opens the lists with matches. `Escape` or the button at
/// its end empties it.
///
/// Feature: refs.filter
pub fn filter_view(state: &ReferencesFilter<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut intents = Vec::new();
    let input = InputBox::new(state.typed, words::FILTER).with_icon(icons::SEARCH);
    let id = ui.id("rail filter");
    if let Some(text) = ui.input_box("rail filter", rect, &input).changed_text {
        intents.push(Intent::Rail(RailIntent::Type(text)));
    }
    let is_escaped = ui
        .unclaimed_keys_of(id)
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Escape));
    if !state.typed.is_empty() {
        let side = rect.height;
        let end = Rect::new(rect.right() - side, rect.y, side, side);
        let button = Button::icon(icons::CLOSE, words::EMPTY_FILTER).kind(ButtonKind::Icon);
        if ui.button("empty rail filter", end, &button).clicked || is_escaped {
            intents.push(Intent::Rail(RailIntent::Type(String::new())));
        }
    }
    intents
}

/// The line at the top of the rail while its own filter lists less than everything, over the rectangle its caller
/// clipped it to: what it lists, and **Show all**, which lists everything again and leaves the timeline as it is.
///
/// Feature: refs.scope-line
pub fn scope_line_view(state: &ScopeLine<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_inactive_selection);
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.small, 0.0)), spacing.tiny);
    let show_all = Button::secondary(words::SHOW_ALL).kind(ButtonKind::Icon);
    let (width, _) = ui.button_size(&show_all);
    if ui.button("scope show all", parts.right(width), &show_all).clicked {
        intents.push(Intent::Rail(RailIntent::List(None)));
    }
    let what = label(Some(state.listed));
    // The filter icon goes first while the words fit beside it.
    let words_width = ui.measure(&what, ui.text_style(TextKind::Body));
    if parts.rest().width >= words_width + theme.sizes.icon + spacing.small {
        ui.icon_in(parts.left(theme.sizes.icon), icons::FILTER, Tone::Accent);
    }
    let _ = ui.label(parts.rest(), &Label::new(&what));
    intents
}

/// **Add to favorites** or **Remove from favorites** chosen on a branch: it is a favorite after, or no longer.
///
/// Feature: refs.favorite
pub fn favorite_view(state: &Favorites<'_>, _ui: &mut Ui<'_>) -> Vec<Intent> {
    favorite_intents(state)
}

/// What **Add to favorites** or **Remove from favorites** asks.
fn favorite_intents(state: &Favorites<'_>) -> Vec<Intent> {
    vec![Intent::Rail(RailIntent::Favorite {
        reference: state.reference.to_owned(),
        is_favorite: !state.is_favorite,
    })]
}
