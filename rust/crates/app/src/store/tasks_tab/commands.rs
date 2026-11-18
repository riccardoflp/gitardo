//! The commands of new worktrees among the tasks (`worktree-setup.command-steps`): the commands an operation hands
//! over once its other steps ran, run in the Tasks tab as tasks of the source **New worktrees**, each in a terminal of
//! its own, one after the other. What the operation does once it succeeded, such as opening the worktree, waits for
//! the last of them; when one does not succeed, those after it do not run, and Gitardo says which and offers to open
//! the worktree all the same.

use std::path::PathBuf;

use engine::{LogEntry, TasksFailure};
use model::contract::{FollowUp, Step, Text};
use model::repo::ReadCause;
use model::tasks::{RunOrder, RunStep, Task, TaskEnd, TaskFailure, TaskKind, TaskReference, Word, failure_said};
use views::panel::{PanelIntent, PanelTab};

use super::super::feature::{Ended, Said, ended_as_said};
use super::super::message::Effect;
use super::super::state::State;
use super::super::terminals::shown_tab;
use super::super::{panel, reads, worktrees};
use super::{EntryStatus, TabTasks, TaskRunEffect, runs};

/// How many worktrees a repository tab keeps the commands of, the latest; those still running are never dropped.
const KEPT: usize = 5;

/// What a plan hands over from its first command on, with what it offers when one does not succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    /// The steps, from the first command on.
    pub steps: Vec<Step>,
    /// What the plan has the window do when a step fails: the offers to open the worktree all the same.
    pub on_failure: Vec<FollowUp>,
}

/// What the thread that writes the Command log and runs the steps after the commands says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandsMessage {
    /// Lines for the Command log: a command that started, or what is said under one that did not succeed.
    Logged(Vec<LogEntry>),
    /// The steps a plan has after its commands ended: all ran, or the one that failed; and the lines they logged.
    Ended {
        /// The commands of which worktree, by their number.
        batch: u64,
        /// What did not succeed; none when all did.
        failure: Option<TasksFailure>,
        /// The lines of the Command log.
        log: Vec<LogEntry>,
    },
}

/// What the store asks about the commands of new worktrees besides their terminals, which the Tasks tab starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandsEffect {
    /// Writes in the Command log that a command starts, as the step of the plan it is.
    LogStarted(Step),
    /// Writes in the Command log what is said under a command that did not succeed.
    LogFailure(String),
    /// Runs the steps a plan has after its commands, apart, once they all succeeded.
    RunAfter {
        /// The commands of which worktree, by their number.
        batch: u64,
        /// The folder of the worktree.
        folder: PathBuf,
        /// The steps.
        steps: Vec<Step>,
    },
}

/// Where the commands of a worktree are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Some wait or run.
    Running,
    /// They all succeeded, and the steps the plan has after them run.
    Finishing,
    /// They are over.
    Over,
}

/// How a command stands, as its terminal says.
enum Stand {
    /// It waits for those before it, or did not run.
    NotStarted,
    /// It runs.
    Runs,
    /// It ended.
    Ended(Result<(), TaskEnd>),
}

/// The commands a plan handed over for a new worktree, with what the plan does once they all succeeded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commands {
    /// Their number in the window.
    pub number: u64,
    /// The worktree, as the views name it.
    pub worktree: String,
    /// The commands as tasks, in the order they run.
    pub tasks: Vec<Task>,
    /// How many of them were written in the Command log as started.
    logged: usize,
    /// Where they are.
    stage: Stage,
    /// The steps the plan has after them.
    after_them: Vec<Step>,
    /// The title of the plan.
    title: Text,
    /// What the plan does once they all succeeded.
    after: Vec<FollowUp>,
    /// Whether Undo can take the plan back.
    can_undo: bool,
    /// What the plan has the window do when one did not succeed.
    on_failure: Vec<FollowUp>,
}

impl Commands {
    /// Whether some of them wait or run, or the steps after them do.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.stage != Stage::Over
    }
}

/// The commands among the steps a plan handed over, as tasks of the source **New worktrees** that run in a shell in
/// the folder of the worktree, with the name of the worktree; and the steps that are not commands, which run after
/// them. A command written twice is listed twice, the second with its place after its name.
fn tasks_of(steps: &[Step]) -> (String, Vec<Task>, Vec<Step>) {
    let mut worktree = String::new();
    let mut tasks: Vec<Task> = Vec::new();
    let mut after_them = Vec::new();
    for step in steps {
        let Step::Task {
            command,
            directory,
            name,
        } = step
        else {
            after_them.push(step.clone());
            continue;
        };
        worktree.clone_from(name);
        let same = tasks.iter().filter(|task| task.command_line() == *command).count();
        let listed = if same == 0 {
            command.clone()
        } else {
            format!("{command} ({})", same + 1)
        };
        tasks.push(Task {
            kind: TaskKind::Shell,
            command: Some(Word::plain(command)),
            ..Task::named(TaskReference::new_worktree(name, &listed), directory.clone())
        });
    }
    (worktree, tasks, after_them)
}

