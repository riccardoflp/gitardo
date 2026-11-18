//! The Tasks tab of the bottom panel (`tasks.view`): each repository tab has its own tasks, as last read from the
//! worktree it shows, the terminals of those it ran, kept while another tab is shown and ended when the tab closes,
//! the task selected and the width of its column. The store knows how each task stands and the screen its terminal
//! shows, and asks the window's task runs to read, start, write, size, scroll, select, stop and end.

mod closing;
mod commands;
mod groups;
mod quick_run;
mod runs;
mod view;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use model::contract::{Answer, Answers, FeatureId, FollowUp, Target};
use model::repo::Change;
use model::tasks::{Run, RunOrder, TASKS_FILE, Task, TaskChoice, TaskEnd, TaskGroup, TaskReference, TaskTarget, Tasks};
use model::terminal::{Screen, Selection};
use views::file_view::FileTarget;
use views::tasks::TasksIntent;
use views::terminal::TerminalIntent;

use super::message::Effect;
use super::state::State;
use super::terminals::shown_tab;

pub use closing::{Closing, close_requested, close_tabs};
pub use commands::{Commands, CommandsEffect, CommandsMessage, Waiting, hold, reduce_commands};
pub use groups::operation_ended;
pub use quick_run::{QuickRun, quick_run_entries, quick_run_state};
pub use runs::{carry_out, follow, reduce_message};
pub use view::{task_list, terminal_state};

/// The size the terminal of a task starts at before the tab said its own.
const START_SIZE: (u16, u16) = (80, 24);

/// What the tasks of a worktree were read as.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TasksReadout {
    /// The tasks the worktree defines.
    pub tasks: Tasks,
    /// The groups of the repository.
    pub groups: Vec<TaskGroup>,
    /// What kept the groups, or the files of the worktree, from being read.
    pub problem: Option<String>,
    /// Whether the worktree has a `.vscode/tasks.json`.
    pub has_tasks_file: bool,
}

/// How the run of a task stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryStatus {
    /// Its terminal starts.
    Starting,
    /// It runs.
    Running,
    /// It ended: with exit code 0, or how otherwise.
    Ended(Result<(), TaskEnd>),
}

/// A task run in the session, with its terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The number of its terminal in the window, which the window's task runs know it by.
    pub id: u64,
    /// The task, as it was when it started.
    pub task: Task,
    /// How it stands.
    pub status: EntryStatus,
    /// Whether it is a background task that is ready for those that wait for it.
    pub is_ready: bool,
    /// Whether **Stop** was asked of it: its end is then a stop, whatever its exit code.
    pub is_stopping: bool,
    /// What its terminal shows, once it printed.
    pub screen: Option<Screen>,
    /// Its exit code, once it ended with one.
    pub code: Option<i32>,
    /// How long it ran, once it ended.
    pub took: Option<Duration>,
}

impl Entry {
    /// Whether it starts or runs.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self.status, EntryStatus::Starting | EntryStatus::Running)
    }
}

/// The tasks of a repository tab.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TabTasks {
    /// The worktree the tasks were read from, or are being read from.
    pub folder: PathBuf,
    /// What was read.
    pub read: Option<TasksReadout>,
    /// The number of the read under way.
    pub reading: Option<u64>,
    /// Whether a file of tasks, or the file of the groups, changed since the last read started.
    pub is_stale: bool,
    /// The tasks run in the session, each with its terminal, the latest run of each.
    pub entries: Vec<Entry>,
    /// The runs under way: each a task with those it depends on.
    pub runs: Vec<Run>,
    /// The tasks stopped to be restarted, each with the order it runs in once it ended.
    pub restarting: Vec<(TaskReference, RunOrder)>,
    /// The tasks that did not run because one they waited for did not succeed, each with what is said of it.
    pub blocked: BTreeMap<TaskReference, String>,
    /// The commands of the worktrees made from the tab in the session, the latest last
    /// (`worktree-setup.command-steps`).
    pub commands: Vec<Commands>,
    /// The heading the column scrolls to, by its key, with the number of the request: the worktree whose commands
    /// started last.
    pub revealed: Option<(u64, String)>,
    /// The task selected.
    pub selected: Option<TaskReference>,
    /// The group whose heading was clicked last, which `F2` renames; none once a task is selected.
    pub group_selected: Option<String>,
    /// Whether **Import groups…** waits for the tasks to be read, which say what the names of its file stand for.
    pub is_importing: bool,
    /// What is typed in the search field.
    pub search: String,
    /// The headings folded, by their keys.
    pub folded: BTreeSet<String>,
    /// The width of the column, when it was dragged.
    pub width: Option<f32>,
    /// The cell the main button was pressed on in the terminal shown, while a selection is dragged from it.
    pub anchor: Option<(usize, usize)>,
}

