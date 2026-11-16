//! The rows of the branch rail under its filter box, in their order: each list with its title and, while it is open,
//! its entries: what the timeline does not show dimmed, and at most 100 of them until **Show all `<n>`…**. While something is typed in the filter box, one list of the entries of every list
//! that have it, the best first, takes their place.

use std::collections::HashSet;

use model::filter::{ReferenceKind, rank_names, ticked};
use model::state::{Branch, RemoteBranch, Stash, Tag, Worktree};
use model::submodule::Submodule;

use super::ReferencesState;
use super::stashes::{StashLine, with_stash_rows};
use crate::intent::RailList;

/// How many entries a list shows until **Show all `<n>`…**.
pub(super) const MOST_LISTED: usize = 100;
/// From how many branches, remote branches, tags and stashes together the filter box is offered.
pub(super) const FILTER_FROM: usize = 15;

/// A row of the rail.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum RailRow<'a> {
    /// The title of what is kept aside.
    KeptTitle,
    /// The title of the one list of the matches, while something is typed in the filter box (`refs.filter`).
    Matches {
        /// How many entries have what is typed.
        matches: usize,
        /// How many entries the lists have.
        all: usize,
    },
    /// The title of a list that is not kept aside, which opens and closes it, with how many it lists.
    Heading(ListHead),
    /// A list kept aside, with how many it holds, which a click opens or closes.
    Kept(ListHead),
    /// An entry of a list open.
    Entry {
        /// The list it is in.
        list: RailList,
        /// What it is.
        entry: Entry<'a>,
        /// How it is marked.
        marks: Marks,
    },
    /// How to make a stash, under the title of the Stashes list open with none (`stash.pane`).
    NoStashes,
    /// A line under a stash unfolded: **Loading…**, **No files**, or one of its files (`stash.pane`).
    StashLine {
        /// The stash.
        stash: &'a Stash,
        /// What the line is.
        line: StashLine<'a>,
    },
    /// **Show all `<n>`…** under the first entries of a long list.
    InFull {
        /// The list.
        list: RailList,
        /// How many entries it has.
        count: usize,
    },
    /// Room between two lists.
    Gap,
}

/// The title of a list: which, how many it lists, whether it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ListHead {
    /// The list.
    pub(super) list: RailList,
    /// How many entries match what is typed; all of them while nothing is.
    pub(super) matches: usize,
    /// How many entries it lists.
    pub(super) all: usize,
    /// Whether something is typed in the filter box.
    pub(super) is_typed: bool,
    /// Whether it is open.
    pub(super) is_open: bool,
}

