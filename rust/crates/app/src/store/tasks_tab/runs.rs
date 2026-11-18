//! The runs of the Tasks tab (`tasks.run`, `tasks.stop`): what an operation of the tasks asks of the window, the tasks
//! started as those they wait for end or are ready, and what the window's task runs tell of each.

use std::path::PathBuf;

use model::contract::FollowUp;
use model::tasks::{Run, RunOrder, StepState, TaskEnd, TaskReference, TasksToRun};
use views::panel::{PanelIntent, PanelTab};

use super::super::message::Effect;
use super::super::panel;
use super::super::state::State;
use super::super::terminals::shown_tab;
use super::{Entry, EntryStatus, START_SIZE, TabTasks, TaskRunEffect, TaskRunMessage};

/// What an operation of the tasks that ran asks of the window: tasks run, each after those it waits for, with the
/// Tasks tab shown at one of them, or tasks stopped.
pub fn carry_out(state: &mut State, follow_up: FollowUp) -> Vec<Effect> {
    let Some((key, _)) = shown_tab(state) else {
        return Vec::new();
    };
    match follow_up {
        FollowUp::RunTasks(run) => run_tasks(state, &key, *run),
        FollowUp::StopTasks(tasks) => {
            let tab = state.tasks_tab.tabs.entry(key).or_default();
            stop(tab, |entry| tasks.contains(&entry.task.reference))
        }
        // The other follow-ups are carried out elsewhere.
        _ => Vec::new(),
    }
}

/// Asks every task of a tab that runs, among those chosen, to stop: an interrupt now, and its end three seconds later
/// if it still runs.
pub(super) fn stop(tab: &mut TabTasks, is_chosen: impl Fn(&Entry) -> bool) -> Vec<Effect> {
    let mut effects = Vec::new();
    for entry in &mut tab.entries {
        if entry.is_running() && is_chosen(entry) {
            entry.is_stopping = true;
            effects.push(Effect::TaskRuns(TaskRunEffect::Stop(entry.id)));
        }
    }
    effects
}

/// Runs tasks in the repository tab shown: the panel is shown at the Tasks tab with the task to show selected, a task
/// to restart is stopped first and runs again once it ended, and the others start as their order says.
fn run_tasks(state: &mut State, key: &PathBuf, run: TasksToRun) -> Vec<Effect> {
    let mut effects = Vec::new();
    if run.show.is_some() {
        effects.extend(panel::reduce(state, &PanelIntent::Show(PanelTab::Tasks)));
        state.panel.focus += 1;
    }
    let tab = state.tasks_tab.tabs.entry(key.clone()).or_default();
    if let Some(shown) = run.show {
        tab.selected = Some(shown);
        tab.anchor = None;
    }
    effects.extend(stop(tab, |entry| run.restart.contains(&entry.task.reference)));
    for order in run.orders {
        for step in &order.steps {
            let _said = tab.blocked.remove(&step.task);
        }
        let restarted = order.steps.last().map(|step| step.task.clone());
        match restarted.filter(|task| run.restart.contains(task) && tab.running().contains(task)) {
            Some(task) => tab.restarting.push((task, order)),
            None => begin(tab, order),
        }
    }
    effects.extend(advance(state, key));
    effects
}

/// A run of an order begins in a tab: the tasks running already are not run again, and those of them that are ready
/// let the tasks that wait for them start.
pub(super) fn begin(tab: &mut TabTasks, order: RunOrder) {
    let mut run = Run::new(order, &tab.running());
    for entry in tab.entries.iter().filter(|entry| entry.is_running() && entry.is_ready) {
        run.ready(&entry.task.reference);
    }
    tab.runs.push(run);
}

/// Starts every task the runs of a tab say to start now, until none says more: a task that ends as it starts, having
/// no command of its own or not being in the worktree any more, lets those that wait for it go on at once. The tasks
/// kept from running are remembered with what is said of them, and the runs that are over forgotten.
pub(super) fn advance(state: &mut State, key: &PathBuf) -> Vec<Effect> {
    let shell_path = state.settings.terminal.shell_path.clone();
    let scrollback = state.settings.terminal.scrollback;
    let size = state.tasks_tab.size.unwrap_or(START_SIZE);
    let mut effects = Vec::new();
    loop {
        let Some(tab) = state.tasks_tab.tabs.get_mut(key) else {
            return effects;
        };
        let started: Vec<TaskReference> = tab.runs.iter_mut().flat_map(Run::start).collect();
        if started.is_empty() {
            break;
        }
        for task in started {
            if tab.running().contains(&task) {
                continue;
            }
            // The terminal of the run it had before is forgotten: this one takes its place.
            let before = tab.entries.iter().filter(|entry| entry.task.reference == task);
            effects.extend(before.map(|entry| Effect::TaskRuns(TaskRunEffect::Kill(entry.id))));
            let id = state.tasks_tab.next_id;
            match start(tab, &task, id) {
                Started::Runs(task) => {
                    state.tasks_tab.next_id += 1;
                    effects.push(Effect::TaskRuns(TaskRunEffect::Start {
                        id,
                        task,
                        shell_path: shell_path.clone(),
                        scrollback,
                        size,
                    }));
                }
                Started::Ended(end) => {
                    state.tasks_tab.next_id += 1;
                    tell_ended(tab, &task, &end);
                }
            }
        }
    }
    if let Some(tab) = state.tasks_tab.tabs.get_mut(key) {
        for run in &tab.runs {
            for (task, step) in run.steps() {
                if let StepState::DidNotRun(blocked) = step {
                    let _before = tab.blocked.insert(task.clone(), blocked.said());
                }
            }
        }
        tab.runs.retain(|run| !run.is_over());
    }
    effects
}

