//! The menus of the branch rail: the menu of a reference, with what narrows the timeline to it, **Add to favorites**
//! or **Remove from favorites**, **Show in the timeline** and the operations of the reference or the stash, those of
//! later milestones greyed out; and the menu of a list, with **Sort by name** and **Sort by date**, **Create worktree…** or
//! **Add submodule…**.

use model::refs::SortOrder;
use ui::{Icon, MenuEntry, MenuItem};

use super::ReferencesState;
use super::rows::Entry;
use crate::branch::{OperationEntry, ReferenceMenus, branch_entries, menu_items, remote_entries, tag_entries};
use crate::filter::{FilterEntry, filter_entries, filter_entry_words};
use crate::graph::with_compare;
use crate::graph::{StashEntry, stash_menu};
use crate::intent::RailList;
use crate::text::rail as words;
use model::contract::Target;

/// What an entry of the menu of a reference does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum EntryCommand {
    /// Changes the branches the timeline shows.
    Filter(FilterEntry),
    /// **Add to favorites** or **Remove from favorites**.
    Favorite,
    /// **Show in the timeline**: its commit, selected.
    ShowInTimeline,
    /// The operation at a place among those of the reference.
    Operation(usize),
    /// An entry of the menu of a stash: **Apply**, **Pop**, **Create branch from stash…** or **Drop**.
    Stash(StashEntry),
}

/// What an entry of the menu of a list does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListCommand {
    /// **Sort by name** or **Sort by date**.
    Sort(SortOrder),
    /// An operation of a later milestone, greyed out until then.
    Later,
}

/// The menu of a reference of the rail, with the operations of the reference, from [`operations`].
pub(super) fn entry_menu<'t>(
    state: &ReferencesState<'_>,
    entry: &Entry<'_>,
    operations: &'t [OperationEntry],
) -> Vec<MenuEntry<'t, EntryCommand>> {
    let mut entries = Vec::new();
    if let Some(full) = entry.full_name() {
        entries.extend(
            filter_entries(state.shown, &full, state.references)
                .into_iter()
                .map(|chosen| MenuEntry::Item(MenuItem::new(filter_entry_words(chosen), EntryCommand::Filter(chosen)))),
        );
        entries.push(MenuEntry::Separator);
    }
    if entry.commit().is_some() {
        entries.push(MenuEntry::Item(MenuItem::new(
            words::SHOW_IN_TIMELINE,
            EntryCommand::ShowInTimeline,
        )));
    }
    if let Some(full) = entry
        .full_name()
        .filter(|_| matches!(entry, Entry::Branch(_) | Entry::Remote(_)))
    {
        let is_favorite = state.favorites.contains(&full);
        let label = if is_favorite {
            words::REMOVE_FROM_FAVORITES
        } else {
            words::ADD_TO_FAVORITES
        };
        entries.push(MenuEntry::Item(
            MenuItem::new(label, EntryCommand::Favorite).with_icon(Icon::Star),
        ));
    }
    let mut operations = menu_items(operations, EntryCommand::Operation);
    if let Entry::Stash(_) = entry {
        operations = stash_operations();
    }
    if !operations.is_empty() {
        entries.push(MenuEntry::Separator);
        entries.extend(operations);
    }
    entries
}

/// The operations of a branch, a remote branch or a tag, as their menus have them wherever they are listed; none for
/// the others.
pub(super) fn operations(state: &ReferencesState<'_>, entry: &Entry<'_>) -> Vec<OperationEntry> {
    let menus = ReferenceMenus {
        repository: state.repository,
        open_in: state.open_in,
    };
    match entry {
        Entry::Branch(branch) => with_compare(
            menus,
            branch_entries(menus, branch),
            &Target::Branch(branch.name.clone()),
            &branch.commit,
        ),
        Entry::Remote(remote) => with_compare(
            menus,
            remote_entries(menus, remote),
            &Target::RemoteBranch(remote.name.clone()),
            &remote.commit,
        ),
        Entry::Tag(tag) => with_compare(
            menus,
            tag_entries(menus, tag),
            &Target::Tag(tag.name.clone()),
            &tag.commit,
        ),
        Entry::Stash(_) | Entry::Worktree(_) | Entry::Submodule(_) => Vec::new(),
    }
}

/// The operations of a stash, as its row in the timeline has them.
fn stash_operations() -> Vec<MenuEntry<'static, EntryCommand>> {
    stash_menu()
        .into_iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(MenuEntry::Item(MenuItem::new(
                item.label,
                EntryCommand::Stash(item.value),
            ))),
            MenuEntry::Separator => Some(MenuEntry::Separator),
            MenuEntry::Submenu { .. } => None, // The menu of a stash has no submenu.
        })
        .collect()
}

/// The menu of a list, none for a list that has none: the order of the local branches, of each remote and of the
/// tags, checked as it is; **Create worktree…** and **Add submodule…**, greyed out until their operations exist.
pub(super) fn list_menu(list: &RailList, order: SortOrder) -> Vec<MenuEntry<'static, ListCommand>> {
    let sort = |label, by| {
        let item = MenuItem::new(label, ListCommand::Sort(by));
        MenuEntry::Item(if order == by { item.with_icon(Icon::Check) } else { item })
    };
    match list {
        RailList::Local | RailList::Remote(_) | RailList::Tags => vec![
            sort(words::SORT_BY_NAME, SortOrder::Name),
            sort(words::SORT_BY_DATE, SortOrder::Date),
        ],
        RailList::Worktrees => vec![MenuEntry::Item(
            MenuItem::new(words::CREATE_WORKTREE, ListCommand::Later).disabled(),
        )],
        RailList::Submodules => vec![MenuEntry::Item(
            MenuItem::new(words::ADD_SUBMODULE, ListCommand::Later).disabled(),
        )],
        RailList::Favorites | RailList::Recent | RailList::Stashes => Vec::new(),
    }
}