/// The order the commands run in: each once the one before it ended with exit code 0.
fn in_sequence(tasks: &[Task]) -> RunOrder {
    let mut before: Option<TaskReference> = None;
    let steps = tasks.iter().map(|task| {
        let step = RunStep {
            task: task.reference.clone(),
            waits_for: before.iter().cloned().collect(),
        };
        before = Some(task.reference.clone());
        step
    });
    RunOrder { steps: steps.collect() }
}

/// Forgets the commands of a tab that are over beyond the [`KEPT`] latest, and those of a worktree named as one
/// being made, with their terminals.
fn drop_old(tab: &mut TabTasks, worktree: &str) -> Vec<Effect> {
    let mut dropped = Vec::new();
    tab.commands.retain(|commands| {
        let is_kept = commands.worktree != worktree;
        if !is_kept {
            dropped.push(commands.clone());
        }
        is_kept
    });
    while tab.commands.len() >= KEPT {
        let Some(oldest) = tab.commands.iter().position(|commands| !commands.is_running()) else {
            break;
        };
        dropped.push(tab.commands.remove(oldest));
    }
    let gone: Vec<&TaskReference> = dropped
        .iter()
        .flat_map(|commands| commands.tasks.iter().map(|task| &task.reference))
        .collect();
    let mut effects = Vec::new();
    tab.entries.retain(|entry| {
        let is_gone = gone.contains(&&entry.task.reference);
        if is_gone {
            effects.push(Effect::TaskRuns(TaskRunEffect::Kill(entry.id)));
        }
        !is_gone
    });
    tab.runs
        .retain(|run| !run.steps().any(|(task, _)| gone.contains(&task)));
    for task in gone {
        let _said = tab.blocked.remove(task);
    }
    effects
}

/// An operation ended: when its plan handed commands over, what follows it is held until they all succeed, they start
/// in the Tasks tab of the repository tab shown, which the bottom panel is shown at with the first of them selected,
/// and the repository is read again for the worktree made; otherwise the ending is given back as it came.
///
/// # Errors
///
/// The ending, given back, when no command was handed over or no repository tab is shown to list them in.
pub fn hold(state: &mut State, ended: Ended) -> Result<Vec<Effect>, Box<Ended>> {
    let Some((key, _)) = shown_tab(state) else {
        return Err(Box::new(ended));
    };
    let Ended::Ran {
        title,
        after,
        can_undo,
        waiting: Some(waiting),
        ..
    } = ended
    else {
        return Err(Box::new(ended));
    };
    let number = state.tasks_tab.next_commands;
    state.tasks_tab.next_commands += 1;
    let (worktree, tasks, after_them) = tasks_of(&waiting.steps);
    let said = after.iter().rev().find_map(|follow_up| match follow_up {
        FollowUp::Say(words) => Some(words.as_str().to_owned()),
        _ => None,
    });
    state.operations.said = said.map(|said| Said::new(said, false));
    let tab = state.tasks_tab.tabs.entry(key.clone()).or_default();
    let mut effects = drop_old(tab, &worktree);
    for heading in [
        super::view::NEW_WORKTREES.to_owned(),
        super::view::worktree_key(&worktree),
    ] {
        let _was_folded = tab.folded.remove(&heading);
    }
    tab.revealed = Some((number + 1, super::view::worktree_key(&worktree)));
    tab.selected = tasks.first().map(|task| task.reference.clone());
    tab.group_selected = None;
    tab.anchor = None;
    runs::begin(tab, in_sequence(&tasks));
    tab.commands.push(Commands {
        number,
        worktree,
        tasks,
        logged: 0,
        stage: Stage::Running,
        after_them,
        title,
        after,
        can_undo,
        on_failure: waiting.on_failure,
    });
    effects.extend(panel::reduce(state, &PanelIntent::Show(PanelTab::Tasks)));
    state.panel.focus += 1;
    effects.extend(reads::asked(state, ReadCause::OperationEnded));
    effects.extend(runs::advance(state, &key));
    effects.extend(follow(state, &key));
    Ok(effects)
}

/// What the Command log says under a command that did not succeed.
fn end_said(end: &TaskEnd) -> String {
    match end {
        TaskEnd::Code(code) => format!("It exited with {code}."),
        TaskEnd::Stopped => String::from("It was stopped."),
        TaskEnd::NotStarted(said) => said.clone(),
    }
}