/// How the start of a task went in the store.
enum Started {
    /// Its terminal is to be started, with the task.
    Runs(Box<model::tasks::Task>),
    /// It ended as it started.
    Ended(Result<(), TaskEnd>),
}

/// Starts a task of a tab under the number of its terminal: its run takes the place of the one it had before.
fn start(tab: &mut TabTasks, reference: &TaskReference, id: u64) -> Started {
    let Some(task) = tab.task(reference).cloned() else {
        let said = format!("`{reference}` is not in this worktree.");
        let _before = tab.blocked.insert(reference.clone(), said.clone());
        return Started::Ended(Err(TaskEnd::NotStarted(said)));
    };
    let has_command = task.command.is_some();
    tab.entries.retain(|entry| entry.task.reference != *reference);
    tab.entries.push(Entry {
        id,
        task: task.clone(),
        status: if has_command {
            EntryStatus::Starting
        } else {
            EntryStatus::Ended(Ok(()))
        },
        is_ready: false,
        is_stopping: false,
        screen: None,
        code: (!has_command).then_some(0),
        took: None,
    });
    if has_command {
        Started::Runs(Box::new(task))
    } else {
        // A task with no command of its own is only the tasks it depends on: it ended once they did.
        Started::Ended(Ok(()))
    }
}

/// Tells every run of a tab that a task ended.
fn tell_ended(tab: &mut TabTasks, task: &TaskReference, end: &Result<(), TaskEnd>) {
    for run in &mut tab.runs {
        run.ended(task, end.clone());
    }
}

/// The repository tab that has the terminal of a number.
fn tab_of(state: &State, id: u64) -> Option<PathBuf> {
    let mut tabs = state.tasks_tab.tabs.iter();
    tabs.find_map(|(key, tab)| tab.entries.iter().any(|entry| entry.id == id).then(|| key.clone()))
}

/// What the window's task runs told; what waited for its tasks to end closes once they did.
pub fn reduce_message(state: &mut State, message: TaskRunMessage) -> Vec<Effect> {
    let mut effects = told(state, message);
    effects.extend(super::closing::go_on(state));
    effects
}

/// What the window's task runs told of the tasks read, or of a task; the commands of a new worktree go on, or end,
/// as their tasks do.
fn told(state: &mut State, message: TaskRunMessage) -> Vec<Effect> {
    let key = match &message {
        TaskRunMessage::Started(id) | TaskRunMessage::NotStarted(id, _) | TaskRunMessage::Ended(id, ..) => {
            tab_of(state, *id)
        }
        // What a task prints changes nothing for the commands that wait for it.
        _ => None,
    };
    let mut effects = told_of(state, message);
    if let Some(key) = key {
        effects.extend(super::commands::follow(state, &key));
    }
    effects
}

