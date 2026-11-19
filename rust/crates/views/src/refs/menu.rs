//! The menus of the branch rail: the menu of a reference, with what narrows the timeline to it, **Add to favorites**
//! or **Remove from favorites**, **Show in the timeline** and the operations of the reference or the stash, those of
//! later milestones greyed out; and the menu of a list, with **Sort by name** and **Sort by date**, **Create worktree…** or
//! **Add submodule…**.

use model::refs::SortOrder;
use ui::MenuEntry;

use super::ReferencesState;
use super::rows::Entry;
use crate::branch::{OperationEntry, ReferenceMenus, branch_entries, menu_items, remote_entries, tag_entries};
use crate::filter::{FilterEntry, filter_entries, filter_entry_words};
use crate::graph::with_compare;
use crate::graph::{StashEntry, stash_menu};
use crate::icons;
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
    /// The operation of the catalog it runs at `HEAD`, as **Add submodule…**.
    Run(&'static str),
    /// The operation at a place among those of the list, from [`list_operations`].
    Operation(usize),
    /// **Check tags on `<remote>`**, for the remote of `remote.default` (`tag.remote-marks`).
    CheckTags,
}

/// What the menu of the Tags list is told: the remote **Check tags on** asks, from `remote.default`, none without a
/// remote; and whether **Push tags…** is offered: before any check, or while the remote checked lacks a tag.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(super) struct TagsMenu {
    /// The remote of `remote.default`.
    pub(super) remote: Option<String>,
    /// **Check tags on `<remote>`**.
    pub(super) check: String,
    /// Whether **Push tags…** is in the menu.
    pub(super) offers_push: bool,
}

/// What the menu of the Tags list is told of the repository and of the marks of its tags.
pub(super) fn tags_menu(state: &ReferencesState<'_>) -> TagsMenu {
    let remote = model::remote::default(&state.repository.remotes);
    let tags = &state.references.tags;
    let lacks_one = state.tag_marks.is_none_or(|marks| marks.not_pushed(tags) > 0);
    TagsMenu {
        offers_push: remote.is_some() && !tags.is_empty() && lacks_one,
        check: remote.as_deref().map(words::check_tags_on).unwrap_or_default(),
        remote,
    }
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
                .map(|chosen| {
                    MenuEntry::Item(icons::action(
                        filter_icon(chosen),
                        filter_entry_words(chosen),
                        EntryCommand::Filter(chosen),
                    ))
                }),
        );
        entries.push(MenuEntry::Separator);
    }
    if entry.commit().is_some() {
        entries.push(MenuEntry::Item(icons::action(
            icons::TIMELINE,
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
        entries.push(MenuEntry::Item(icons::action(
            icons::FAVORITE,
            label,
            EntryCommand::Favorite,
        )));
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
        window: state.window,
        github: state.github,
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
        Entry::Worktree(worktree) => crate::worktree::worktree_entries(menus, worktree),
        Entry::Stash(_) | Entry::Submodule(_) => Vec::new(),
    }
}

/// The icon of an entry that changes the branches the timeline shows: the funnel narrows it to one history, the eye
/// shows more of it, the struck eye hides one.
fn filter_icon(entry: FilterEntry) -> ui::Icon {
    match entry {
        FilterEntry::ShowOnly => icons::FILTER,
        FilterEntry::ShowToo | FilterEntry::ShowAll => icons::SHOW,
        FilterEntry::Hide => icons::HIDE,
    }
}

/// The operations of a stash, as its row in the timeline has them.
fn stash_operations() -> Vec<MenuEntry<'static, EntryCommand>> {
    stash_menu()
        .into_iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(MenuEntry::Item(item.map(EntryCommand::Stash))),
            MenuEntry::Separator => Some(MenuEntry::Separator),
            MenuEntry::Submenu { .. } => None, // The menu of a stash has no submenu.
        })
        .collect()
}

/// The operations of a list: those of the **…** of the Worktrees list; **Fetch notes** and, when the repository has
/// notes, **Push notes** for a remote; none for the others.
pub(super) fn list_operations(state: &ReferencesState<'_>, list: &RailList) -> Vec<OperationEntry> {
    let menus = ReferenceMenus {
        repository: state.repository,
        open_in: state.open_in,
        window: state.window,
        github: state.github,
    };
    match list {
        RailList::Worktrees => crate::worktree::list_entries(menus),
        RailList::Remote(remote) => {
            let target = Target::Remote(remote.clone());
            let mut entries = vec![OperationEntry::runs(words::FETCH_NOTES, "note.fetch", target.clone())];
            if state.repository.notes.has_reference() {
                entries.push(OperationEntry::runs(words::PUSH_NOTES, "note.push", target));
            }
            entries
        }
        _ => Vec::new(), // Only the Worktrees list and the remotes run operations from their menu.
    }
}

/// The menu of a list, none for a list that has none: the order of the local branches, of each remote and of the
/// tags, checked as it is; the operations of the Worktrees list and of a remote, from [`list_operations`]; **Add
/// submodule…**; and, in the Tags list of a repository with a remote, **Check tags on `<remote>`** and **Push tags…**
/// (`tag.remote-marks`, `tag.push-all`).
pub(super) fn list_menu<'t>(
    list: &RailList,
    (order, tags): (SortOrder, &'t TagsMenu),
    operations: &'t [OperationEntry],
) -> Vec<MenuEntry<'t, ListCommand>> {
    let sort = |label, by| MenuEntry::Item(icons::choice(label, ListCommand::Sort(by), order == by));
    match list {
        RailList::Tags => {
            let mut entries = vec![
                sort(words::SORT_BY_NAME, SortOrder::Name),
                sort(words::SORT_BY_DATE, SortOrder::Date),
            ];
            if tags.remote.is_some() {
                entries.push(MenuEntry::Separator);
                entries.push(MenuEntry::Item(icons::action(
                    icons::READ_AGAIN,
                    &tags.check,
                    ListCommand::CheckTags,
                )));
                if tags.offers_push {
                    entries.push(MenuEntry::Item(icons::operation_item(
                        words::PUSH_TAGS,
                        "tag.push-all",
                        ListCommand::Run("tag.push-all"),
                    )));
                }
            }
            entries
        }
        RailList::Local => vec![
            sort(words::SORT_BY_NAME, SortOrder::Name),
            sort(words::SORT_BY_DATE, SortOrder::Date),
        ],
        RailList::Remote(_) => {
            let mut entries = vec![
                sort(words::SORT_BY_NAME, SortOrder::Name),
                sort(words::SORT_BY_DATE, SortOrder::Date),
                MenuEntry::Separator,
            ];
            entries.extend(menu_items(operations, ListCommand::Operation));
            entries
        }
        RailList::Worktrees => menu_items(operations, ListCommand::Operation),
        RailList::Submodules => vec![MenuEntry::Item(icons::operation_item(
            words::ADD_SUBMODULE,
            "submodule.add",
            ListCommand::Run("submodule.add"),
        ))],
        RailList::Favorites | RailList::Recent | RailList::Stashes => Vec::new(),
    }
}
