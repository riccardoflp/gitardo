//! The words of the repositories of a workspace (`repo.open-folder`, `repo.open-recent`, `repo.select`,
//! `repo.no-repository`).

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
/// The island of the tabs, as the screen reader names it (`repo.tabs`).
pub const TABS: &str = "Repositories open";
/// The tooltip of **+** at the end of the island of the tabs.
pub const OPEN_ANOTHER: &str = "Open a folder or a recent workspace";
/// The button that puts a notice away.
pub const DISMISS: &str = "Dismiss";

/// The title of the question asked when a folder is opened while the window shows a workspace.
#[must_use]
pub fn open_question(folder: &str) -> String {
    format!("Open {folder}")
}
