//! The entries of the menus of the worktrees: the menu of a worktree in the Worktrees list, the entries the menu of a
//! branch another worktree has starts and ends with, and the **…** of the list.

use model::contract::{Answer, Answers, Target};
use model::state::Worktree;
use model::worktree::{DELETE_BRANCH, KEEP_BRANCH, REMOVE_BRANCH, WorktreeBranch, is_listed};

use crate::branch::{OperationEntry, ReferenceMenus};
use crate::text::branch as branch_words;
use crate::text::worktree as words;

/// Whether a worktree is renamed and removed from this window: not the main worktree, the one the views show, nor the
/// folder of the window.
fn can_change(menus: ReferenceMenus<'_>, worktree: &Worktree) -> bool {
    let is_window = menus
        .window
        .folder
        .is_some_and(|folder| is_listed(&[folder.to_path_buf()], &worktree.folder));
    !worktree.is_main && !worktree.is_current && !is_window && !worktree.is_missing
}

/// The context menu of a worktree in the Worktrees list: what opens it, in a new window and **Show in Gitardo** except
/// for the one the views show; **Rename…** and **Remove…** except for the main worktree, the one shown and the folder
/// of the window; **Copy path**; **Lock…** or **Unlock** except on the main worktree; and **Prune missing worktrees**
/// while one is missing and not locked. A missing worktree has only these.
pub(crate) fn worktree_entries(menus: ReferenceMenus<'_>, worktree: &Worktree) -> Vec<OperationEntry> {
    let target = || Target::Worktree(worktree.folder.clone());
    let runs = |label: &str, feature: &'static str| OperationEntry::runs(label, feature, target());
    let mut entries = Vec::new();
    if !worktree.is_missing {
        if !worktree.is_current {
            entries.push(runs(words::OPEN_IN_NEW_WINDOW, "worktree.open-in-new-window"));
        }
        if menus.window.has_editor {
            entries.push(runs(words::OPEN_IN_EDITOR, "worktree.open-in-editor"));
        }
        entries.push(runs(words::OPEN_IN_TERMINAL, "worktree.open-in-terminal"));
        entries.push(runs(words::reveal(), "worktree.reveal"));
        if !worktree.is_current {
            entries.push(runs(words::SHOW_IN_GITARDO, "worktree.show-in-gitardo"));
        }
        if can_change(menus, worktree) {
            entries.push(runs(words::RENAME, "worktree.rename"));
        }
    }
    entries.push(runs(words::COPY_PATH, "worktree.copy-path"));
    if worktree.locked.is_some() {
        entries.push(runs(words::UNLOCK, "worktree.unlock"));
    } else if !worktree.is_main {
        entries.push(runs(words::LOCK, "worktree.lock"));
    }
    if can_change(menus, worktree) {
        entries.push(runs(words::REMOVE, "worktree.remove"));
    }
    if model::worktree::has_prunable(&menus.repository.worktrees) {
        entries.push(runs(words::PRUNE, "worktree.prune"));
    }
    entries
}

/// The entries of the menu of a branch another worktree has: what opens that worktree, which come first, and
/// **Rename worktree…**, **Remove worktree…** and **Remove worktree and delete branch…**, which come last, except for
/// the main worktree and the folder of the window. A missing worktree has none.
pub(crate) fn other_branch_entries(
    menus: ReferenceMenus<'_>,
    found: WorktreeBranch<'_>,
) -> (Vec<OperationEntry>, Vec<OperationEntry>) {
    let worktree = found.worktree;
    if found.is_missing() {
        return (Vec::new(), Vec::new());
    }
    let target = || Target::Worktree(worktree.folder.clone());
    let runs = |label: &str, feature: &'static str| OperationEntry::runs(label, feature, target());
    let mut opening = vec![runs(
        branch_words::OPEN_WORKTREE_IN_NEW_WINDOW,
        "worktree.open-in-new-window",
    )];
    if menus.window.has_editor {
        opening.push(runs(words::OPEN_WORKTREE_IN_EDITOR, "worktree.open-in-editor"));
    }
    opening.push(runs(words::OPEN_TERMINAL_IN_WORKTREE, "worktree.open-in-terminal"));
    opening.push(runs(words::reveal_worktree(), "worktree.reveal"));
    opening.push(runs(words::SHOW_WORKTREE_IN_GITARDO, "worktree.show-in-gitardo"));
    let mut changing = Vec::new();
    if found.can_rename_or_remove {
        let removing = |label: &str, choice: &str| {
            let answers = Answers::new().with(REMOVE_BRANCH, Answer::Choice(choice.to_owned()));
            OperationEntry::runs_with(label, "worktree.remove", target(), answers)
        };
        changing.push(runs(words::RENAME_WORKTREE, "worktree.rename"));
        changing.push(removing(branch_words::REMOVE_WORKTREE, KEEP_BRANCH));
        changing.push(removing(branch_words::REMOVE_WORKTREE_AND_DELETE_BRANCH, DELETE_BRANCH));
    }
    (opening, changing)
}

/// The **…** of the Worktrees list: **Create worktree…**, **Switch worktree…**, and **Prune missing worktrees** while a
/// worktree is missing and not locked.
pub(crate) fn list_entries(menus: ReferenceMenus<'_>) -> Vec<OperationEntry> {
    let mut entries = vec![
        OperationEntry::runs(words::CREATE_WORKTREE, "worktree.create", Target::Head),
        OperationEntry::runs(words::SWITCH, "worktree.switch", Target::Head),
        OperationEntry::runs(
            crate::text::worktree_setup::MENU_ENTRY,
            "worktree-setup.page",
            Target::Head,
        ),
    ];
    if model::worktree::has_prunable(&menus.repository.worktrees) {
        entries.push(OperationEntry::runs(words::PRUNE, "worktree.prune", Target::Head));
    }
    entries
}
