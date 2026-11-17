//! The header (`graph.toolbar`): where one is (`graph.where`), **Search**, Sync, the menu of the view and the **…**
//! menu, in a bar on the ground across the top of the window, always shown (D56).

use std::path::PathBuf;

use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Button, ButtonKind, Ui};

use super::location::{WhereState, where_view};
use super::more_menu::{MoreMenuState, more_menu_view};
use super::sync_menu::{SyncWords, sync_entries, sync_intent};
use super::view_menu::{ViewMenuState, view_menu_view};
use crate::icons;
use crate::intent::Intent;
use crate::line::{control_in, is_narrow};
use crate::search;
use crate::text::header as words;

/// What the header shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarState<'a> {
    /// Where one is: the repository, the worktree and the branch shown, once a workspace is open.
    pub place: Option<WhereState<'a>>,
    /// How many commits Sync would bring and send, once the repository is read and its branch has an upstream.
    pub sync: Option<SyncCounts>,
    /// What **Sync** and its menu may run, once the repository is read.
    pub sync_menu: Option<SyncMenu>,
    /// What the menu of the view offers: the switches of the timeline.
    pub view: ViewMenuState,
    /// What the **…** menu offers: the actions.
    pub more: MoreMenuState,
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

/// The header, over the rectangle its caller clipped it to, on the ground of the window: where one is, then at its end
/// **Search**, **Sync** with its menu, the menu of the view and **…**. In a narrow window each keeps its icon and gives
/// up its words.
///
/// Feature: graph.toolbar
pub fn toolbar_view(state: &ToolbarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.region("header", (rect, AccessRole::Header, words::HEADER), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.small, 0.0));
        let mut area = Area::with_gap(inner, spacing.large);
        more_button(state.more, ui, &mut area, &mut intents);
        view_button(state.view, ui, &mut area, &mut intents);
        sync_button((state.sync, state.sync_menu), ui, &mut area, &mut intents);
        let width = search::search_button_width(ui);
        let cell = control_in(ui, area.right(width), width);
        intents.extend(ui.clip(cell, |ui| search::quick_search_view(&state.quick, ui)));
        if let Some(place) = &state.place {
            intents.extend(ui.clip(area.rest(), |ui| where_view(place, ui)));
        }
    });
    intents
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
        button = button.with_icon(icons::SYNC);
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
    let chevron = Button::icon(icons::CHOOSE, words::SYNC_MENU).kind(ButtonKind::Outline);
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

/// The button of the menu of the view, right before **…**, with its menu.
fn view_button(state: ViewMenuState, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    intents.extend(ui.clip(rect, |ui| view_menu_view(&state, ui)));
}

/// The **…** button at the end of the header, with its menu.
fn more_button(state: MoreMenuState, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    intents.extend(ui.clip(rect, |ui| more_menu_view(&state, ui)));
}
