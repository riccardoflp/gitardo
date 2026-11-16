//! What the **▾** of a tab lists, and the name of a tab while it shows a worktree or a submodule of its repository
//! (`repo.group-tabs`).

use std::path::{Path, PathBuf};

use model::repo::TabGroup;
use model::state::Worktree;
use views::repo::TabMemberEntry;
use views::text::repo as words;

use super::super::state::{Shown, State};
use super::{Tab, kept_folder};
use crate::persisted::same_folder;

/// What a tab shows, and the repositories of its group it can show.
pub(super) struct TabShows<'a> {
    /// The folder it shows: its own, or a worktree's or a submodule's.
    pub folder: PathBuf,
    /// The entries of its **▾**; none when its repository has no other worktree and no submodule.
    pub members: Vec<TabMemberEntry>,
    /// The name of the worktree or the submodule it shows, put after its own name; none while it shows its own.
    pub shown_label: Option<String>,
    /// The tab.
    tab: &'a Tab,
}

impl<'a> TabShows<'a> {
    /// What a tab shows, from the groups of the repositories of the workspace and what the tab and its group read last.
    pub(super) fn new(state: &'a State, (index, tab): (usize, &'a Tab), groups: &[TabGroup], name: &str) -> Self {
        let is_shown = state.tabs.shown == Some(index);
        let folder = if is_shown {
            state.shown_folder().cloned()
        } else {
            tab.kept.as_deref().and_then(kept_folder).map(Path::to_path_buf)
        }
        .unwrap_or_else(|| tab.folder.clone());
        let group = groups.iter().find(|group| group.holds(&tab.folder));
        let mut shows = TabShows {
            folder,
            members: Vec::new(),
            shown_label: None,
            tab,
        };
        let Some(group) = group else {
            return shows;
        };
        let worktrees = shows.worktrees(state, is_shown, group);
        let mut members = vec![TabMemberEntry {
            label: name.to_owned(),
            folder: tab.folder.clone(),
            depth: 0,
            is_shown: same_folder(&shows.folder, &tab.folder),
        }];
        for worktree in worktrees {
            let is_shown = same_folder(&shows.folder, &worktree.folder);
            if is_shown {
                let branch = worktree.branch.as_deref().map(short_branch);
                shows.shown_label = Some(branch.map_or_else(|| folder_name(&worktree.folder), str::to_owned));
            }
            members.push(TabMemberEntry {
                label: worktree_entry(&worktree),
                folder: worktree.folder,
                depth: 0,
                is_shown,
            });
        }
        for submodule in group.members.iter().filter(|member| member.superproject.is_some()) {
            let label = path_from(&tab.folder, &submodule.folder);
            let is_shown = same_folder(&shows.folder, &submodule.folder);
            if is_shown {
                shows.shown_label = Some(label.clone());
            }
            members.push(TabMemberEntry {
                label,
                folder: submodule.folder.clone(),
                depth: depth(group, &submodule.folder),
                is_shown,
            });
        }
        if shows.shown_label.is_none() && !same_folder(&shows.folder, &tab.folder) {
            shows.shown_label = Some(folder_name(&shows.folder));
        }
        if members.len() > 1 {
            shows.members = members;
        }
        shows
    }

    /// The other worktrees of the tab's repository: from what the tab, or a worktree of its group, read last; from the
    /// worktrees of the group found in the workspace when none was read.
    fn worktrees(&self, state: &State, is_shown: bool, group: &TabGroup) -> Vec<Worktree> {
        let is_submodule = |folder: &Path| {
            group
                .members
                .iter()
                .any(|member| member.superproject.is_some() && same_folder(&member.folder, folder))
        };
        let own = if is_shown {
            state.workspace.as_ref().map(|workspace| &workspace.shown)
        } else {
            self.tab.kept.as_ref().and_then(|kept| kept.shown.as_ref())
        };
        let kept = self.tab.members.iter().filter_map(|member| member.kept.shown.as_ref());
        let read = own.into_iter().chain(kept).find_map(|shown| match shown {
            Shown::Read(read) if !is_submodule(&read.folder) => Some(read),
            _ => None,
        });
        let listed: Vec<Worktree> = match read {
            Some(read) => read.state.worktrees.clone(),
            None => group
                .members
                .iter()
                .filter(|member| member.superproject.is_none())
                .map(|member| Worktree {
                    folder: member.folder.clone(),
                    commit: None,
                    branch: None,
                    is_main: false,
                    is_current: false,
                    is_missing: false,
                    uncommitted: None,
                })
                .collect(),
        };
        listed
            .into_iter()
            .filter(|worktree| !worktree.is_missing && !same_folder(&worktree.folder, &self.tab.folder))
            .collect()
    }
}

/// The name of a tab: its repository's, and the name of the worktree or the submodule it shows after ` › `.
pub(super) fn tab_name(name: &str, shown_label: Option<&str>) -> String {
    match shown_label {
        Some(label) => format!("{name}{}{label}", words::IN_TAB),
        None => name.to_owned(),
    }
}

/// A worktree in the **▾**: its branch, or `detached at <short>`.
fn worktree_entry(worktree: &Worktree) -> String {
    match (&worktree.branch, &worktree.commit) {
        (Some(branch), _) => short_branch(branch).to_owned(),
        (None, Some(commit)) => words::detached_at(commit.short()),
        (None, None) => folder_name(&worktree.folder),
    }
}

/// A branch without `refs/heads/`.
fn short_branch(full: &str) -> &str {
    full.strip_prefix("refs/heads/").unwrap_or(full)
}

/// The path of a submodule from the repository of its tab, with forward slashes.
fn path_from(repository: &Path, submodule: &Path) -> String {
    submodule.strip_prefix(repository).map_or_else(
        |_| folder_name(submodule),
        |path| {
            path.iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        },
    )
}

/// How many submodules a submodule is inside, in its group: none for a submodule of the repository itself.
fn depth(group: &TabGroup, folder: &Path) -> usize {
    let superproject_of = |folder: &Path| {
        group
            .members
            .iter()
            .find(|member| same_folder(&member.folder, folder))
            .and_then(|member| member.superproject.clone())
    };
    let mut depth = 0;
    let mut current = superproject_of(folder);
    while let Some(superproject) = current {
        current = superproject_of(&superproject).filter(|_| depth < 8);
        if current.is_some() {
            depth += 1;
        }
    }
    depth
}

/// The last name of a folder.
fn folder_name(folder: &Path) -> String {
    folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