/// An entry of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry<'a> {
    /// A local branch.
    Branch(&'a Branch),
    /// A remote branch.
    Remote(&'a RemoteBranch),
    /// A tag.
    Tag(&'a Tag),
    /// A stash.
    Stash(&'a Stash),
    /// A worktree.
    Worktree(&'a Worktree),
    /// A submodule.
    Submodule(&'a Submodule),
}

impl Entry<'_> {
    /// The name the entry is listed and filtered by.
    pub(super) fn name(&self) -> String {
        match self {
            Entry::Branch(branch) => branch.name.clone(),
            Entry::Remote(branch) => branch.name.clone(),
            Entry::Tag(tag) => tag.name.clone(),
            Entry::Stash(stash) => stash.message.clone(),
            Entry::Worktree(worktree) => worktree_folder(worktree),
            Entry::Submodule(submodule) => submodule.folder.clone(),
        }
    }

    /// The full name of a branch, a remote branch or a tag; none for what the timeline does not narrow to.
    pub(super) fn full_name(&self) -> Option<String> {
        match self {
            Entry::Branch(branch) => Some(ReferenceKind::Local.full_name(&branch.name)),
            Entry::Remote(branch) => Some(ReferenceKind::Remote.full_name(&branch.name)),
            Entry::Tag(tag) => Some(ReferenceKind::Tag.full_name(&tag.name)),
            Entry::Stash(_) | Entry::Worktree(_) | Entry::Submodule(_) => None,
        }
    }

    /// The commit the entry is at, when it has one.
    pub(super) fn commit(&self) -> Option<&model::CommitId> {
        match self {
            Entry::Branch(branch) => Some(&branch.commit),
            Entry::Remote(branch) => Some(&branch.commit),
            Entry::Tag(tag) => Some(&tag.commit),
            Entry::Stash(stash) => Some(&stash.commit),
            Entry::Worktree(worktree) => worktree.commit.as_ref(),
            Entry::Submodule(_) => None,
        }
    }

    /// Whether it is what is checked out here: the branch checked out, or the worktree of this window.
    pub(super) fn is_current(&self) -> bool {
        match self {
            Entry::Branch(branch) => branch.is_checked_out,
            Entry::Worktree(worktree) => worktree.is_current,
            _ => false,
        }
    }
}

/// How an entry is marked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) struct Marks {
    /// It is a favorite: it has a star.
    pub(super) is_favorite: bool,
    /// The timeline leaves it out.
    pub(super) is_dimmed: bool,
    /// It is at the commit selected in the timeline.
    pub(super) is_selected: bool,
}

/// The rows of the rail of a repository. In the narrow layout there are no titles, no favorites, no recent branches
/// and no entries kept aside.
pub(super) fn rail_rows<'a>(state: &ReferencesState<'a>, is_icons: bool) -> Vec<RailRow<'a>> {
    let mut rows = Vec::new();
    let shown: Option<HashSet<String>> = state
        .shown
        .map(|shown| ticked(Some(shown), state.references).into_iter().collect());
    let lister = Lister { state, shown, is_icons };
    if !is_icons && !state.typed.is_empty() {
        return with_stash_rows(lister.matches(), &state.stashes);
    }
    for (list, entries) in branch_lists(state) {
        let is_extra = matches!(list, RailList::Favorites | RailList::Recent);
        if is_extra && (entries.is_empty() || is_icons) {
            continue;
        }
        if list != RailList::Favorites && !rows.is_empty() && !matches!(rows.last(), Some(RailRow::Gap)) {
            rows.push(RailRow::Gap);
        }
        lister.add(&mut rows, &list, entries);
    }
    rows.push(RailRow::Gap);
    if !is_icons {
        rows.push(RailRow::KeptTitle);
    }
    for (list, entries) in kept_lists(state) {
        lister.add(&mut rows, &list, entries);
    }
    if is_icons {
        rows
    } else {
        with_stash_rows(rows, &state.stashes)
    }
}

/// Whether the filter box is offered: 15 names or more among the branches, the remote branches, the tags and the
/// stashes, or something typed in it.
pub(super) fn is_filter_offered(state: &ReferencesState<'_>) -> bool {
    let references = state.references;
    let count = references.branches.len()
        + references.remote_branches.len()
        + references.tags.len()
        + state.repository.stashes.len();
    count >= FILTER_FROM || !state.typed.is_empty()
}

/// The lists of branches, in their order, each with its entries: **Favorites** and **Recent**, the local branches, and
/// the branches of each remote.
fn branch_lists<'a>(state: &ReferencesState<'a>) -> Vec<(RailList, Vec<Entry<'a>>)> {
    let references = state.references;
    let favorites = state
        .favorites
        .iter()
        .filter_map(|full| favorite_entry(state, full))
        .collect();
    let recent = state
        .recent
        .iter()
        .filter_map(|name| references.branches.iter().find(|branch| &branch.name == name))
        .map(Entry::Branch)
        .collect();
    let mut lists = vec![
        (RailList::Favorites, favorites),
        (RailList::Recent, recent),
        (RailList::Local, references.branches.iter().map(Entry::Branch).collect()),
    ];
    for remote in &state.repository.remotes {
        let branches = references
            .remote_branches
            .iter()
            .filter(|branch| &branch.remote == remote)
            .map(Entry::Remote)
            .collect();
        lists.push((RailList::Remote(remote.clone()), branches));
    }
    lists
}

/// The entry of a favorite, while its branch exists.
fn favorite_entry<'a>(state: &ReferencesState<'a>, full_name: &str) -> Option<Entry<'a>> {
    let references = state.references;
    if let Some(name) = full_name.strip_prefix(ReferenceKind::Local.prefix()) {
        return references
            .branches
            .iter()
            .find(|branch| branch.name == name)
            .map(Entry::Branch);
    }
    let name = full_name.strip_prefix(ReferenceKind::Remote.prefix())?;
    references
        .remote_branches
        .iter()
        .find(|branch| branch.name == name)
        .map(Entry::Remote)
}

/// The lists kept aside, each with its entries: the submodules only when there are some.
fn kept_lists<'a>(state: &ReferencesState<'a>) -> Vec<(RailList, Vec<Entry<'a>>)> {
    let repository = state.repository;
    let mut lists = vec![
        (RailList::Stashes, repository.stashes.iter().map(Entry::Stash).collect()),
        (
            RailList::Worktrees,
            repository.worktrees.iter().map(Entry::Worktree).collect(),
        ),
        (RailList::Tags, state.references.tags.iter().map(Entry::Tag).collect()),
    ];
    if !repository.submodules.is_empty() {
        lists.push((
            RailList::Submodules,
            repository.submodules.iter().map(Entry::Submodule).collect(),
        ));
    }
    lists
}

