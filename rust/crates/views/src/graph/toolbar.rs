//! The header (`graph.toolbar`): the branch checked out and how far it is from its upstream, **Search**, Sync and the
//! **…** menu, in the island under the tabs (D21, D50).

use std::path::PathBuf;

use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Button, ButtonKind, Description, Icon, Label, TextKind, Tone, Ui};

use super::more_menu::{MoreMenuState, more_menu_view};
use super::sync_menu::{SyncWords, sync_entries, sync_intent};
use super::toggle::{ReferencesShown, toggle_references_view};
use crate::intent::Intent;
use crate::line::{control_in, is_narrow};
use crate::search;
use crate::text::header as words;

/// What the header shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarState<'a> {
    /// What is checked out, once the repository is read.
    pub branch: Option<BranchChip<'a>>,
    /// How many commits Sync would bring and send, once the repository is read and its branch has an upstream.
    pub sync: Option<SyncCounts>,
    /// What **Sync** and its menu may run, once the repository is read.
    pub sync_menu: Option<SyncMenu>,
    /// What the **…** menu offers.
    pub more: MoreMenuState,
    /// Whether the branch rail is shown, once a repository is: the first button puts it away.
    pub references: Option<ReferencesShown>,
    /// Quick search.
    pub quick: search::QuickSearchState<'a>,
}

/// A repository of the workspace, as the picker lists it (`repo.name-repositories`).
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

/// What **Sync** and its menu may run: each operation once it is built, none while an operation runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncMenu {
    /// Whether an operation runs.
    pub is_running: bool,
    /// Whether each operation of **Sync** and its menu is built, in the order of `SYNC_FEATURES`.
    pub built: [bool; 7],
    /// The place among `PULL_WAYS` of the way **Pull** takes, as the settings `pull.rebase` and `pull.autostash` say.
    pub pull_way: usize,
}

/// The commits Sync would bring and send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncCounts {
    /// The commits to pull: how far the branch is behind its upstream.
    pub behind: u32,
    /// The commits to push: how far it is ahead.
    pub ahead: u32,
}

/// The header, over the rectangle its caller clipped it to: the button of the branch rail, the branch checked out,
/// then at its end **Search**, **Sync** with its menu and **…**. In a narrow window each keeps its icon and
/// gives up its words.
///
/// Feature: graph.toolbar
pub fn toolbar_view(state: &ToolbarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.header_background);
    ui.region("header", (rect, AccessRole::Header, words::HEADER), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.medium, 0.0));
        let mut area = Area::with_gap(inner, spacing.large);
        if let Some(references) = &state.references {
            let side = theme.sizes.control_height;
            let rect = control_in(ui, area.left(side), side);
            intents.extend(ui.clip(rect, |ui| toggle_references_view(references, ui)));
        }
        if let Some(branch) = &state.branch {
            branch_chip(branch, ui, &mut area);
        }
        more_button(state.more, ui, &mut area, &mut intents);
        sync_button((state.sync, state.sync_menu), ui, &mut area, &mut intents);
        let width = search::search_button_width(ui);
        let cell = control_in(ui, area.right(width), width);
        intents.extend(ui.clip(cell, |ui| search::quick_search_view(&state.quick, ui)));
    });
    intents
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

/// Sync, with how many commits it would bring and send, which runs a fetch, a pull and a push (`sync.sync`); and the
/// button of its menu (`sync_entries`).
fn sync_button(
    (counts, menu): (Option<SyncCounts>, Option<SyncMenu>),
    ui: &mut Ui<'_>,
    area: &mut Area,
    intents: &mut Vec<Intent>,
) {
    let menu = menu.unwrap_or_default();
    let can_sync = menu.built.first().copied().unwrap_or_default();
    // A narrow window keeps the icon of Sync and its counts, and gives up its word.
    let is_narrow = is_narrow(ui);
    let words = match (counts, is_narrow) {
        (Some(counts), false) => words::sync_counts(counts.behind, counts.ahead),
        (Some(counts), true) => words::counts_only(counts.behind, counts.ahead),
        (None, false) => words::SYNC.to_owned(),
        (None, true) => String::new(),
    };
    let mut button = Button::primary(&words).with_tooltip(words::SYNC_TOOLTIP);
    if is_narrow {
        button = button.with_icon(Icon::Refresh);
    }
    if !can_sync || menu.is_running {
        button = button.disabled();
    }
    let (width, _) = ui.button_size(&button);
    let side = ui.theme().sizes.control_height;
    let joined = ui.theme().sizes.spacing.small;
    // Sync and the button of its menu are one control, with a hair of room between them.
    let mut pair = Area::with_gap(area.right(width + joined + side), joined);
    let rect = control_in(ui, pair.right(side), side);
    let said = SyncWords::new(counts, &menu);
    let entries = sync_entries(&menu, &said);
    let chevron = Button::icon(Icon::ChevronDown, words::SYNC_MENU).kind(ButtonKind::Outline);
    if let Some(command) = ui.menu_button("sync menu", rect, &chevron, &entries) {
        intents.push(sync_intent(command));
    }
    let rect = control_in(ui, pair.rest(), width);
    if ui.button("sync", rect, &button).clicked {
        intents.push(run("sync.sync"));
    }
}

/// The intent that runs an operation of the catalog at `HEAD`.
fn run(feature: &'static str) -> Intent {
    Intent::Feature(crate::feature::FeatureIntent::Run(
        model::contract::FeatureId(feature),
        model::contract::Target::Head,
    ))
}

/// The **…** button at the end of the header, with its menu.
fn more_button(state: MoreMenuState, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    intents.extend(ui.clip(rect, |ui| more_menu_view(&state, ui)));
}
