//! The words of what runs: the activity indicator of the header and the Activity drawer (`activity.status-bar`,
//! `activity.what-is-running`, `activity.describe-task`).

/// The title of the Activity drawer.
pub const DRAWER: &str = "Activity";
/// The button that closes the Activity drawer.
pub const CLOSE_DRAWER: &str = "Close the Activity drawer";
/// What the drawer says with nothing running.
pub const NOTHING_RUNNING: &str = "Nothing is running now";
/// What the drawer says under it.
pub const NOTHING_RUNNING_DETAIL: &str = "Gitardo is not reading or changing anything";

/// The title of the column of what runs.
pub const RUNNING_COLUMN: &str = "WHAT IS RUNNING";
/// The title of the column of the Command log.
pub const LOG_COLUMN: &str = "COMMAND LOG";
/// The button under what runs that shows the Command log (`activity.what-is-running`).
pub const SHOW_COMMANDS: &str = "Show the commands Gitardo runs";
/// The button that puts the Command log away again.
pub const HIDE_COMMANDS: &str = "Hide the commands";
/// The Command log, as the screen reader says its list (`log.show`).
pub const COMMAND_LOG: &str = "Command log";
/// What the Command log says before anything was logged.
pub const LOG_EMPTY: &str = "No command has run yet";

/// What asks for the search for the repositories: a workspace opened, or looked at again.
pub const OPENING_WORKSPACE: &str = "Opening the workspace";
/// What asks for a read of a repository: it was chosen, or is read again.
pub const SHOWING_REPOSITORY: &str = "Showing the repository";
/// What asks for the next pages of the timeline: it was scrolled near the end of the commits loaded.
pub const LOADING_MORE_COMMITS: &str = "Loading more commits";
/// What asks for the first page of another history: the branches the timeline shows were changed.
pub const NARROWING_TIMELINE: &str = "Showing other branches";
/// What asks for a read of the details of a commit: a row of the timeline was selected.
pub const SHOWING_DETAILS: &str = "Showing the details of a commit";
/// What asks for a read of the commit of a page and its lines: the page was opened.
pub const OPENING_COMMIT_PAGE: &str = "Opening the page of a commit";

/// The title of a read of a repository: `Reading app`.
#[must_use]
pub fn reading(name: &str) -> String {
    format!("Reading {name}")
}

/// The title of the search for the repositories of a workspace.
#[must_use]
pub fn finding_repositories(folders: &str) -> String {
    format!("Looking for repositories in {folders}")
}

/// What the Command log says once older lines were dropped: `12 older lines were dropped`.
#[must_use]
pub fn dropped_lines(count: usize) -> String {
    if count == 1 {
        String::from("1 older line was dropped")
    } else {
        format!("{count} older lines were dropped")
    }
}

/// What ran a moment ago, in the drawer: `Took 2 s`.
#[must_use]
pub fn took(seconds: u64) -> String {
    format!("Took {}", duration(seconds))
}

/// How long something has run, in whole seconds, `12 s`, and from a minute on `1 min 5 s`.
#[must_use]
pub fn duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::duration;

    #[test]
    fn says_seconds_then_minutes_and_seconds() {
        assert_eq!(duration(59), "59 s");
        assert_eq!(duration(65), "1 min 5 s");
        assert_eq!(duration(0), "0 s");
    }
}
