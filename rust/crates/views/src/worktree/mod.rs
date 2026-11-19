//! The worktrees in the window (`docs/features/12-worktrees.md`): the entries of their menus and of the menus of the
//! branches they have, how a worktree stands in the Worktrees list, and the banner of a worktree shown in this window.

mod banner;
mod menu;
#[cfg(test)]
mod tests;

use std::path::Path;

use model::state::{RepositoryState, Worktree};
use model::worktree::{worktree_label, worktrees_folder};

use crate::text::header;
use crate::text::rail as rail_words;
use crate::text::worktree as words;

pub use banner::{BannerState, banner};
pub(crate) use menu::{list_entries, other_branch_entries, worktree_entries};

/// What the menus and the list of the worktrees are told of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorktreeWindow<'a> {
    /// The folder of the window while the views show another worktree, which is then neither renamed nor removed from
    /// here; none while they show the window's own.
    pub folder: Option<&'a Path>,
    /// Whether `editor.external` is set, which offers **Open in external editor**.
    pub has_editor: bool,
    /// `worktrees.folder`, which the labels of the worktrees are worked out from.
    pub setting: &'a str,
}

/// How the views name a worktree: its path from the folder of the worktrees, or the name of its folder
/// (`worktree.folder-path`). A setting that starts with `~` names the folder in the home folder, which the views do not
/// know: the worktrees are then named by their folders.
#[must_use]
pub fn label(repository: &RepositoryState, window: WorktreeWindow<'_>, worktree: &Worktree) -> String {
    let main = repository
        .worktrees
        .iter()
        .find(|worktree| worktree.is_main)
        .map_or(repository.folder.as_path(), |main| main.folder.as_path());
    let trees = worktrees_folder(window.setting, main, Path::new(""));
    worktree_label(&trees, &worktree.folder)
}

/// How a worktree stands, after its label in the Worktrees list: its branch or `detached`, then `↑n ↓m` from the
/// branch's upstream when it has one; `missing` in place of all that when its folder is gone, `missing, locked` when it
/// is locked too (`worktree.locked`).
#[must_use]
pub fn standing(repository: &RepositoryState, worktree: &Worktree) -> String {
    if let Some(standing) = lock_rules(worktree).standing {
        return standing.to_owned();
    }
    if worktree.is_missing {
        return rail_words::MISSING.to_owned();
    }
    let Some(name) = branch_of(worktree) else {
        return rail_words::DETACHED.to_owned();
    };
    let upstream = repository
        .branches
        .iter()
        .find(|branch| branch.name == name)
        .and_then(|branch| branch.upstream.as_ref());
    match upstream {
        Some(upstream) if upstream.is_gone => format!("{name} {}", rail_words::GONE),
        Some(upstream) => format!("{name} {}", header::upstream_distance(upstream.ahead, upstream.behind)),
        None => name.to_owned(),
    }
}

/// The tooltip of a worktree in the Worktrees list: its folder, `<branch> → <upstream>`, how many files are not
/// committed, whether it is the one shown in this window, and its lock with the reason (`worktree.locked`).
#[must_use]
pub fn tooltip(repository: &RepositoryState, worktree: &Worktree) -> String {
    let mut lines = vec![worktree.folder.display().to_string()];
    lines.extend(lock_rules(worktree).tooltip);
    if let Some(name) = branch_of(worktree) {
        let upstream = repository
            .branches
            .iter()
            .find(|branch| branch.name == name)
            .and_then(|branch| branch.upstream.as_ref())
            .map(|upstream| {
                upstream
                    .reference
                    .strip_prefix("refs/remotes/")
                    .unwrap_or(&upstream.reference)
            });
        lines.push(rail_words::upstream_of(name, upstream));
    }
    if let Some(files) = worktree.uncommitted.filter(|files| *files > 0) {
        lines.push(words::not_committed(files));
    }
    if worktree.is_current {
        lines.push(words::CURRENT.to_owned());
    }
    lines.join("\n")
}

/// What a double click on a worktree of the Worktrees list asks: opening it as `worktree.open` does, except on the one
/// the views show and on a missing one, where it does nothing.
#[must_use]
pub fn open_intent(worktree: &Worktree) -> Option<crate::intent::Intent> {
    (!worktree.is_current && !worktree.is_missing).then(|| {
        let target = model::contract::Target::Worktree(worktree.folder.clone());
        let open = crate::feature::FeatureIntent::Run(model::contract::FeatureId("worktree.open"), target);
        crate::intent::Intent::Feature(open)
    })
}

/// What a lock changes for a worktree, named by its folder: the views need only its tooltip and its standing.
#[must_use]
pub fn lock_rules(worktree: &Worktree) -> model::worktree::LockedRules {
    model::worktree::locked(&model::worktree::LockedInput {
        label: "",
        reason: worktree.locked.as_deref(),
        is_missing: worktree.is_missing,
    })
}

/// The branch a worktree has checked out, by its short name.
fn branch_of(worktree: &Worktree) -> Option<&str> {
    worktree
        .branch
        .as_deref()
        .map(|full| full.strip_prefix("refs/heads/").unwrap_or(full))
}