/// What the lists are made with: the rail, the references the timeline shows, and the layout.
struct Lister<'s, 'a> {
    /// The rail.
    state: &'s ReferencesState<'a>,
    /// The full names of the references the timeline shows, while it is narrowed.
    shown: Option<HashSet<String>>,
    /// Whether the rail shows only icons.
    is_icons: bool,
}

impl<'a> Lister<'_, 'a> {
    /// The one list of the matches of what is typed: the entries of every list but **Favorites** and **Recent**, which
    /// hold local branches already listed, whose names have it, the best first (`filter.rank-names`), and among those
    /// that match as well in the order of the lists.
    fn matches(&self) -> Vec<RailRow<'a>> {
        let candidates: Vec<(RailList, Entry<'a>, Marks)> = branch_lists(self.state)
            .into_iter()
            .chain(kept_lists(self.state))
            .filter(|(list, _)| !matches!(list, RailList::Favorites | RailList::Recent))
            .flat_map(|(list, entries)| {
                entries
                    .into_iter()
                    .map(|entry| (list.clone(), entry, self.marks(&entry)))
                    .collect::<Vec<_>>()
            })
            .collect();
        let names: Vec<String> = candidates.iter().map(|(_, entry, _)| entry.name()).collect();
        let ranked = rank_names(&names, self.state.typed);
        let mut rows = vec![RailRow::Matches {
            matches: ranked.len(),
            all: candidates.len(),
        }];
        rows.extend(
            ranked
                .into_iter()
                .filter_map(|index| candidates.get(index).cloned())
                .map(|(list, entry, marks)| RailRow::Entry { list, entry, marks }),
        );
        rows
    }

    /// Adds a list: its title, and while it is open its entries, at most 100 until it is listed in full, the one
    /// checked out among them.
    fn add(&self, rows: &mut Vec<RailRow<'a>>, list: &RailList, entries: Vec<Entry<'a>>) {
        let marked: Vec<(Entry<'a>, Marks)> = entries.into_iter().map(|entry| (entry, self.marks(&entry))).collect();
        let all = marked.len();
        let names: Vec<String> = marked.iter().map(|(entry, _)| entry.name()).collect();
        let typed = self.state.typed;
        let ranked: Vec<(Entry<'a>, Marks)> = rank_names(&names, typed)
            .into_iter()
            .filter_map(|index| marked.get(index).copied())
            .collect();
        let is_typed = !typed.is_empty();
        let is_open = (is_typed && !ranked.is_empty()) || self.state.lists.is_open(list);
        let head = ListHead {
            list: list.clone(),
            matches: ranked.len(),
            all,
            is_typed,
            is_open,
        };
        let is_kept = list.is_kept_aside();
        if !self.is_icons || is_kept {
            rows.push(if is_kept {
                RailRow::Kept(head)
            } else {
                RailRow::Heading(head)
            });
        }
        let shows_entries = if self.is_icons { !is_kept } else { is_open };
        if shows_entries {
            self.add_entries(rows, list, ranked);
        }
    }

    /// Adds the entries of an open list.
    fn add_entries(&self, rows: &mut Vec<RailRow<'a>>, list: &RailList, ranked: Vec<(Entry<'a>, Marks)>) {
        let count = ranked.len();
        let is_cut = count > MOST_LISTED && !self.state.lists.is_in_full(list);
        let row = |(entry, marks): (Entry<'a>, Marks)| RailRow::Entry {
            list: list.clone(),
            entry,
            marks,
        };
        if !is_cut {
            rows.extend(ranked.into_iter().map(row));
            return;
        }
        let current = ranked
            .iter()
            .skip(MOST_LISTED)
            .find(|(entry, _)| entry.is_current())
            .copied();
        rows.extend(ranked.into_iter().take(MOST_LISTED).map(row));
        rows.extend(current.map(row));
        rows.push(RailRow::InFull {
            list: list.clone(),
            count,
        });
    }

    /// How an entry is marked: dimmed when the timeline leaves it out.
    fn marks(&self, entry: &Entry<'_>) -> Marks {
        let full = entry.full_name();
        let is_dimmed = match (&self.shown, &full) {
            (Some(names), Some(full)) => !names.contains(full),
            _ => false,
        };
        Marks {
            is_favorite: full
                .as_ref()
                .is_some_and(|full| self.state.favorites.iter().any(|favorite| favorite == full)),
            is_dimmed,
            is_selected: self
                .state
                .selected
                .is_some_and(|selected| entry.commit() == Some(selected)),
        }
    }
}

/// A worktree as the rail names it: the name of its folder.
pub(super) fn worktree_folder(worktree: &Worktree) -> String {
    worktree
        .folder
        .file_name()
        .map_or_else(
            || worktree.folder.to_string_lossy(),
            |name| std::path::Path::new(name).to_string_lossy(),
        )
        .into_owned()
}
