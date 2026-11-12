//! The menus of the branch rail: the menu of a reference, with what narrows the timeline to it, **Add to favorites**
//! or **Remove from favorites**, **Show in the timeline** and the operations of later milestones greyed out; and the
//! menu of a list, with **Sort by name** and **Sort by date**, **Create worktree…** or **Add submodule…**.

use model::refs::SortOrder;
use ui::{Icon, MenuEntry, MenuItem};

use super::ReferencesState;
use super::rows::Entry;
use crate::filter::{FilterEntry, filter_entries, filter_entry_words};
use crate::intent::RailList;
use crate::text::rail as words;
use crate::text::timeline as stash_words;

/// What an entry of the menu of a reference does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum EntryCommand {
    /// Changes the branches the timeline shows.
    Filter(FilterEntry),
    /// **Add to favorites** or **Remove from favorites**.
    Favorite,
    /// **Show in the timeline**: its commit, selected.
    ShowInTimeline,
    /// An operation of a later milestone, greyed out until then.
    Later,
}

/// What an entry of the menu of a list does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListCommand {
    /// **Sort by name** or **Sort by date**.
    Sort(SortOrder),
    /// An operation of a later milestone, greyed out until then.
    Later,
}

/// An item greyed out until its operation exists.
fn later(label: &str) -> MenuEntry<'_, EntryCommand> {
    MenuEntry::Item(MenuItem::new(label, EntryCommand::Later).disabled())
}

/// The menu of a reference of the rail. `check_out` is the words of **Check out `<branch>`** for a local branch.
pub(super) fn entry_menu<'t>(
    state: &ReferencesState<'_>,
    entry: &Entry<'_>,
    check_out: &'t str,
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
    let operations = later_operations(entry, check_out);
    if !operations.is_empty() {
        entries.push(MenuEntry::Separator);
        entries.extend(operations);
    }
    entries
}

/// The operations of a reference that come in later milestones, greyed out.
fn later_operations<'t>(entry: &Entry<'_>, check_out: &'t str) -> Vec<MenuEntry<'t, EntryCommand>> {
    match entry {
        Entry::Branch(branch) if branch.is_checked_out => vec![later(words::RENAME)],
        Entry::Branch(_) => vec![later(check_out), later(words::RENAME), later(words::DELETE)],
        Entry::Remote(_) => vec![later(words::CHECK_OUT_AS_LOCAL), later(words::DELETE)],
        Entry::Tag(_) => vec![later(words::DELETE)],
        Entry::Stash(_) => vec![
            later(stash_words::APPLY),
            later(stash_words::POP),
            later(stash_words::CREATE_BRANCH_FROM_STASH),
            later(stash_words::DROP),
        ],
        Entry::Worktree(_) | Entry::Submodule(_) => Vec::new(),
    }
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