/// What the window's task runs told of the tasks read, or of a task.
fn told_of(state: &mut State, message: TaskRunMessage) -> Vec<Effect> {
    let id = match message {
        TaskRunMessage::Read { tab, serial, readout } => {
            if let Some(tasks) = state.tasks_tab.tabs.get_mut(&tab)
                && tasks.reading == Some(serial)
            {
                tasks.read = Some(*readout);
                tasks.reading = None;
                return super::groups::read(state);
            }
            return Vec::new();
        }
        TaskRunMessage::Started(id)
        | TaskRunMessage::NotStarted(id, _)
        | TaskRunMessage::Printed(id)
        | TaskRunMessage::Screen(id, ..)
        | TaskRunMessage::Tick(id)
        | TaskRunMessage::Ready(id)
        | TaskRunMessage::StopDue(id)
        | TaskRunMessage::Ended(id, ..) => id,
    };
    let Some(key) = tab_of(state, id) else {
        return Vec::new();
    };
    let found = state.tasks_tab.tabs.get_mut(&key);
    let Some(entry) = found.and_then(|tab| tab.entries.iter_mut().find(|entry| entry.id == id)) else {
        return Vec::new();
    };
    match message {
        TaskRunMessage::Started(_) => {
            if entry.status == EntryStatus::Starting {
                entry.status = EntryStatus::Running;
            }
            Vec::new()
        }
        TaskRunMessage::Screen(_, screen, is_ready) => {
            entry.screen = Some(*screen);
            if is_ready { ready(state, &key, id) } else { Vec::new() }
        }
        TaskRunMessage::Ready(_) => ready(state, &key, id),
        TaskRunMessage::NotStarted(_, said) => ended(state, &key, id, &Err(TaskEnd::NotStarted(said))),
        TaskRunMessage::Ended(_, code, took) => {
            entry.code = code;
            entry.took = Some(took);
            let end = match code {
                Some(0) if !entry.is_stopping => Ok(()),
                Some(code) if !entry.is_stopping => Err(TaskEnd::Code(code)),
                Some(_) | None => Err(TaskEnd::Stopped),
            };
            ended(state, &key, id, &end)
        }
        // The window's task runs answer these themselves before the store hears of them, and the tasks read were
        // kept above.
        TaskRunMessage::Printed(_)
        | TaskRunMessage::Tick(_)
        | TaskRunMessage::StopDue(_)
        | TaskRunMessage::Read { .. } => Vec::new(),
    }
}

/// A background task that runs is ready: the tasks that wait for it start.
fn ready(state: &mut State, key: &PathBuf, id: u64) -> Vec<Effect> {
    let Some(tab) = state.tasks_tab.tabs.get_mut(key) else {
        return Vec::new();
    };
    let Some(entry) = tab.entries.iter_mut().find(|entry| entry.id == id) else {
        return Vec::new();
    };
    if entry.is_ready || !entry.is_running() {
        return Vec::new();
    }
    entry.is_ready = true;
    let task = entry.task.reference.clone();
    for run in &mut tab.runs {
        run.ready(&task);
    }
    advance(state, key)
}

/// A task ended: its row says how, the runs that wait for it go on, and a task that was stopped to be restarted runs
/// again.
fn ended(state: &mut State, key: &PathBuf, id: u64, end: &Result<(), TaskEnd>) -> Vec<Effect> {
    let Some(tab) = state.tasks_tab.tabs.get_mut(key) else {
        return Vec::new();
    };
    let Some(entry) = tab.entries.iter_mut().find(|entry| entry.id == id) else {
        return Vec::new();
    };
    entry.status = EntryStatus::Ended(end.clone());
    let task = entry.task.reference.clone();
    tell_ended(tab, &task, end);
    let (again, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut tab.restarting)
        .into_iter()
        .partition(|(restarted, _)| *restarted == task);
    tab.restarting = waiting;
    for (_, order) in again {
        begin(tab, order);
    }
    advance(state, key)
}

/// After each message: the tasks of the tabs that closed end, with every process they started, and the tasks of the
/// repository tab shown are read when the Tasks tab or the picker of **Run Task…** shows them and they were not read,
/// or a file of theirs changed since, or another worktree is shown in the tab.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let shown = shown_tab(state);
    let mut kept: Vec<PathBuf> = state.tabs.open.iter().map(|tab| tab.folder.clone()).collect();
    kept.extend(shown.iter().map(|(tab, _)| tab.clone()));
    let closed: Vec<PathBuf> = state
        .tasks_tab
        .tabs
        .keys()
        .filter(|key| !kept.contains(key))
        .cloned()
        .collect();
    let mut effects = Vec::new();
    for key in closed {
        if let Some(tab) = state.tasks_tab.tabs.remove(&key) {
            let ids = tab.entries.iter().map(|entry| entry.id);
            effects.extend(ids.map(|id| Effect::TaskRuns(TaskRunEffect::Kill(id))));
        }
    }
    let look = panel::look(state);
    let is_listed = (look.is_shown && look.tab == PanelTab::Tasks) || state.tasks_tab.quick_run.is_some();
    let Some((key, folder)) = shown.filter(|_| is_listed) else {
        return effects;
    };
    let tab = state.tasks_tab.tabs.entry(key.clone()).or_default();
    let is_elsewhere = tab.folder != folder;
    let is_wanted = tab.read.is_none() || tab.is_stale;
    if is_elsewhere || (is_wanted && tab.reading.is_none()) {
        if is_elsewhere {
            tab.read = None;
        }
        let serial = state.tasks_tab.next_read;
        state.tasks_tab.next_read += 1;
        tab.reading = Some(serial);
        tab.is_stale = false;
        tab.folder.clone_from(&folder);
        effects.push(Effect::TaskRuns(TaskRunEffect::Read {
            tab: key,
            folder,
            serial,
        }));
    }
    effects
}
