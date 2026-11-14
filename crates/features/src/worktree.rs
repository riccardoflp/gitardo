//! Worktrees: creating them, opening them where the settings say, showing one in this window, renaming, removing and
//! pruning them, and their paths and commits (`docs/features/12-worktrees.md`).

mod add;
mod copy_path;
mod create;
mod create_at_commit;
mod create_from_branch;
mod names;
mod open;
mod open_in_editor;
mod open_in_new_window;
mod open_in_terminal;
mod open_in_worktree;
mod opening;
mod prune;
mod remove;
mod rename;
mod reveal;
mod setup;
mod show_in_gitardo;
mod show_in_graph;
mod switch;
#[cfg(test)]
mod testing;

pub use copy_path::copy_path;
pub use create::{NAME, START, WAY, create};
pub use create_at_commit::{BASE, create_at_commit};
pub use create_from_branch::{FOLDER, create_from_branch};
pub use open::open;
pub use open_in_editor::open_in_editor;
pub use open_in_new_window::open_in_new_window;
pub use open_in_terminal::open_in_terminal;
pub use open_in_worktree::open_in_worktree;
pub use prune::prune;
pub use remove::{BRANCH, DELETE_BRANCH, FORCE, KEEP_BRANCH, remove};
pub use rename::{BRANCH_NAME, KEEP, RENAME_BRANCH, rename};
pub use reveal::reveal;
pub use setup::{NewWorktree, Setup, setup_steps};
pub use show_in_gitardo::show_in_gitardo;
pub use show_in_graph::show_in_graph;
pub use switch::{CREATE, PICKED, switch};

use std::path::{Path, PathBuf};

use model::branch::{local_name, other_worktree};
use model::contract::{Context, Danger, FollowUp, Outcome, Plan, Refusal, Target, Text, Undo};
use model::state::Worktree;
use model::worktree::{worktree_label, worktrees_folder};

/// Why opening, copying or showing a worktree is not taken back by Undo.
const NOTHING_CHANGES: &str = "Nothing to take back: nothing changes.";
/// Why a worktree made is not taken back by Undo.
const REMOVE_TO_UNDO: &str = "Not taken back by Undo: Remove… on the worktree removes it.";

/// The worktree a feature was invoked on: by its folder from a list of worktrees, or the worktree that has a branch,
/// or the local branch of a remote branch, from the menu of that branch.
fn target_worktree(context: &Context) -> Result<&Worktree, Box<Outcome>> {
    let state = &context.repository;
    let found = match &context.target {
        Target::Worktree(folder) => state
            .worktrees
            .iter()
            .find(|worktree| model::worktree::is_listed(std::slice::from_ref(folder), &worktree.folder)),
        Target::Branch(name) => other_worktree(state, name),
        Target::RemoteBranch(name) => other_worktree(state, local_name(name, &state.remotes)),
        _ => None, // Nothing else names a worktree.
    };
    found.ok_or_else(|| {
        Box::new(Outcome::Refuse(Refusal::saying(
            "That worktree is no longer there: the list of worktrees was read again.",
        )))
    })
}

/// The refusal on a worktree whose folder is gone.
fn refuse_missing(worktree: &Worktree) -> Option<Outcome> {
    worktree.is_missing.then(|| {
        Outcome::Refuse(Refusal::saying(&format!(
            "The folder of this worktree is missing from `{}`: prune the worktrees to forget it.",
            worktree.folder.display()
        )))
    })
}

/// The refusal on the worktree the views show, which is open here already.
fn refuse_shown(worktree: &Worktree) -> Option<Outcome> {
    worktree
        .is_current
        .then(|| Outcome::Refuse(Refusal::saying("This worktree is the one shown in this window.")))
}

/// The folder of the main worktree, whichever worktree the views show.
fn main_worktree(context: &Context) -> &Path {
    let state = &context.repository;
    state
        .worktrees
        .iter()
        .find(|worktree| worktree.is_main)
        .map_or(state.folder.as_path(), |worktree| worktree.folder.as_path())
}

/// The folder of the worktrees, worked out from `worktrees.folder`.
pub(crate) fn folder_of_worktrees(context: &Context) -> String {
    let settings = &context.settings.worktrees;
    worktrees_folder(&settings.folder, main_worktree(context), &settings.home)
}

/// How the views name a worktree.
fn label(context: &Context, folder: &Path) -> String {
    worktree_label(&folder_of_worktrees(context), folder)
}

/// The folders of the worktrees there are.
fn taken_folders(context: &Context) -> Vec<PathBuf> {
    context
        .repository
        .worktrees
        .iter()
        .map(|worktree| worktree.folder.clone())
        .collect()
}

/// A plan with no steps whose follow-ups do it all: what opens a window, puts a path on the clipboard or selects a
/// commit.
fn no_steps(title: String, after: Vec<FollowUp>) -> Plan {
    Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_CHANGES)),
        after,
        on_failure: Vec::new(),
    }
}

/// A folder as the system writes it, for the commands of the shell: git gives it with `/` on Windows too.
fn system_path(folder: &Path) -> String {
    let path = folder.display().to_string();
    if cfg!(windows) { path.replace('/', "\\") } else { path }
}
