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
/// What **Sync** runs, in its tooltip.
pub const SYNC_TOOLTIP: &str = "Sync: git fetch, git pull, git push";
/// The button of the menu of **Sync**, as the screen reader and its tooltip say it.
pub const SYNC_MENU: &str = "More ways to sync";
/// **Fetch** in the menu of **Sync**.
pub const FETCH: &str = "Fetch";
/// What **Fetch** runs.
pub const FETCH_TOOLTIP: &str = "Fetch: git fetch --all --prune";
/// **Pull** in the menu of **Sync**.
pub const PULL: &str = "Pull";
/// **Pull (no options)** among the ways to pull (`sync.pull-with`).
pub const PULL_PLAIN: &str = "Pull (no options)";
/// **Pull (rebase)**.
pub const PULL_REBASE: &str = "Pull (rebase)";
/// **Pull (autostash)**.
pub const PULL_AUTOSTASH: &str = "Pull (autostash)";
/// **Pull (rebase, autostash)**.
pub const PULL_REBASE_AUTOSTASH: &str = "Pull (rebase, autostash)";
/// Beside the way to pull that **Pull** takes.
pub const DEFAULT_WAY: &str = "default";
/// **Push** in the menu of **Sync**.
pub const PUSH: &str = "Push";
/// What **Push** runs.
pub const PUSH_TOOLTIP: &str = "Push: git push";
/// **Push (force with lease)** in the menu of **Sync** (`sync.push-force-with-lease`).
pub const PUSH_FORCE_WITH_LEASE: &str = "Push (force with lease)";
/// What **Push (force with lease)** runs.
pub const PUSH_FORCE_WITH_LEASE_TOOLTIP: &str = "git push --force-with-lease";
/// **Push (force)** in the menu of **Sync** (`sync.push-force`).
pub const PUSH_FORCE: &str = "Push (force)";
/// What **Push (force)** runs.
pub const PUSH_FORCE_TOOLTIP: &str = "git push --force";
/// The button of the Activity drawer, as the screen reader and its tooltip say it.
pub const ACTIVITY: &str = "Activity";
/// The **…** button at the end of the header, as the screen reader and its tooltip say it.
pub const MORE: &str = "More";
/// **Refresh** in the **…** menu: reads the repository again at once.
pub const REFRESH: &str = "Refresh";
/// **Branch** in **…**: create a branch at `HEAD`.
pub const BRANCH: &str = "Branch";
/// **Stash** in **…**: stash all changes.
pub const STASH: &str = "Stash";
/// **Manage remotes…** in **…** (`remote.manage`).
pub const MANAGE_REMOTES: &str = "Manage remotes\u{2026}";
/// **History of the active file** in **…**: the history of the file of the File page in front (`file-history.file`).
pub const ACTIVE_FILE_HISTORY: &str = "History of the active file";
/// **Undo** in **…**: take back the last operation.
pub const UNDO: &str = "Undo";
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

/// An entry of the menu of **Sync** with the commits it would bring or send, when there are some: `Pull  ↓3`.
#[must_use]
pub fn with_count(label: &str, arrow: char, count: u32) -> String {
    if count == 0 {
        label.to_owned()
    } else {
        format!("{label}  {arrow}{count}")
    }
}

/// The words of Sync with the commits to pull, then to push: `↓3 ↑2   Sync`.
#[must_use]
pub fn sync_counts(behind: u32, ahead: u32) -> String {
    format!("{}   {SYNC}", counts_only(behind, ahead))
}

/// The commits Sync would bring and send, without its word, as a narrow window shows them beside its icon: `↓3 ↑2`.
#[must_use]
pub fn counts_only(behind: u32, ahead: u32) -> String {
    format!("\u{2193}{behind}  \u{2191}{ahead}")
}

/// What the activity indicator says: the title of what has run longest and for how long, `Reading app (2 s)`.
#[must_use]
pub fn running(title: &str, seconds: u64) -> String {
    format!("{title} ({})", super::activity::duration(seconds))
}
/// **Show stashes** in **…** (`graph.stashes`).
pub const SHOW_STASHES: &str = "Show stashes";
/// **Show only the first parent of merges** in **…** (`graph.first-parent`).
pub const FIRST_PARENT: &str = "Show only the first parent of merges";
/// **Bring out the history of the selected commit** in **…** (`graph.trace-selected`).
pub const TRACE_SELECTED: &str = "Bring out the history of the selected commit";
/// **Fold all merges** in **…** (`graph.fold-all`).
pub const FOLD_ALL: &str = "Fold all merges";
/// **Unfold all merges** in **…** (`graph.fold-all`).
pub const UNFOLD_ALL: &str = "Unfold all merges";
/// **Show the author** in **…** (`graph.columns`).
pub const SHOW_AUTHOR: &str = "Show the author";
/// **Show the date** in **…** (`graph.columns`).
pub const SHOW_DATE: &str = "Show the date";
/// **Show the hash** in **…** (`graph.columns`).
pub const SHOW_HASH: &str = "Show the hash";
/// **Separate the days, weeks and months** in **…** (`graph.date-separators`).
pub const DATE_SEPARATORS: &str = "Separate the days, weeks and months";
/// **Put a branch and its upstream on one label** in **…** (`graph.combine-upstream`).
pub const COMBINE_UPSTREAM: &str = "Put a branch and its upstream on one label";
/// The tooltip of the first button of the header while the branch rail is shown (`graph.toggle-references`).
pub const HIDE_REFERENCES: &str = "Hide the branches, tags and stashes beside the commits";
/// Its tooltip while the branch rail is put away.
pub const SHOW_REFERENCES: &str = "Show the branches, tags and stashes beside the commits";
/// The button of Quick search in the header (`graph.search-button`).
pub const SEARCH: &str = "Search";
/// The tooltip of the button of Quick search when it shows its icon alone.
pub const SEARCH_TOOLTIP: &str = "Search (Ctrl+K)";
