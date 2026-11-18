//! The words of the Tasks tab of the bottom panel and of **Run Task…** (`tasks.view`, `tasks.quick-run`).

/// The tab of the tasks of the repository.
pub const TASKS: &str = "Tasks";
/// The list of the tasks, for a screen reader.
pub const LIST: &str = "Tasks of the repository";
/// What the empty search field shows.
pub const SEARCH: &str = "Search tasks";
/// The menu at the right of the search field.
pub const MORE: &str = "More of the tasks";
/// Opens `.vscode/tasks.json` in a File page.
pub const OPEN_TASKS_FILE: &str = "Open tasks.json";
/// Folds every heading of the list.
pub const COLLAPSE_ALL: &str = "Collapse all";
/// Unfolds every heading of the list.
pub const EXPAND_ALL: &str = "Expand all";
/// The heading of the groups the user made.
pub const GROUPS: &str = "Groups";
/// The heading of the tasks of `.vscode/tasks.json`.
pub const WORKSPACE: &str = "Workspace";
/// The heading of the scripts of the `package.json` files.
pub const NPM: &str = "npm";
/// The heading of the commands of new worktrees (`worktree-setup.command-steps`).
pub const NEW_WORKTREES: &str = "New worktrees";
/// Runs a task.
pub const RUN: &str = "Run";
/// Stops a task that runs.
pub const STOP: &str = "Stop";
/// Stops a task and runs it again.
pub const RESTART: &str = "Restart";
/// Opens the menu of the groups a task can be added to.
pub const ADD_TO_GROUP: &str = "Add to group";
/// Opens `.vscode/tasks.json` at the line of the label of a task.
pub const OPEN_IN_TASKS_FILE: &str = "Open in tasks.json";
/// Copies the command of a task.
pub const COPY_COMMAND: &str = "Copy command";
/// What a row says of a task that was stopped.
pub const STOPPED: &str = "stopped";
/// What a row says of a task that waits for those it depends on.
pub const WAITS: &str = "waits";
/// What a row says of a task that did not run because one it waited for did not succeed.
pub const DID_NOT_RUN: &str = "did not run";
/// What a row says of a task that could not be started.
pub const NOT_STARTED: &str = "not started";
/// What a row says of a background task that is ready for those that wait for it.
pub const READY: &str = "ready";
/// What the tab says in a folder not trusted.
pub const NOT_TRUSTED: &str = "Tasks run commands of the repository: trust this folder to run them.";
/// The button that asks to trust the folder.
pub const TRUST: &str = "Trust this folder";
/// What the tab says while no repository is shown.
pub const NO_REPOSITORY: &str = "Open a repository to list its tasks";
/// What the tab says while the tasks are read for the first time.
pub const READING: &str = "Reading the tasks\u{2026}";
/// What the list says when the worktree defines no task.
pub const NO_TASKS: &str =
    "This worktree defines no tasks: add them to .vscode/tasks.json, or scripts to a package.json";
/// What the list says when no task matches the search.
pub const NO_MATCH: &str = "No task matches the search";
/// What the right side says while no task is selected.
pub const NOTHING_SELECTED: &str = "Select a task to see its terminal";
/// The button that opens a file of tasks that could not be read.
pub const OPEN_THE_FILE: &str = "Open the file";
/// The terminal of a task, as the screen reader says it.
pub const TERMINAL_AREA: &str = "Terminal of the task";
/// What the terminal of a task says under its output once it ended, after how it ended.
pub const PRESS_ENTER: &str = "Press Enter to run it again";
/// The button of the question asked before closing with tasks running that stops them.
pub const STOP_AND_CLOSE: &str = "Stop them and close";
/// What that question says under its title.
pub const CLOSING_DETAIL: &str = "Closing stops them, and every process they started.";
/// The question asked before `.vscode/tasks.json` is made.
pub const MAKE_FILE: &str = "Make .vscode/tasks.json?";
/// What that question says under its title.
pub const MAKE_FILE_DETAIL: &str = "This worktree has none yet: it is made with no tasks, for you to add them.";
/// The button of that question.
pub const MAKE_IT: &str = "Make the file";

/// The name of the tab with the number of tasks running beside it, when some are: `Tasks (2)`.
#[must_use]
pub fn tab_name(running: usize) -> String {
    if running == 0 {
        TASKS.to_owned()
    } else {
        format!("{TASKS} ({running})")
    }
}

/// The title of the question asked before closing with tasks running: `2 tasks are running.`
#[must_use]
pub fn tasks_running(count: usize) -> String {
    if count == 1 {
        String::from("1 task is running.")
    } else {
        format!("{count} tasks are running.")
    }
}

/// What the terminal of a task says under its output once it ended: `The task ended with exit code 2`, or that it was
/// stopped when the system gave no code.
#[must_use]
pub fn task_ended(code: Option<i32>, is_stopped: bool) -> String {
    match (is_stopped, code) {
        (true, _) | (false, None) => String::from("The task was stopped"),
        (false, Some(code)) => format!("The task ended with exit code {code}"),
    }
}

/// What the terminal of a task says when the task could not be started, with what the system said.
#[must_use]
pub fn not_started(why: &str) -> String {
    format!("The task could not be started: {}", why.trim())
}

/// How long a task ran, under how it ended: `It ran for 3 s`, `It ran for 2 min 5 s`.
#[must_use]
pub fn ran_for(seconds: u64) -> String {
    let (hours, minutes, rest) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    if hours > 0 {
        format!("It ran for {hours} h {minutes} min")
    } else if minutes > 0 {
        format!("It ran for {minutes} min {rest} s")
    } else {
        format!("It ran for {rest} s")
    }
}

/// The heading of the scripts of a `package.json`, under **npm**: the name of its folder.
#[must_use]
pub fn folder_line(folder: &str) -> String {
    format!("Folder: {folder}")
}

/// What a row is called for a screen reader: the name of its task, its command and how it stands.
#[must_use]
pub fn row_name(name: &str, command: &str, standing: &str) -> String {
    let mut said = name.to_owned();
    for part in [command, standing] {
        if !part.is_empty() {
            said.push_str(", ");
            said.push_str(part);
        }
    }
    said
}