impl TabTasks {
    /// The latest run of a task in the session.
    #[must_use]
    pub fn entry(&self, task: &TaskReference) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.task.reference == *task)
    }

    /// The tasks that start or run.
    #[must_use]
    pub fn running(&self) -> Vec<TaskReference> {
        let running = self.entries.iter().filter(|entry| entry.is_running());
        running.map(|entry| entry.task.reference.clone()).collect()
    }

    /// A task by its reference: as the worktree defines it now, or as it was when it last ran.
    #[must_use]
    pub fn task(&self, reference: &TaskReference) -> Option<&Task> {
        if reference.is_of_new_worktree() {
            let mut commands = self.commands.iter().flat_map(|commands| &commands.tasks);
            return commands.find(|task| task.reference == *reference);
        }
        let read = self.read.as_ref().and_then(|read| read.tasks.find(reference));
        read.or_else(|| self.entry(reference).map(|entry| &entry.task))
    }
}

/// The Tasks tab of the window.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TasksTab {
    /// The tasks of each repository tab, by the folder of the tab.
    pub tabs: BTreeMap<PathBuf, TabTasks>,
    /// The number of the next terminal of a task.
    pub next_id: u64,
    /// The number of the next read.
    pub next_read: u64,
    /// The number of the next commands of a new worktree.
    pub next_commands: u64,
    /// The size the tab last gave the terminals of the tasks, in columns and rows.
    pub size: Option<(u16, u16)>,
    /// What waits for the answer to **`<n>` tasks are running.**
    pub closing: Option<Closing>,
    /// What closes once the tasks it stopped ended, with the terminals of those tasks.
    pub closing_once_ended: Option<(Closing, Vec<u64>)>,
    /// The picker of **Run Task…**, while it is open.
    pub quick_run: Option<QuickRun>,
}

/// What the window's task runs are asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskRunEffect {
    /// Read the tasks of a worktree and the groups of its repository, for a repository tab.
    Read {
        /// The folder of the tab.
        tab: PathBuf,
        /// The worktree.
        folder: PathBuf,
        /// The number of the read.
        serial: u64,
    },
    /// Start a task in a terminal of a number.
    Start {
        /// Its number.
        id: u64,
        /// The task.
        task: Box<Task>,
        /// `terminal.shellPath`: empty for the system's own shell.
        shell_path: String,
        /// `terminal.scrollbackLines`.
        scrollback: usize,
        /// Its size, in columns and rows.
        size: (u16, u16),
    },
    /// Give a task what was typed or pasted.
    Write(u64, Vec<u8>),
    /// Give the terminal of every task a size, in columns and rows.
    Resize((u16, u16)),
    /// Scroll the terminal of a task by some lines, up into what it printed before for a positive number.
    Scroll(u64, i32),
    /// Select in the terminal of a task, or take its selection away.
    Select(u64, Option<Selection>),
    /// Stop a task: an interrupt now, and its end with every process it started three seconds later if it still runs.
    Stop(u64),
    /// End a task and every process it started at once, and forget its terminal.
    Kill(u64),
}

/// What the window's task runs tell.
#[derive(Clone, Debug, PartialEq)]
pub enum TaskRunMessage {
    /// The tasks of a repository tab were read.
    Read {
        /// The folder of the tab.
        tab: PathBuf,
        /// The number of the read.
        serial: u64,
        /// What was read.
        readout: Box<TasksReadout>,
    },
    /// The terminal of a task started.
    Started(u64),
    /// A task could not start, with what the system said.
    NotStarted(u64, String),
    /// A task printed something: the window reads its screen, which it tells as [`TaskRunMessage::Screen`].
    Printed(u64),
    /// What the terminal of a task shows now, and whether the task is ready for those that wait for it.
    Screen(u64, Box<Screen>, bool),
    /// A moment passed for a background task: the window looks whether it is ready, which it tells as
    /// [`TaskRunMessage::Ready`].
    Tick(u64),
    /// A background task is ready for those that wait for it.
    Ready(u64),
    /// Three seconds passed since a task was asked to stop: the window ends it if it still runs.
    StopDue(u64),
    /// A task ended, with its exit code when the system gave one, and how long it ran.
    Ended(u64, Option<i32>, Duration),
}

