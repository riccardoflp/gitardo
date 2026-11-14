//! The words of the worktrees: the entries of their menus, how one stands, and the banner of one shown in this window
//! (`worktree.`).

/// **Open in new window** in the menu of a worktree (`worktree.open-in-new-window`).
pub const OPEN_IN_NEW_WINDOW: &str = "Open in new window";
/// **Open in external editor** in the menu of a worktree (`worktree.open-in-editor`).
pub const OPEN_IN_EDITOR: &str = "Open in external editor";
/// **Open in terminal** in the menu of a worktree (`worktree.open-in-terminal`).
pub const OPEN_IN_TERMINAL: &str = "Open in terminal";
/// **Show in Gitardo** in the menu of a worktree (`worktree.show-in-gitardo`).
pub const SHOW_IN_GITARDO: &str = "Show in Gitardo";
/// **Rename…** in the menu of a worktree (`worktree.rename`).
pub const RENAME: &str = "Rename\u{2026}";
/// **Copy path** in the menu of a worktree (`worktree.copy-path`).
pub const COPY_PATH: &str = "Copy path";
/// **Remove…** in the menu of a worktree (`worktree.remove`).
pub const REMOVE: &str = "Remove\u{2026}";
/// **Prune missing worktrees** in the menu of a worktree and of the Worktrees list (`worktree.prune`).
pub const PRUNE: &str = "Prune missing worktrees";
/// **Switch worktree…** in the menu of the Worktrees list (`worktree.switch`).
pub const SWITCH: &str = "Switch worktree\u{2026}";
/// **Open worktree in external editor** in the menu of a branch another worktree has.
pub const OPEN_WORKTREE_IN_EDITOR: &str = "Open worktree in external editor";
/// **Open terminal in worktree** in the menu of a branch another worktree has.
pub const OPEN_TERMINAL_IN_WORKTREE: &str = "Open terminal in worktree";
/// **Show worktree in Gitardo** in the menu of a branch another worktree has.
pub const SHOW_WORKTREE_IN_GITARDO: &str = "Show worktree in Gitardo";
/// **Rename worktree…** in the menu of a branch another worktree has.
pub const RENAME_WORKTREE: &str = "Rename worktree\u{2026}";
/// **Create worktree…** in the menu of a local branch (`worktree.create-from-branch`).
pub const CREATE_WORKTREE: &str = "Create worktree\u{2026}";
/// **Create worktree here…** in the menu of a commit (`worktree.create-at-commit`).
pub const CREATE_WORKTREE_HERE: &str = "Create worktree here\u{2026}";
/// The mark of the worktree the views show, in its tooltip.
pub const CURRENT: &str = "Shown in this window";

/// **Reveal in File Explorer**, **Reveal in Finder** or **Open containing folder**, by the system, in the menu of a
/// worktree (`worktree.reveal`).
#[must_use]
pub fn reveal() -> &'static str {
    if cfg!(windows) {
        "Reveal in File Explorer"
    } else if cfg!(target_os = "macos") {
        "Reveal in Finder"
    } else {
        "Open containing folder"
    }
}

/// The same, in the menu of a branch another worktree has.
#[must_use]
pub fn reveal_worktree() -> &'static str {
    if cfg!(windows) {
        "Reveal worktree in File Explorer"
    } else if cfg!(target_os = "macos") {
        "Reveal worktree in Finder"
    } else {
        "Open folder of worktree"
    }
}

/// The banner of a worktree or a submodule shown in this window: **Showing the worktree feature/login on
/// feature/login**, **Showing the submodule lib on main**.
#[must_use]
pub fn showing(label: &str, branch: Option<&str>, is_submodule: bool) -> String {
    let kind = if is_submodule { "submodule" } else { "worktree" };
    match branch {
        Some(branch) => format!("Showing the {kind} {label} on {branch}"),
        None => format!("Showing the {kind} {label}"),
    }
}

/// The button of the banner that shows the window's folder again: **Back to app**.
#[must_use]
pub fn back_to(name: &str) -> String {
    format!("Back to {name}")
}

/// The line of the tooltip of a worktree with files not committed.
#[must_use]
pub fn not_committed(files: usize) -> String {
    if files == 1 {
        String::from("1 file not committed")
    } else {
        format!("{files} files not committed")
    }
}
