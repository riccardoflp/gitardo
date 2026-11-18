//! The Tasks tab of the bottom panel (`tasks.view`, D65): the tasks of the repository on the left, by source, with a
//! search field over them, and on the right the terminal of the task selected, drawn as those of `terminal.view`.

mod detail;
mod groups;
mod list;
mod menu;
mod quick_run;

pub use quick_run::{QuickRunChoice, QuickRunEntry, QuickRunIntent, QuickRunState, is_quick_run_key, quick_run_view};

use model::tasks::{GroupsAction, TaskReference};
use ui::{ConfirmAnswer, ConfirmStyle, Confirmation, Rect, Split, Splitter, Ui};

use crate::intent::Intent;
use crate::terminal::{TerminalIntent, TerminalState};
use crate::text::tasks_tab as words;

/// The least width of the column of the tasks.
const LEAST_LIST: f32 = 180.0;
/// The least width of the terminal of a task.
const LEAST_TERMINAL: f32 = 220.0;
/// How far an arrow moves the line between them.
const STEP: f32 = 16.0;

/// How a task stands, as its row says it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RowStatus {
    /// It was not run in the session.
    #[default]
    NotRun,
    /// It waits for the tasks it depends on.
    Waiting,
    /// It runs; a background task that is ready for those that wait for it says so.
    Running {
        /// Whether it is ready.
        is_ready: bool,
    },
    /// It ended with exit code 0.
    Succeeded,
    /// It ended with another exit code.
    Failed(i32),
    /// It was stopped.
    Stopped,
    /// It could not be started.
    NotStarted,
    /// A task it waited for did not succeed.
    DidNotRun,
}

impl RowStatus {
    /// Whether the task runs.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, RowStatus::Running { .. })
    }
}

/// What a heading of the list is over.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum HeadingKind {
    /// A source of tasks, or the scripts of one `package.json`.
    #[default]
    Source,
    /// **Groups**: the groups of the repository.
    Groups,
    /// A group of tasks.
    Group {
        /// Its name.
        name: String,
        /// Whether one of its tasks runs.
        is_running: bool,
        /// Whether it is the one clicked last, which `F2` renames.
        is_selected: bool,
    },
}

/// A heading of the list: **Groups**, a group, a source of tasks, or the scripts of one `package.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingRow {
    /// What it is over.
    pub kind: HeadingKind,
    /// What its fold is kept by.
    pub key: String,
    /// Its words.
    pub name: String,
    /// How far in it is: 0 for a source.
    pub depth: u8,
    /// Whether what is under it is hidden.
    pub is_folded: bool,
}

/// A task of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRow {
    /// The task.
    pub reference: TaskReference,
    /// The group its row is in; none under the heading of its source.
    pub group: Option<String>,
    /// Its name.
    pub name: String,
    /// Its command, on one line: the script of a `package.json` as the file has it.
    pub command: String,
    /// What **Copy command** copies: the command line that runs it.
    pub copied: String,
    /// How far in it is.
    pub depth: u8,
    /// How it stands.
    pub status: RowStatus,
    /// Why it cannot be run, when it cannot: it is dimmed and says so.
    pub cannot_run: Option<String>,
    /// The line of its label in `.vscode/tasks.json`, from 1; none for a script.
    pub line: Option<usize>,
    /// Whether it is the one selected.
    pub is_selected: bool,
}

/// A row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// A heading.
    Heading(HeadingRow),
    /// A task.
    Task(TaskRow),
}

/// A file of tasks that could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// What is said of it.
    pub said: String,
    /// The file, from the top of the worktree with `/`; empty when it is not a file of the worktree.
    pub file: String,
}

/// What the right side shows of the task selected.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Detail {
    /// The task; none while none is selected.
    pub reference: Option<TaskReference>,
    /// Its name.
    pub name: String,
    /// Its command.
    pub command: String,
    /// The folder it runs in, from the top of the worktree.
    pub folder: String,
    /// What is said over **Run**: why it cannot run, or why it did not.
    pub said: Option<String>,
    /// Whether it cannot be run at all: **Run** is dimmed.
    pub cannot_run: bool,
    /// Whether it has a terminal: it was run in the session.
    pub has_terminal: bool,
    /// Whether it runs.
    pub is_running: bool,
    /// The two lines under what it printed once it ended: how it ended, and how long it ran.
    pub ended: Option<(String, String)>,
}

/// What the Tasks tab lists, made by the store for each frame.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TaskList {
    /// Whether a repository is shown.
    pub has_repository: bool,
    /// Whether its tasks were read.
    pub is_read: bool,
    /// Whether its folder is trusted.
    pub is_trusted: bool,
    /// The rows, those under a folded heading left out, those the search does not find too.
    pub rows: Vec<Row>,
    /// The names of the groups of the repository, in their order.
    pub groups: Vec<String>,
    /// What is typed in the search field.
    pub search: String,
    /// The width of the column, when it was dragged.
    pub width: Option<f32>,
    /// The files that could not be read.
    pub problems: Vec<Problem>,
    /// The task selected.
    pub detail: Detail,
    /// How many tasks run, said beside the name of the tab.
    pub running: usize,
    /// How many tasks run, while closing a tab or the window asks about them.
    pub closing: Option<usize>,
}

