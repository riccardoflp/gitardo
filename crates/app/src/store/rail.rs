//! The branch rail and the branch filter as the store keeps them (`refs.tree`, `refs.filter`, `refs.favorite`,
//! `refs.sort`, `filter.picker`, `filter.list-too`, `filter.kinds`): the lists opened and closed, what is typed, the
//! references in their order and the branches checked out last, the favorites of each repository, the stashes unfolded
//! in the Stashes list (`stash.pane`).

use std::path::PathBuf;
use std::sync::Arc;

use model::filter::{ReferenceKind, kinds};
use model::refs::{References, SortOrder, recent, sort};
use views::{FilterIntent, RailIntent, RailLists};

use super::message::Effect;
use super::state::State;
use super::timeline;
use crate::persisted::rail::favorites_of;
use crate::persisted::timeline::remember;
use crate::persisted::{RepositoryFavorites, same_folder};
use crate::settings::{keys, sort_order_value};

/// The branch rail and the branch filter of a window.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Rail {
    /// The lists turned from how they start, and those listed in full; they stay so until the window is closed, from
    /// one repository to another.
    pub lists: RailLists,
    /// What is typed in the filter box of the rail.
    pub typed: String,
    /// The list of the branch filter, while it is open.
    pub picker: Option<FilterPicker>,
    /// The references of the repository shown, in the order of `branches.sortOrder`.
    pub references: Arc<References>,
    /// The local branches checked out last, the latest first (`refs.recent`).
    pub recent: Vec<String>,
    /// The stashes unfolded in the Stashes list, and what was read of their files (`stash.pane`).
    pub stashes: super::stashes::StashesList,
}

/// The list of the branch filter, open.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FilterPicker {
    /// What is typed in its box.
    pub typed: String,
    /// The size of the window it opened in.
    pub window: (f32, f32),
}

/// The order of the branches and tags in force: `branches.sortOrder`, by name while it is not set.
#[must_use]
pub fn sort_order(state: &State) -> SortOrder {
    state.settings.sort_order.unwrap_or_default()
}

/// Puts the references of the repository shown in their order, and works out **Recent**, after a read or a change of
/// the order.
pub fn arrange(state: &mut State) {
    let Some(read) = state.shown_read() else {
        return;
    };
    let repository = &read.state;
    let references = sort(&References::of(repository), sort_order(state));
    let names: Vec<String> = repository.branches.iter().map(|branch| branch.name.clone()).collect();
    let current = repository.branches.iter().find(|branch| branch.is_checked_out);
    let recent = recent(
        &repository.reflog_subjects,
        &names,
        current.map(|branch| branch.name.as_str()),
    );
    state.rail.references = Arc::new(references);
    state.rail.recent = recent;
    super::stashes::forget_gone(state);
}

/// The favorites of the repository shown.
#[must_use]
pub fn favorites(state: &State) -> &[String] {
    state
        .shown_folder()
        .map(|folder| favorites_of(&state.persisted.favorites, folder))
        .unwrap_or_default()
}

/// What the branch rail is asked.
pub fn reduce_rail(state: &mut State, intent: RailIntent) -> Vec<Effect> {
    let lists = &mut state.rail.lists;
    match intent {
        RailIntent::Toggle(list) => {
            if !lists.turned.remove(&list) {
                let _turned = lists.turned.insert(list.clone());
            }
            if !lists.is_open(&list) {
                let _closed = lists.in_full.remove(&list);
            }
            Vec::new()
        }
        RailIntent::InFull(list) => {
            let _new = lists.in_full.insert(list);
            Vec::new()
        }
        RailIntent::Type(typed) => {
            state.rail.typed = typed;
            Vec::new()
        }
        RailIntent::Favorite { reference, is_favorite } => favorite(state, &reference, is_favorite),
        RailIntent::Sort(order) => {
            state.settings.sort_order = Some(order);
            arrange(state);
            write_setting(state, keys::SORT_ORDER, sort_order_value(order))
        }
        RailIntent::ToggleStash(commit) => super::stashes::toggle(state, commit),
        RailIntent::ShowStashes => super::stashes::show(state),
    }
}

/// A branch added to the favorites of the repository shown, or taken out; those whose branch no longer exists are
/// forgotten then. Kept between runs.
fn favorite(state: &mut State, reference: &str, is_favorite: bool) -> Vec<Effect> {
    let Some(folder) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let existing = &state.rail.references;
    let exists = |name: &str| {
        let local = name
            .strip_prefix(ReferenceKind::Local.prefix())
            .is_some_and(|name| existing.branches.iter().any(|branch| branch.name == name));
        let remote = name
            .strip_prefix(ReferenceKind::Remote.prefix())
            .is_some_and(|name| existing.remote_branches.iter().any(|branch| branch.name == name));
        local || remote
    };
    let mut references: Vec<String> = favorites(state)
        .iter()
        .filter(|name| exists(name) && name.as_str() != reference)
        .cloned()
        .collect();
    if is_favorite {
        references.push(reference.to_owned());
    }
    let entry = RepositoryFavorites {
        folder: folder.clone(),
        references,
    };
    remember(&mut state.persisted.favorites, entry, |kept| {
        same_folder(&kept.folder, &folder)
    });
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// What is asked of the branches the timeline shows.
pub fn reduce_filter(state: &mut State, intent: FilterIntent) -> Vec<Effect> {
    match intent {
        FilterIntent::Show(shown) => timeline::show(state, shown.as_ref()),
        FilterIntent::SwitchKind(kind) => {
            let shown = kinds(state.timeline.shown.as_ref(), kind);
            timeline::show(state, shown.as_ref())
        }
        FilterIntent::OpenPicker { window } => {
            state.rail.picker = Some(FilterPicker {
                typed: String::new(),
                window,
            });
            Vec::new()
        }
        FilterIntent::ClosePicker => {
            state.rail.picker = None;
            Vec::new()
        }
        FilterIntent::Type(typed) => {
            if let Some(picker) = &mut state.rail.picker {
                picker.typed = typed;
            }
            Vec::new()
        }
        FilterIntent::FollowGraphFilter(is_on) => {
            state.settings.follows_graph_filter = is_on;
            write_setting(state, keys::FOLLOW_GRAPH_FILTER, json::Value::Bool(is_on))
        }
    }
}

/// Writes a setting in the user's settings file, when the system says where it is.
fn write_setting(state: &State, key: &'static str, value: json::Value) -> Vec<Effect> {
    let file: Option<PathBuf> = state.user_settings_file.clone();
    file.map(|file| Effect::WriteSetting { file, key, value })
        .into_iter()
        .collect()
}
