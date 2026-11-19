//! The icons of the items of the menus (`ui.menu-icons`): an item that does something shows the icon of its action at
//! the left of its words, the one a button of the same action draws, so that one meaning is one icon across the window;
//! an item that chooses or switches shows the check while it is chosen, and no other icon. The views make their items
//! here, never with `MenuItem::new`, so that an item without an icon is one this module was asked for by name.

use ui::{Icon, MenuItem};

/// The icon of an item of a menu that runs an operation of the catalog, by the identifier of its card, the one a button
/// of the same operation draws; none for the operations no icon of Lucide says, which rewrite the history (rebase,
/// reset, squash, split, fixup) or have no picture of their own (apply a patch, switch identity, synchronize the URLs of
/// a submodule).
///
/// Feature: ui.menu-icons
#[must_use]
pub(crate) fn operation_icon(feature: &str) -> Option<Icon> {
    history_icon(feature).or_else(|| place_icon(feature))
}

/// The icon of an operation on the history, its branches, tags, stashes and remotes.
fn history_icon(feature: &str) -> Option<Icon> {
    let icon = match feature {
        "branch.checkout"
        | "branch.checkout-remote"
        | "branch.checkout-remote-and-rebase"
        | "branch.checkout-detached"
        | "tag.checkout"
        | "submodule.update-to-recorded" => super::CHECK_OUT,
        "branch.create" | "branch.create-orphan" | "stash.branch" => super::CREATE_BRANCH,
        "tag.create" => super::TAG,
        "branch.rename" | "worktree.rename" | "message.edit" => super::RENAME,
        "branch.delete"
        | "branch.clean-up"
        | "tag.delete"
        | "tag.delete-remote"
        | "remote.delete-branch"
        | "stash.drop" => super::DELETE,
        "sync.sync" => super::SYNC,
        "sync.fetch" => super::FETCH,
        "sync.pull" | "sync.pull-with" | "branch.fast-forward" | "submodule.update-to-latest" => super::PULL,
        "sync.push" | "sync.push-force-with-lease" | "sync.push-force" | "tag.push" => super::PUSH,
        "merge.into-current" | "merge.choose-way" => super::MERGE,
        "cherry-pick.commit" | "selection.cherry-pick" => super::CHERRY_PICK,
        "revert.commit" | "undo.last" | "changes.discard" | "changes.discard-all" | "settings.reset" => super::UNDO,
        "commits.copy-hash" | "commits.copy-message" | "patch.copy" => super::COPY,
        "patch.save" => super::SAVE,
        "compare.with"
        | "compare.with-current"
        | "compare.selected-before"
        | "compare.choose"
        | "compare.path-with"
        | "details.compare-file-with"
        | "range-diff.compare" => super::COMPARE,
        "bisect.start" | "bisect.mark" => super::BISECT,
        "changes.stage" | "changes.stage-all" | "conflicts.mark-resolved" => super::ADD,
        "changes.unstage" | "changes.unstage-all" => super::UNSTAGE,
        "stash.push" | "stash.push-staged" | "stash.push-file" => super::STASH,
        "stash.apply" => super::APPLY,
        "stash.pop" => super::POP,
        "github.open-pull-request" | "github.load-pull-requests" => super::PULL_REQUEST,
        "reflog.browse" | "file-history.file" | "file-history.folder" | "file-history.lines" => super::HISTORY,
        "remote.manage" | "branch.set-upstream" => super::REMOTE,
        "blame.toggle-file" | "blame.previous" => super::BLAME,
        "blame.open-line-commit" => super::OPEN_AS_PAGE,
        "blame.reveal-line-commit" => super::TIMELINE,
        _ => return None,
    };
    Some(icon)
}

/// The icon of an operation on a worktree or a submodule, as a place on the disk: opened, revealed, copied, removed.
fn place_icon(feature: &str) -> Option<Icon> {
    let icon = match feature {
        "worktree.create"
        | "worktree.create-from-branch"
        | "worktree.create-at-commit"
        | "worktree.open-in-worktree"
        | "worktree.switch" => super::WORKTREE,
        "worktree-setup.page" => super::SETTINGS,
        "worktree.open-in-new-window" | "submodule.open-in-new-window" => super::NEW_WINDOW,
        "worktree.open-in-editor" => super::OPEN_EXTERNAL,
        "worktree.open-in-terminal" | "submodule.open-in-terminal" => super::TERMINAL,
        "worktree.reveal" | "submodule.reveal" => super::REVEAL,
        "worktree.show-in-gitardo" | "submodule.show-in-gitardo" => super::SHOW,
        "worktree.copy-path" | "submodule.copy-path" | "submodule.copy-url" => super::COPY,
        "worktree.remove" | "worktree.prune" | "submodule.remove" => super::DELETE,
        "submodule.initialize" => super::SUBMODULE,
        "submodule.deinitialize" => super::NOT_INITIALIZED,
        "submodule.add" | "submodule.stage-recorded" => super::ADD,
        _ => return None,
    };
    Some(icon)
}

/// An item that does something, with the icon of its action.
#[must_use]
pub(crate) fn action<T>(icon: Icon, label: &str, value: T) -> MenuItem<'_, T> {
    MenuItem::new(label, value).with_icon(icon)
}

/// An item that runs an operation of the catalog, with the icon [`operation_icon`] gives it, or none.
#[must_use]
pub(crate) fn operation_item<'text, T>(label: &'text str, feature: &str, value: T) -> MenuItem<'text, T> {
    let item = MenuItem::new(label, value);
    match operation_icon(feature) {
        Some(icon) => item.with_icon(icon),
        None => item,
    }
}

/// An item that chooses a value or switches something on and off: the check while it is chosen or on, in the place
/// of the icons, and no icon of its own.
#[must_use]
pub(crate) fn choice<T>(label: &str, value: T, is_chosen: bool) -> MenuItem<'_, T> {
    let item = MenuItem::new(label, value);
    if is_chosen { item.with_icon(super::CHOSEN) } else { item }
}

/// An item that no icon says, its words lined up with those of the items that have one. Its words are on the list of
/// the items without an icon that the tests of this module keep.
#[must_use]
pub(crate) fn without_icon<T>(label: &str, value: T) -> MenuItem<'_, T> {
    MenuItem::new(label, value)
}