/// The tasks of the repository tab shown.
pub(super) fn shown(state: &State) -> Option<&TabTasks> {
    let (tab, _) = shown_tab(state)?;
    state.tasks_tab.tabs.get(&tab)
}

/// The tasks of the repository tab shown, to change, made when the tab has none yet.
fn shown_mut(state: &mut State) -> Option<&mut TabTasks> {
    let (tab, _) = shown_tab(state)?;
    Some(state.tasks_tab.tabs.entry(tab).or_default())
}

/// The run of the task selected in the repository tab shown.
fn selected_entry(state: &State) -> Option<&Entry> {
    let tab = shown(state)?;
    tab.entry(tab.selected.as_ref()?)
}

/// How many tasks start or run in the repository tab shown.
#[must_use]
pub fn running_count(state: &State) -> usize {
    shown(state).map_or(0, |tab| tab.entries.iter().filter(|entry| entry.is_running()).count())
}

/// Runs an operation of the tasks in the repository shown, on a task, a group or the folder, with the tasks of the
/// worktree and those running.
pub(super) fn ask(state: &mut State, feature: &'static str, choice: TaskChoice, preset: Answers) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = (read.folder.clone(), Arc::clone(&read.state));
    let tab = shown(state);
    let target = TaskTarget {
        tasks: tab
            .and_then(|tab| tab.read.as_ref())
            .map(|read| read.tasks.clone())
            .unwrap_or_default(),
        choice,
        running: tab.map(TabTasks::running).unwrap_or_default(),
    };
    let asked = (FeatureId(feature), Target::Task(Box::new(target)));
    super::feature::run_in(state, repository, asked, preset)
}

/// What was asked of the Tasks tab.
pub fn reduce(state: &mut State, intent: TasksIntent) -> Vec<Effect> {
    match intent {
        TasksIntent::Select(task) => {
            select(state, task);
            Vec::new()
        }
        // A command of a new worktree runs once, when the worktree is made: asked again, it is only shown.
        TasksIntent::Run(task) | TasksIntent::Restart(task) if task.is_of_new_worktree() => {
            select(state, task);
            Vec::new()
        }
        TasksIntent::Run(task) => {
            select(state, task.clone());
            ask(state, "tasks.run", TaskChoice::Task(task), Answers::new())
        }
        TasksIntent::Restart(task) => {
            let restart = Answer::Choice(features::tasks::run::RESTART.to_owned());
            let preset = Answers::new().with(features::tasks::run::RUNNING, restart);
            ask(state, "tasks.run", TaskChoice::Task(task), preset)
        }
        TasksIntent::Stop(task) => ask(state, "tasks.stop", TaskChoice::Task(task), Answers::new()),
        TasksIntent::Groups(_)
        | TasksIntent::RunGroup(_)
        | TasksIntent::StopGroup(_)
        | TasksIntent::ImportGroups
        | TasksIntent::ExportGroups => groups::reduce(state, intent),
        TasksIntent::QuickRun(asked) => quick_run::reduce(state, asked),
        TasksIntent::Trust => ask(state, "tasks.run", TaskChoice::Trust, Answers::new()),
        TasksIntent::Terminal(asked) => terminal(state, asked),
        TasksIntent::CloseAnswer(is_closing) => closing::answered(state, is_closing),
        TasksIntent::OpenTasksFile | TasksIntent::OpenFile(_) | TasksIntent::OpenAt(_) => open_file(state, &intent),
        TasksIntent::Search(_) | TasksIntent::Fold(_) | TasksIntent::FoldAll(_) | TasksIntent::SetWidth(_) => {
            look(state, intent);
            Vec::new()
        }
    }
}

/// Selects a task in the repository tab shown, and gives its terminal the keyboard.
fn select(state: &mut State, task: TaskReference) {
    if let Some(tab) = shown_mut(state) {
        tab.selected = Some(task);
        tab.group_selected = None;
        tab.anchor = None;
        state.panel.focus += 1;
    }
}

