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

/// What asks for the search for the repositories: a workspace opened, or looked at again.
pub const OPENING_WORKSPACE: &str = "Opening the workspace";
/// What asks for a read of a repository: it was chosen, or is read again.
pub const SHOWING_REPOSITORY: &str = "Showing the repository";
/// What asks for the next pages of the timeline: it was scrolled near the end of the commits loaded.
pub const LOADING_MORE_COMMITS: &str = "Loading more commits";
/// What asks for the first page of another history: the branches the timeline shows were changed.
pub const NARROWING_TIMELINE: &str = "Showing other branches";

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
