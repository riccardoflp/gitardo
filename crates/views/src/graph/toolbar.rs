//! The header (`graph.toolbar`): the repository with its menu, the branch checked out and how far it is from its
//! upstream, the branch filter, Quick search, the activity indicator, Sync, the button of the Activity drawer and the **…** menu.

use std::path::PathBuf;

use ui::layout::{Area, Insets, inset};
use ui::{
    AccessRole, Axis, Button, ButtonKind, Description, Icon, Label, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui,
};

use super::more_menu::{MoreMenuState, more_menu_view};
use super::toggle::{ReferencesShown, toggle_references_view};
use crate::activity::{StatusBarState, status_bar_view, status_bar_width};
use crate::filter::{BranchPicker, picker_button_width, picker_view};
use crate::intent::Intent;
use crate::line::{control_in, is_narrow};
use crate::search;
use crate::text::header as words;

/// The key of the menu of the repository.
const REPOSITORY_MENU: &str = "repository menu";

/// What the header shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarState<'a> {
    /// The pill of the repository and its menu.
    pub repository: RepositoryPill<'a>,
    /// What is checked out, once the repository is read.
    pub branch: Option<BranchChip<'a>>,
    /// What runs.
    pub status: StatusBarState<'a>,
    /// How many commits Sync would bring and send, once the repository is read and its branch has an upstream.
    pub sync: Option<SyncCounts>,
    /// What the **…** menu offers.
    pub more: MoreMenuState,
    /// Whether the Activity drawer is open.
    pub is_activity_open: bool,
    /// Whether the branch rail is shown, once a repository is: the first button puts it away.
    pub references: Option<ReferencesShown>,
    /// The branch filter, once a repository is read (`filter.picker`).
    pub filter: Option<BranchPicker<'a>>,
    /// Quick search.
    pub quick: search::QuickSearchState<'a>,
}

/// The pill of the repository: its name, and its menu of the repositories of the workspace and the recent ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryPill<'a> {
    /// The name of the repository shown, or of the workspace while it has none; none with no workspace.
    pub name: Option<&'a str>,
    /// The repositories to choose from, only when the workspace has more than one (`repo.select`).
    pub repositories: &'a [RepositoryEntry],
    /// The recent workspaces, the latest first, each by its folders (`repo.open-recent`).
    pub recent: &'a [String],
}

/// A repository of the workspace, as the menu and the picker list it (`repo.name-repositories`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryEntry {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub folder: PathBuf,
    /// Whether it is the one shown.
    pub is_current: bool,
}

/// What is checked out, as the branch of the header names it (`activity.status-bar`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchChip<'a> {
    /// The branch, or how `HEAD` stands without one.
    pub head: HeadName<'a>,
    /// How far the branch is from its upstream, `(ahead, behind)`, when it has one.
    pub upstream: Option<(u32, u32)>,
    /// The folder of the repository, for the tooltip.
    pub folder: &'a str,
}

/// What `HEAD` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadName<'a> {
    /// A branch, by its short name.
    Branch(&'a str),
    /// A commit with no branch, by its short hash.
    Detached(&'a str),
    /// Nothing yet: the repository has no commits.
    NoCommits,
}

/// The commits Sync would bring and send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncCounts {
    /// The commits to pull: how far the branch is behind its upstream.
    pub behind: u32,
    /// The commits to push: how far it is ahead.
    pub ahead: u32,
}

/// What an item of the repository menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepositoryCommand {
    /// Show a repository, by its place in the list.
    Select(usize),
    /// **Open folder…**.
    OpenFolder,
    /// Open a recent workspace, by its place in the list.
    OpenRecent(usize),
}

/// The header, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it further: `graph.toolbar` (the menu of Sync with **Fetch**, **Pull** and **Push**, and
/// **Sync** itself, in M4), `activity.status-bar`.
pub fn toolbar_view(state: &ToolbarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.header_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    ui.region("header", (rect, AccessRole::Header, words::REPOSITORY_TOOLTIP), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0));
        let mut area = Area::with_gap(inner, spacing.large);
        if let Some(references) = &state.references {
            let side = theme.sizes.control_height;
            let rect = control_in(ui, area.left(side), side);
            intents.extend(ui.clip(rect, |ui| toggle_references_view(references, ui)));
        }
        repository_pill(&state.repository, ui, &mut area, &mut intents);
        if let Some(branch) = &state.branch {
            branch_chip(branch, ui, &mut area);
        }
        if let Some(filter) = &state.filter {
            let width = picker_button_width(filter, ui);
            let rect = control_in(ui, area.left(width), width);
            intents.extend(ui.clip(rect, |ui| picker_view(filter, ui)));
        }
        more_button(state.more, ui, &mut area, &mut intents);
        activity_button(state.is_activity_open, ui, &mut area, &mut intents);
        sync_button(state.sync, ui, &mut area);
        // The indicator is declared even while it takes no room, so that it asks for the frame it appears in; it takes
        // only the room Quick search leaves.
        let room = area.rest().width - search::SEARCH_LEAST_WIDTH - spacing.large;
        let width = status_bar_width(&state.status, ui, room);
        let cell = if width > 0.0 {
            area.right(width)
        } else {
            Rect::new(inner.right(), inner.y, 0.0, inner.height)
        };
        intents.extend(ui.clip(cell, |ui| status_bar_view(&state.status, ui)));
        let middle = area.rest();
        intents.extend(ui.clip(middle, |ui| search::quick_search_view(&state.quick, ui)));
    });
    intents
}

