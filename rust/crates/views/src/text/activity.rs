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
/// The button under what runs that shows the bottom panel at the Command log (`activity.what-is-running`).
pub const SHOW_COMMANDS: &str = "Show the commands Gitardo runs";
/// The Command log, as the screen reader says its list (`log.show`).
pub const COMMAND_LOG: &str = "Command log";
/// What the Command log says before anything was logged.
pub const LOG_EMPTY: &str = "No command has run yet";

/// The title of the column of the tasks, the commands of new worktrees (`worktree-setup.command-steps`).
pub const TASKS_COLUMN: &str = "TASKS";
/// The tasks, as the screen reader says their list.
pub const TASKS: &str = "Tasks";
/// What the output of a task says before it printed anything.
pub const NOTHING_PRINTED: &str = "Nothing printed yet";
/// The button that stops the task running, and those after it.
pub const STOP_TASK: &str = "Stop";
/// The tooltip of **Stop**.
pub const STOP_TASK_TOOLTIP: &str = "Stop this command; the commands after it do not run";
/// What a task waiting for those before it says.
pub const TASK_WAITS: &str = "waits";
/// What a task that did not run says.
pub const TASK_DID_NOT_RUN: &str = "did not run";
/// What a task stopped says.
pub const TASK_STOPPED: &str = "stopped";
/// What a task whose shell could not start says.
pub const TASK_NOT_STARTED: &str = "could not start";
/// Why a task runs, as the activity indicator says it.
pub const RUNNING_IN_TASKS: &str = "A command of a new worktree, in Tasks";

/// What a task that exited with a code other than 0 says: `exit code 1`.
#[must_use]
pub fn exit_code(code: i32) -> String {
    format!("exit code {code}")
}

/// What the box under the output of the task running says while it is empty: `Answer npm ci, then press Enter`.
#[must_use]
pub fn answer_task(command: &str) -> String {
    format!("Answer {command}, then press Enter")
}

/// The title of a task running, as the activity indicator says it: `Running npm ci in fix`.
#[must_use]
pub fn running_task(command: &str, name: &str) -> String {
    format!("Running {command} in {name}")
}

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
/// Why the Files page reads.
pub const LISTING_FILES: &str = "Listing the files";
/// Why the page of new worktrees reads.
pub const LOOKING_AT_IGNORED: &str = "Looking at what git ignores";
/// Why git is asked which files not saved it ignores (`changes.unsaved`).
pub const CHECKING_UNSAVED: &str = "Checking the files not saved";
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