/// How the commands of a worktree that run stand in their tab: each that started since is written in the Command
/// log, and they are over once one did not succeed, or all succeeded.
fn look(tab: &mut TabTasks, index: usize) -> (Vec<Effect>, Option<Result<(), TaskFailure>>) {
    let stands: Vec<Stand> = tab.commands.get(index).map_or_else(Vec::new, |commands| {
        let entries = commands.tasks.iter().map(|task| tab.entry(&task.reference));
        entries
            .map(|entry| match entry.map(|entry| &entry.status) {
                None => Stand::NotStarted,
                Some(EntryStatus::Starting | EntryStatus::Running) => Stand::Runs,
                Some(EntryStatus::Ended(end)) => Stand::Ended(end.clone()),
            })
            .collect()
    });
    let Some(commands) = tab
        .commands
        .get_mut(index)
        .filter(|commands| commands.stage == Stage::Running)
    else {
        return (Vec::new(), None);
    };
    let mut effects = Vec::new();
    let started = stands
        .iter()
        .take_while(|stand| !matches!(stand, Stand::NotStarted))
        .count();
    for task in commands.tasks.iter().take(started).skip(commands.logged) {
        effects.push(Effect::Commands(CommandsEffect::LogStarted(Step::Task {
            command: task.command_line(),
            directory: task.folder.clone(),
            name: commands.worktree.clone(),
        })));
    }
    commands.logged = commands.logged.max(started);
    let failed = stands.iter().enumerate().find_map(|(place, stand)| match stand {
        Stand::Ended(Err(end)) => Some((place, end.clone())),
        Stand::NotStarted | Stand::Runs | Stand::Ended(Ok(())) => None,
    });
    let over = match failed {
        Some((place, end)) => commands.tasks.get(place).map(|task| {
            Err(TaskFailure {
                command: task.command_line(),
                worktree: task.folder.clone(),
                end,
                after: commands.tasks.len() - place - 1,
            })
        }),
        None => stands
            .iter()
            .all(|stand| matches!(stand, Stand::Ended(Ok(()))))
            .then_some(Ok(())),
    };
    (effects, over)
}

/// After something was told of a task of a tab: the commands of new worktrees that started are written in the Command
/// log, and those that are over end as their plan says.
pub(super) fn follow(state: &mut State, key: &PathBuf) -> Vec<Effect> {
    let mut effects = Vec::new();
    let count = state.tasks_tab.tabs.get(key).map_or(0, |tab| tab.commands.len());
    for index in 0..count {
        let Some(tab) = state.tasks_tab.tabs.get_mut(key) else {
            break;
        };
        let (logged, over) = look(tab, index);
        effects.extend(logged);
        let Some(over) = over else {
            continue;
        };
        let Some(commands) = tab.commands.get_mut(index) else {
            continue;
        };
        match over {
            Ok(()) if commands.after_them.is_empty() => effects.extend(end(state, (key, index), None)),
            Ok(()) => {
                commands.stage = Stage::Finishing;
                let folder = commands.tasks.first().map(|task| task.folder.clone());
                effects.push(Effect::Commands(CommandsEffect::RunAfter {
                    batch: commands.number,
                    folder: folder.unwrap_or_default(),
                    steps: commands.after_them.clone(),
                }));
            }
            Err(failure) => {
                let said = end_said(&failure.end);
                effects.push(Effect::Commands(CommandsEffect::LogFailure(said)));
                effects.extend(end(state, (key, index), Some(TasksFailure::Task(failure))));
            }
        }
    }
    effects
}

/// The commands of a worktree are over: once all succeeded, and the steps after them ran, what the plan does after
/// them is done; when a command did not succeed, Gitardo says which and offers what the plan offers when it fails,
/// such as opening the worktree all the same; a step after the commands that failed is said as the failure of a plan.
fn end(state: &mut State, (key, index): (&PathBuf, usize), failure: Option<TasksFailure>) -> Vec<Effect> {
    let found = state.tasks_tab.tabs.get_mut(key);
    let Some(commands) = found.and_then(|tab| tab.commands.get_mut(index)) else {
        return Vec::new();
    };
    commands.stage = Stage::Over;
    let (title, after, can_undo) = (commands.title.clone(), commands.after.clone(), commands.can_undo);
    let on_failure = commands.on_failure.clone();
    let ending = match failure {
        None => Ended::Ran {
            title,
            after,
            can_undo,
            prepared_message: None,
            waiting: None,
        },
        Some(TasksFailure::Task(failure)) => {
            state.operations.said = Some(Said {
                runs: worktrees::offered(&on_failure),
                ..Said::error(failure_said(&failure))
            });
            return reads::asked(state, ReadCause::OperationEnded);
        }
        Some(TasksFailure::Step(failure)) => Ended::Failed {
            title: format!("`{}`", failure.failed),
            said: failure.said,
            plan: String::new(),
            on_failure,
        },
    };
    ended_as_said(state, (ending, None))
}

/// What the thread of the commands said: lines for the Command log, or the end of the steps after some commands.
pub fn reduce_commands(state: &mut State, message: CommandsMessage) -> Vec<Effect> {
    match message {
        CommandsMessage::Logged(log) => {
            state.log.append(log);
            Vec::new()
        }
        CommandsMessage::Ended { batch, failure, log } => {
            state.log.append(log);
            let mut tabs = state.tasks_tab.tabs.iter();
            let place = tabs.find_map(|(key, tab)| {
                let index = tab.commands.iter().position(|commands| commands.number == batch)?;
                Some((key.clone(), index))
            });
            place.map_or_else(Vec::new, |(key, index)| end(state, (&key, index), failure))
        }
    }
}
