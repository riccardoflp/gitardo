//! The words of the header (`graph.toolbar`, `activity.status-bar`, `graph.more-menu`).

/// The pill of the repository when the window shows no folder.
pub const NO_FOLDER: &str = "No folder open";
/// The tooltip of the pill of the repository.
pub const REPOSITORY_TOOLTIP: &str = "Choose the repository shown, or open a folder";
/// The entry of the repository menu that opens a folder (`repo.open-folder`).
pub const OPEN_FOLDER: &str = "Open folder\u{2026}";
/// The submenu of the recent workspaces in the repository menu (`repo.open-recent`).
pub const OPEN_RECENT: &str = "Open recent";
/// Said after the repository shown in the picker of the repositories (`repo.select`).
pub const CURRENT: &str = "current";
/// The branch of the header before the first commit (`activity.status-bar`).
pub const NO_COMMITS_YET: &str = "no commits yet";
/// What the empty Quick search says.
pub const QUICK_SEARCH: &str = "Commits, branches, files, commands";
/// The keys of Quick search, shown in its box.
pub const QUICK_SEARCH_KEYS: &str = "Ctrl K";
/// The words of Sync.
pub const SYNC: &str = "Sync";
/// The button of the Activity drawer, as the screen reader and its tooltip say it.
pub const ACTIVITY: &str = "Activity";
/// The **…** button at the end of the header, as the screen reader and its tooltip say it.
pub const MORE: &str = "More";
/// **Refresh** in the **…** menu: reads the repository again at once.
pub const REFRESH: &str = "Refresh";
/// **Select repository…** in the **…** menu (`repo.select`).
pub const SELECT_REPOSITORY: &str = "Select repository\u{2026}";
/// The last line of the tooltip of the activity indicator (`activity.status-bar`).
pub const OPEN_THE_DRAWER: &str = "Click to open the Activity drawer.";

/// The branch of the header when `HEAD` is detached: `detached at 3f2a1c9`.
#[must_use]
pub fn detached_at(short_hash: &str) -> String {
    format!("detached at {short_hash}")
}

/// How far the branch checked out is from its upstream: `↑2 ↓0`.
#[must_use]
pub fn upstream_distance(ahead: u32, behind: u32) -> String {
    format!("\u{2191}{ahead} \u{2193}{behind}")
}

/// The words of Sync with the commits to pull, then to push: `↓3 ↑2   Sync`.
#[must_use]
pub fn sync_counts(behind: u32, ahead: u32) -> String {
    format!("\u{2193}{behind}  \u{2191}{ahead}   {SYNC}")
}

/// What the activity indicator says: the title of what has run longest and for how long, `Reading app (2 s)`.
#[must_use]
pub fn running(title: &str, seconds: u64) -> String {
    format!("{title} ({})", super::activity::duration(seconds))
}
