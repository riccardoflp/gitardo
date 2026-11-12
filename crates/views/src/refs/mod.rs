//! The branch rail, on the left of the window (`refs.tree`): the legend and the filter of the timeline. At its top the
//! filter box (`refs.filter`) and, while the timeline is narrowed, the line of the branches shown (`refs.scope-line`);
//! under them **Favorites** and **Recent**, the local branches each with the dot of its lane, the branches of each
//! remote under its name, and **Kept aside**: the stashes, the worktrees, the tags and the submodules, in lists that
//! open and close. In the narrow layout it shows only the dots and the icons.

mod draw;
mod entry;
mod menu;
mod rows;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::time::Duration;

use model::CommitId;
use model::contract::OpenIn;
use model::filter::{BranchesShown, label};
use model::refs::{References, SortOrder};
use model::state::RepositoryState;
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Axis, Button, ButtonKind, Icon, InputBox, Key, Label, NamedKey, Rect, TextKind, Tone, Ui};

use crate::graph::HoverTarget;
use crate::intent::{FilterIntent, Intent, RailIntent, RailLists};
use crate::line::is_narrow;
use crate::text::rail as words;
use draw::{Context, RailReport, draw_row};
use rows::{is_filter_offered, rail_rows};

/// What the branch rail lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferencesState<'a> {
    /// The repository shown, as last read: its remotes, stashes and worktrees.
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
    /// The paths of the submodules, once they are read.
    pub submodules: &'a [String],
    /// What is typed in the filter box.
    pub typed: &'a str,
    /// The branches the timeline shows; none is everything.
    pub shown: Option<&'a BranchesShown>,
    /// Whether the rail lists only the branches the timeline shows: `branches.followGraphFilter`.
    pub follows_timeline: bool,
    /// The commit selected in the timeline, whose references are marked.
    pub selected: Option<&'a CommitId>,
    /// The order of the branches and tags: `branches.sortOrder`.
    pub sort: SortOrder,
    /// What a double click on a branch does, and which entry comes first in its menu: `branches.openIn`.
    pub open_in: OpenIn,
}

/// The filter box at the top of the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferencesFilter<'a> {
    /// What is typed in it.
    pub typed: &'a str,
}

/// The line at the top of the rail while the timeline is narrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeLine<'a> {
    /// The branches the timeline shows.
    pub shown: &'a BranchesShown,
    /// Whether the rail lists only those: `branches.followGraphFilter`.
    pub follows_timeline: bool,
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
    let is_icons = is_narrow(ui);
    let spacing = theme.sizes.spacing;
    let across = if is_icons {
        spacing.small + spacing.tiny
    } else {
        spacing.large
    };
    let mut report = RailReport::default();
    ui.region("rail region", (rect, AccessRole::Navigation, words::RAIL), |ui| {
        let mut area = Area::with_gap(inset(rect, Insets::symmetric(across, spacing.large)), spacing.medium);
        if !is_icons && is_filter_offered(state) {
            let filter = ReferencesFilter { typed: state.typed };
            report
                .intents
                .extend(ui.clip(area.top(theme.sizes.control_height), |ui| filter_view(&filter, ui)));
        }
        if let (Some(shown), false) = (state.shown, is_icons) {
            let line = ScopeLine {
                shown,
                follows_timeline: state.follows_timeline,
            };
            report
                .intents
                .extend(ui.clip(area.top(theme.sizes.control_height), |ui| scope_line_view(&line, ui)));
        }
        lists(state, ui, area.rest(), &mut report);
    });
    (report.intents, report.hover)
}

/// The lists of the rail, in an area that scrolls.
fn lists(state: &ReferencesState<'_>, ui: &mut Ui<'_>, rect: Rect, report: &mut RailReport) {
    let theme = *ui.theme();
    let is_icons = is_narrow(ui);
    let rows = rail_rows(state, is_icons);
    let height: f32 = rows.iter().map(|row| row.height(&theme)).sum();
    let context = Context { state, is_icons };
    let _ = ui.scroll_area("rail", rect, (rect.width, height), |ui, content| {
        let mut y = content.y;
        for row in &rows {
            let row_height = row.height(&theme);
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
    let input = InputBox::new(state.typed, words::FILTER).with_icon(Icon::Search);
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
        let button = Button::icon(Icon::Close, words::EMPTY_FILTER).kind(ButtonKind::Icon);
        if ui.button("empty rail filter", end, &button).clicked || is_escaped {
            intents.push(Intent::Rail(RailIntent::Type(String::new())));
        }
    }
    intents
}

/// The line at the top of the rail while the timeline is narrowed, over the rectangle its caller clipped it to: what
/// the timeline is narrowed to, the button that lists every branch with the others dimmed or only those shown, and
/// **Show all**.
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
        intents.push(Intent::Filter(FilterIntent::Show(None)));
    }
    let (tooltip, kind) = if state.follows_timeline {
        (words::LIST_ALL, ButtonKind::Icon)
    } else {
        (words::LIST_SHOWN, ButtonKind::Outline)
    };
    let list_all = Button::icon(Icon::Eye, tooltip).kind(kind);
    let eye = parts.right(theme.sizes.icon + 2.0 * spacing.small);
    let eye = Rect::new(
        eye.x,
        rect.y + spacing.tiny,
        eye.width,
        rect.height - 2.0 * spacing.tiny,
    );
    if ui.button("scope list all", eye, &list_all).clicked {
        intents.push(Intent::Filter(FilterIntent::FollowGraphFilter(!state.follows_timeline)));
    }
    let what = label(Some(state.shown));
    // The filter icon goes first while the words fit beside it.
    let words_width = ui.measure(&what, ui.text_style(TextKind::Body));
    if parts.rest().width >= words_width + theme.sizes.icon + spacing.small {
        ui.icon_in(parts.left(theme.sizes.icon), Icon::Filter, Tone::Accent);
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