/// The pill of the repository, which opens its menu.
fn repository_pill(pill: &RepositoryPill<'_>, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let accent = ui.theme().colors.accent;
    let name = pill.name.unwrap_or(words::NO_FOLDER);
    let button = Button::secondary(name)
        .kind(ButtonKind::Pill)
        .with_dot(accent)
        .with_trailing(Icon::ChevronDown)
        .with_tooltip(words::REPOSITORY_TOOLTIP);
    let (width, _) = ui.button_size(&button);
    let rect = control_in(ui, area.left(width), width);
    let recent_entries: Vec<MenuEntry<'_, RepositoryCommand>> = pill
        .recent
        .iter()
        .enumerate()
        .map(|(index, label)| MenuEntry::Item(MenuItem::new(label.as_str(), RepositoryCommand::OpenRecent(index))))
        .collect();
    let mut entries: Vec<MenuEntry<'_, RepositoryCommand>> = pill
        .repositories
        .iter()
        .enumerate()
        .map(|(index, repository)| {
            let item = MenuItem::new(repository.name.as_str(), RepositoryCommand::Select(index));
            MenuEntry::Item(if repository.is_current {
                item.with_icon(Icon::Check)
            } else {
                item
            })
        })
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(
        MenuItem::new(words::OPEN_FOLDER, RepositoryCommand::OpenFolder).with_icon(Icon::Folder),
    ));
    if !recent_entries.is_empty() {
        entries.push(MenuEntry::Submenu {
            label: words::OPEN_RECENT,
            icon: None,
            entries: &recent_entries,
        });
    }
    let chosen = ui.menu_button(REPOSITORY_MENU, rect, &button, &entries);
    match chosen {
        Some(RepositoryCommand::Select(index)) => {
            if let Some(repository) = pill.repositories.get(index) {
                intents.push(Intent::SelectRepository(repository.folder.clone()));
            }
        }
        Some(RepositoryCommand::OpenFolder) => intents.push(Intent::OpenFolder),
        Some(RepositoryCommand::OpenRecent(index)) => intents.push(Intent::OpenRecent(index)),
        None => {}
    }
}

/// The words of the branch of the header.
fn head_words(head: HeadName<'_>) -> String {
    match head {
        HeadName::Branch(name) => name.to_owned(),
        HeadName::Detached(short) => words::detached_at(short),
        HeadName::NoCommits => words::NO_COMMITS_YET.to_owned(),
    }
}

/// The branch checked out and how far it is from its upstream, with the folder of the repository in its tooltip.
/// Picking another branch from it comes with the everyday operations.
fn branch_chip(branch: &BranchChip<'_>, ui: &mut Ui<'_>, area: &mut Area) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let name = head_words(branch.head);
    let distance = branch
        .upstream
        .map(|(ahead, behind)| words::upstream_distance(ahead, behind));
    let shows_distance = !is_narrow(ui) && distance.is_some();
    let name_width = ui.measure(&name, ui.text_style(TextKind::Medium));
    let distance_width = match (&distance, shows_distance) {
        (Some(words), true) => ui.measure(words, ui.text_style(TextKind::Detail)) + spacing.medium,
        _ => 0.0,
    };
    let width = theme.sizes.icon + spacing.medium + name_width + distance_width + 2.0 * spacing.medium;
    let rect = control_in(ui, area.left(width), width);
    let id = ui.id("branch chip");
    let response = ui.interact(id, rect, ui::Sense::HOVER);
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Branch, Tone::Lane(0));
    let _ = ui.label(parts.left(name_width), &Label::new(&name).kind(TextKind::Medium));
    if let (Some(words), true) = (&distance, shows_distance) {
        let _ = ui.label(
            parts.rest(),
            &Label::new(words).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    let description = Description::new(AccessRole::Label, &name).value(distance.as_deref().unwrap_or_default());
    ui.describe(id, rect, &description);
    ui.tooltip(&response, branch.folder);
}

/// Sync, with how many commits it would bring and send; it runs nothing until the everyday operations.
fn sync_button(counts: Option<SyncCounts>, ui: &mut Ui<'_>, area: &mut Area) {
    let words = counts.map_or_else(
        || words::SYNC.to_owned(),
        |counts| words::sync_counts(counts.behind, counts.ahead),
    );
    let button = Button::primary(&words).disabled();
    let (width, _) = ui.button_size(&button);
    let rect = control_in(ui, area.right(width), width);
    let _ = ui.button("sync", rect, &button);
}

/// The button that opens and closes the Activity drawer.
fn activity_button(is_open: bool, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    let button = Button::icon(Icon::Info, words::ACTIVITY).kind(ButtonKind::Outline);
    if ui.button("activity", rect, &button).clicked {
        intents.push(if is_open {
            Intent::CloseActivity
        } else {
            Intent::OpenActivity
        });
    }
}

/// The **…** button at the end of the header, with its menu.
fn more_button(state: MoreMenuState, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    intents.extend(ui.clip(rect, |ui| more_menu_view(&state, ui)));
}