/// How the column looks: what is searched, the headings folded, its width.
fn look(state: &mut State, intent: TasksIntent) {
    let Some(tab) = shown_mut(state) else {
        return;
    };
    match intent {
        TasksIntent::Search(text) => tab.search = text,
        TasksIntent::Fold(key) => {
            tab.group_selected = groups::named_by(tab, &key);
            if !tab.folded.remove(&key) {
                let _new = tab.folded.insert(key);
            }
        }
        TasksIntent::FoldAll(true) => tab.folded = view::heading_keys(tab).into_iter().collect(),
        TasksIntent::FoldAll(false) => tab.folded.clear(),
        TasksIntent::SetWidth(width) => tab.width = width,
        // The other intents are not about how the column looks.
        _ => {}
    }
}

/// A file of tasks opened in a File page: `.vscode/tasks.json`, made first after asking in a worktree that has none,
/// at the line of the label of a task, or a file that could not be read.
fn open_file(state: &mut State, intent: &TasksIntent) -> Vec<Effect> {
    let tab = shown(state);
    let target = match intent {
        TasksIntent::OpenTasksFile => {
            let has_file = tab
                .and_then(|tab| tab.read.as_ref())
                .is_some_and(|read| read.has_tasks_file);
            if !has_file {
                return ask(state, "tasks.run", TaskChoice::MakeFile, Answers::new());
            }
            FileTarget::working_tree(TASKS_FILE)
        }
        TasksIntent::OpenFile(path) => FileTarget::working_tree(path),
        TasksIntent::OpenAt(task) => FileTarget {
            path: TASKS_FILE.to_owned(),
            revision: None,
            line: tab.and_then(|tab| tab.task(task)).and_then(|task| task.line),
        },
        // The other intents open no file.
        _ => return Vec::new(),
    };
    super::file_page::open(state, target)
}

/// What was asked of the terminal of the task selected.
fn terminal(state: &mut State, intent: TerminalIntent) -> Vec<Effect> {
    let entry = selected_entry(state).map(|entry| (entry.id, entry.is_running()));
    let runs = |effect| vec![Effect::TaskRuns(effect)];
    match (intent, entry) {
        (TerminalIntent::Resize(columns, rows), _) => {
            state.tasks_tab.size = Some((columns, rows));
            runs(TaskRunEffect::Resize((columns, rows)))
        }
        (TerminalIntent::Input(bytes), Some((id, true))) => runs(TaskRunEffect::Write(id, bytes)),
        (TerminalIntent::Scroll(lines), Some((id, _))) => runs(TaskRunEffect::Scroll(id, lines)),
        (TerminalIntent::Press(cell), Some((id, _))) => {
            if let Some(tab) = shown_mut(state) {
                tab.anchor = Some(cell);
            }
            runs(TaskRunEffect::Select(id, None))
        }
        (TerminalIntent::Drag(cell), Some((id, _))) => {
            let anchor = shown(state).and_then(|tab| tab.anchor);
            anchor.map_or_else(Vec::new, |from| {
                let selection = Selection {
                    from,
                    to: cell,
                    is_word: false,
                };
                runs(TaskRunEffect::Select(id, Some(selection)))
            })
        }
        (TerminalIntent::SelectWord(cell), Some((id, _))) => {
            let selection = Selection {
                from: cell,
                to: cell,
                is_word: true,
            };
            runs(TaskRunEffect::Select(id, Some(selection)))
        }
        // The terminal of a task is not made, shown among others or killed as those of the Terminal tab are, and what
        // is typed once the task ended goes nowhere.
        _ => Vec::new(),
    }
}

/// Changes the watcher saw in the repository of a tab: a file of tasks of its worktree, or the file of the groups in
/// its common git folder, has its tasks read again the next time they are shown (`tasks.read`, `tasks.edit-groups`).
pub fn changed(state: &mut State, tab: Option<&std::path::Path>, changes: &[Change]) {
    let tab = match tab {
        Some(tab) => Some(tab.to_path_buf()),
        None => shown_tab(state).map(|(tab, _)| tab),
    };
    let Some(tasks) = tab.and_then(|tab| state.tasks_tab.tabs.get_mut(&tab)) else {
        return;
    };
    let is_about_tasks = changes.iter().any(|change| match change {
        Change::WorkingTree(path) => model::repo::sort_tasks_change(path, &[]) != model::repo::TreeChange::Nothing,
        other => model::repo::changes_task_groups(other),
    });
    tasks.is_stale |= is_about_tasks;
}

/// What an operation that ran asks of the Tasks tab, when a follow-up is one of its own.
#[must_use]
pub fn is_follow_up(follow_up: &FollowUp) -> bool {
    matches!(follow_up, FollowUp::RunTasks(_) | FollowUp::StopTasks(_))
}
