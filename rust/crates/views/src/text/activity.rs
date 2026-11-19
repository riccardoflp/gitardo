//! The words of what runs: the activity indicator of the status bar and the Activity tab of the bottom panel
//! (`activity.status-bar`, `activity.what-is-running`, `activity.describe-task`).

/// The Activity tab, as the screen reader says what it holds; the tab itself is named by the words of the bottom
/// panel.
pub const ACTIVITY: &str = "What is running";
/// What the Activity tab says with nothing running.
pub const NOTHING_RUNNING: &str = "Nothing is running now";
/// What the Activity tab says under it.
pub const NOTHING_RUNNING_DETAIL: &str = "Gitardo is not reading or changing anything";

/// The title of the column of what runs.
pub const RUNNING_COLUMN: &str = "WHAT IS RUNNING";
/// The heading of what ran, under what runs.
pub const RAN_HEADING: &str = "RAN";
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
/// Why the diff of a file is read: a Diff page shows it.
pub const COMPUTING_DIFF: &str = "Showing the diff of a file";
/// Why the regions of a file in conflict are computed.
pub const SHOWING_MERGE: &str = "Showing a file in conflict";
/// Why the File page reads.
pub const OPENING_FILE: &str = "Opening a file";
/// Why a diagram of Mermaid of the preview of a Markdown file is drawn (`file-view.mermaid`).
pub const DRAWING_DIAGRAM: &str = "Drawing a diagram";
/// Why the Files page reads.
pub const LISTING_FILES: &str = "Listing the files";
/// Why the Search page reads.
pub const SEARCHING_FILES: &str = "Searching the files";
/// Why the page of new worktrees reads.
pub const LOOKING_AT_IGNORED: &str = "Looking at what git ignores";
/// What the Large files page reads (`lfs.page`).
pub const READING_LARGE_FILES: &str = "Reading the files stored with Git LFS";
/// Why git is asked which files it ignores: the files not saved (`changes.unsaved`), and the files that changed
/// (`repo.watch-changes`).
pub const CHECKING_IGNORED: &str = "Asking git which files it ignores";
/// Why the File page reads the blame of its file (`blame.file`, `blame.current-line`).
pub const READING_BLAME: &str = "Reading the blame";
/// Why a comparison is read: its page, or the inspector, shows it.
pub const COMPARING: &str = "Comparing two sides";
/// Why the files in conflict are read: to count the conflicts the rules resolve (`conflicts.banner`).
pub const COUNTING_CONFLICTS: &str = "Counting the conflicts";
/// Why the files in conflict are read for their review (`review.page`).
pub const REVIEWING_CONFLICTS: &str = "Reviewing the conflicts";

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

/// What the Activity tab says of what ran: the time it ended and how long it took, `14:02:09 · took 2 s`, and
/// `took less than 1 s` for what was over sooner.
#[must_use]
pub fn ran(ended: i64, took: std::time::Duration) -> String {
    let took = match took.as_secs() {
        0 => String::from("less than 1 s"),
        seconds => duration(seconds),
    };
    format!("{} \u{b7} took {took}", time_of_day(ended))
}

/// The time of day of a moment, `14:02:09`, in UTC as the timeline says dates (open question Q58).
#[must_use]
pub fn time_of_day(seconds: i64) -> String {
    let of_day = seconds.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", of_day / 3_600, of_day % 3_600 / 60, of_day % 60)
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
    use std::time::Duration;

    use super::{duration, ran};

    #[test]
    fn says_seconds_then_minutes_and_seconds() {
        assert_eq!(duration(59), "59 s");
        assert_eq!(duration(65), "1 min 5 s");
        assert_eq!(duration(0), "0 s");
    }

    #[test]
    fn says_when_something_ended_and_how_long_it_took() {
        let ended = 14 * 3_600 + 2 * 60 + 9;
        assert_eq!(ran(ended, Duration::from_millis(2_900)), "14:02:09 \u{b7} took 2 s");
        assert_eq!(
            ran(ended + 86_400, Duration::from_millis(700)),
            "14:02:09 \u{b7} took less than 1 s"
        );
    }
}