/// What the Tasks tab of the bottom panel shows.
#[derive(Clone, Debug, PartialEq)]
pub struct TasksTabState<'a> {
    /// The tasks and the one selected.
    pub list: &'a TaskList,
    /// The terminal of the task selected.
    pub terminal: TerminalState<'a>,
}

/// Something asked of the Tasks tab.
#[derive(Clone, Debug, PartialEq)]
pub enum TasksIntent {
    /// A click on a row: the task is selected, and its terminal shown.
    Select(TaskReference),
    /// **Run** on a row or in its menu, a double click on it, `Enter` in the terminal of a task that ended.
    Run(TaskReference),
    /// **Restart** in the menu of a task that runs: it is stopped, then run again.
    Restart(TaskReference),
    /// **Stop** on a row or in its menu.
    Stop(TaskReference),
    /// A change of the groups: one made, renamed, deleted or moved, a task put in one, moved in it or taken out.
    Groups(GroupsAction),
    /// **Run all** on a group, by its name.
    RunGroup(String),
    /// **Stop all** on a group, by its name.
    StopGroup(String),
    /// **Import groups…**: the groups of a file added to those of the repository.
    ImportGroups,
    /// **Export groups…**: groups of the repository saved to a file.
    ExportGroups,
    /// Something asked of the picker of **Run Task…**.
    QuickRun(QuickRunIntent),
    /// The text of the search field changed.
    Search(String),
    /// A heading clicked: what is under it is folded, or unfolded.
    Fold(String),
    /// **Collapse all**, with true, or **Expand all**.
    FoldAll(bool),
    /// The width of the column dragged; none when a double click puts it back at a third of the panel.
    SetWidth(Option<f32>),
    /// Something asked of the terminal of the task selected.
    Terminal(TerminalIntent),
    /// **Open tasks.json**: the file in a File page, made first, after asking, in a worktree that has none.
    OpenTasksFile,
    /// **Open the file** beside a file that could not be read.
    OpenFile(String),
    /// **Open in tasks.json**: the File page at the line of the label of a task.
    OpenAt(TaskReference),
    /// **Trust this folder**.
    Trust,
    /// The answer to **`<n>` tasks are running.**: true stops them and closes.
    CloseAnswer(bool),
}

/// The Tasks tab over the rectangle its caller clipped it to: the column of the tasks, as wide as a third of the tab
/// until the line at its right is dragged, with its search field, the menu with **Open tasks.json**, **Collapse all**
/// and **Expand all**, the groups of the repository under **Groups**, each with **Run all** or **Stop all**, and a row
/// for each task under the heading of its source, with **Run** or **Stop** at its right, a spinner while it runs, a
/// check when it ended with exit code 0 and the exit code in red otherwise; and at its right the terminal of the task
/// selected, or its command, its folder and **Run** while it was never run. In a folder not trusted every **Run** is
/// dimmed, and the column says so with **Trust this folder**.
///
/// Feature: tasks.view
pub fn tasks_view(state: &TasksTabState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut intents: Vec<Intent> = closing_question(state.list, ui).into_iter().collect();
    if !state.list.has_repository {
        list::say(ui, rect, words::NO_REPOSITORY);
        return intents;
    }
    let splitter = Splitter {
        split: Split::Columns,
        default_size: (rect.width / 3.0).max(LEAST_LIST),
        least_first: LEAST_LIST,
        least_second: LEAST_TERMINAL,
        size: state.list.width,
        step: STEP,
    };
    let split = ui.splitter("tasks column", rect, &splitter);
    if split.is_reset {
        intents.push(tasks(TasksIntent::SetWidth(None)));
    } else if split.is_changed {
        intents.push(tasks(TasksIntent::SetWidth(Some(split.size))));
    }
    let gap = ui.theme().sizes.spacing.small;
    let left = Rect::new(
        split.first.x,
        split.first.y,
        (split.first.width - gap).max(0.0),
        split.first.height,
    );
    let right = Rect::new(
        split.second.x + gap,
        split.second.y,
        (split.second.width - gap).max(0.0),
        split.second.height,
    );
    intents.extend(ui.clip(left, |ui| list::list_view(state.list, ui)));
    intents.extend(ui.clip(right, |ui| detail::detail_view(state, ui)));
    intents
}

/// An intent of the Tasks tab.
pub(crate) fn tasks(intent: TasksIntent) -> Intent {
    Intent::TasksTab(intent)
}

/// **`<n>` tasks are running.**, asked before a repository tab or the window closes with tasks running: **Stop them
/// and close**, or **Cancel**, which leaves them running.
fn closing_question(list: &TaskList, ui: &mut Ui<'_>) -> Option<Intent> {
    let count = list.closing?;
    let title = words::tasks_running(count);
    let confirmation = Confirmation {
        title: &title,
        message: words::CLOSING_DETAIL,
        commands: &[],
        confirm: words::STOP_AND_CLOSE,
        style: ConfirmStyle::Normal,
    };
    let answer = ui.confirm("tasks are running", &confirmation)?;
    Some(tasks(TasksIntent::CloseAnswer(answer == ConfirmAnswer::Confirmed)))
}
