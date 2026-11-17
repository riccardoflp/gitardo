//! The words of the repositories of a workspace (`repo.open-folder`, `repo.open-recent`, `repo.select`,
//! `repo.no-repository`) and of their tabs (`repo.tabs`, `repo.open-tab`, `repo.choose-tabs`, `repo.close-tab`,
//! `repo.open-code-workspace`).

/// What the timeline says in a workspace with no repository (`repo.no-repository`).
pub const NO_REPOSITORY: &str = "No Git repository in this workspace";
/// The button that looks for the repositories again.
pub const CHECK_AGAIN: &str = "Check again";
/// What the timeline says while the repositories are looked for and the first one is read.
pub const LOADING_REPOSITORY: &str = "Loading repository\u{2026}";
/// What the timeline says when the window opened with `readOnStartup` off (`repo.read-state`).
pub const NOT_READ_YET: &str = "Not read yet";
/// The button that reads the repository then.
pub const READ_THE_REPOSITORY: &str = "Read the repository";
/// The title of the system's folder dialog.
pub const CHOOSE_FOLDER: &str = "Open folder";
/// The picker of the repositories (`repo.select`).
pub const PICKER_TITLE: &str = "Repository shown in Gitardo";
/// The answer that replaces the workspace with the folder chosen.
pub const REPLACE: &str = "Replace the workspace";
/// The answer that adds the folder chosen to the workspace.
pub const ADD: &str = "Add to the workspace";
/// The answer that opens nothing.
pub const CANCEL: &str = "Cancel";
/// Why a recent workspace does not open (`repo.open-recent`).
pub const RECENT_GONE: &str = "None of the folders of this workspace is there any more.";
/// Why Select repository… does nothing in a workspace of one repository.
pub const SINGLE_REPOSITORY: &str = "Gitardo: this workspace has a single repository.";
/// Why Select repository… does nothing in a workspace of none.
pub const NO_REPOSITORY_TO_SELECT: &str = "Gitardo: no Git repository in this workspace.";
/// The rows of the repositories open, as the screen reader names them (`repo.tabs`).
pub const TABS: &str = "Repositories open";
/// The heading of the first section of the side column (`repo.side-column`).
pub const REPOSITORIES: &str = "REPOSITORIES";
/// The heading of the second section of the side column: the branch rail.
pub const BRANCHES: &str = "BRANCHES";
/// The side column, as the screen reader names it.
pub const SIDE_COLUMN: &str = "Repositories and branches";
/// A worktree whose folder is gone (`repo.worktree-summary`).
pub const MISSING: &str = "missing";
/// The tooltip of **+** in the heading of **Repositories**.
pub const OPEN_ANOTHER: &str = "Open a repository, a folder or a workspace file";
/// The entry that opens a workspace file of VS Code (`repo.open-code-workspace`).
pub const OPEN_WORKSPACE_FILE: &str = "Open workspace file\u{2026}";
/// The title of the system's dialog for a workspace file of VS Code.
pub const CHOOSE_WORKSPACE_FILE: &str = "Open workspace file";
/// The kind of files that dialog shows.
pub const WORKSPACE_FILES: &str = "VS Code workspaces";
/// Why a workspace file of VS Code does not open.
pub const NAMES_NO_FOLDERS: &str = "This file names no folders.";
/// The picker of the repositories to open in tabs (`repo.choose-tabs`).
pub const CHOOSE_TABS_TITLE: &str = "Repositories to open in tabs";
/// The picker of a repository to open in a tab (`repo.open-tab`).
pub const OPEN_TAB_TITLE: &str = "Open a repository in a tab";
/// The entry of the menu of a tab, and the tooltip of its **×**, that closes it (`repo.close-tab`).
pub const CLOSE_TAB: &str = "Close tab";
/// The entry of the menu of a tab that closes the others.
pub const CLOSE_OTHER_TABS: &str = "Close other tabs";
/// The tooltip of the **▾** of a tab, which lists its worktrees and its submodules (`repo.group-tabs`).
pub const TAB_MEMBERS: &str = "Worktrees and submodules";
/// What is put between the name of a tab and the name of the worktree or the submodule it shows.
pub const IN_TAB: &str = " \u{203a} ";

/// A worktree with no branch checked out, in the **▾** of its tab.
#[must_use]
pub fn detached_at(short: &str) -> String {
    format!("detached at {short}")
}

/// The button that puts a notice away.
pub const DISMISS: &str = "Dismiss";

/// Why a tab where an operation runs does not close (`repo.close-tab`).
#[must_use]
pub fn wait_before_closing(title: &str) -> String {
    format!("Gitardo: wait for {title} to end before closing this tab.")
}

/// Why an operation does not start while another runs, in this tab or another (`repo.switch-tab`).
#[must_use]
pub fn running_elsewhere(title: &str, repository: &str) -> String {
    format!("Gitardo: {title} is running in {repository}. Try again once it ends.")
}

/// What the spinner of a tab where an operation runs says (`repo.tabs`).
pub const OPERATION_RUNNING: &str = "An operation is running";

/// The title of the question asked when a folder is opened while the window shows a workspace.
#[must_use]
pub fn open_question(folder: &str) -> String {
    format!("Open {folder}")
}

/// How many files are changed and not committed in a repository or a worktree: `● 3` (`repo.worktree-summary`).
#[must_use]
pub fn changed_files(count: u32) -> String {
    format!("\u{25cf} {count}")
}

/// The commits to pull and to push, each only when there are some: `↓2 ↑1`, `↓2`, `↑1`, or nothing.
#[must_use]
pub fn to_pull_and_push(behind: u32, ahead: u32) -> String {
    let mut parts = Vec::new();
    if behind > 0 {
        parts.push(format!("\u{2193}{behind}"));
    }
    if ahead > 0 {
        parts.push(format!("\u{2191}{ahead}"));
    }
    parts.join(" ")
}
