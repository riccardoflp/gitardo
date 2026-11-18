//! **Run Task…** (`tasks.quick-run`): the picker of the tasks and the groups of the repository shown, what it lists
//! and what choosing an entry of it does.

use views::panel::{PanelIntent, PanelTab};
use views::tasks::{HeadingKind, QuickRunChoice, QuickRunEntry, QuickRunIntent, QuickRunState, TasksIntent};
use views::text::tasks_tab as words;

use super::super::message::Effect;
use super::super::panel;
use super::super::state::State;
use super::view::{Item, items, shown_command};
use super::{Entry, TabTasks, select, shown};

/// The picker of the tasks, while it is open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QuickRun {
    /// What is typed in its box.
    pub typed: String,
}

/// What was asked of the picker: it opens over a repository, with the tasks of the worktree shown read when they
/// were not yet; and a task or a group chosen is shown in the Tasks tab and run, a task that runs only shown.
pub(super) fn reduce(state: &mut State, intent: QuickRunIntent) -> Vec<Effect> {
    match intent {
        QuickRunIntent::Open => {
            if state.shown_read().is_some() {
                state.tasks_tab.quick_run = Some(QuickRun::default());
            }
            Vec::new()
        }
        QuickRunIntent::Type(typed) => {
            if let Some(open) = &mut state.tasks_tab.quick_run {
                open.typed = typed;
            }
            Vec::new()
        }
        QuickRunIntent::Close => {
            state.tasks_tab.quick_run = None;
            Vec::new()
        }
        QuickRunIntent::Choose(choice) => {
            state.tasks_tab.quick_run = None;
            let mut effects = panel::reduce(state, &PanelIntent::Show(PanelTab::Tasks));
            match choice {
                QuickRunChoice::Task(task) => {
                    let entry = shown(state).and_then(|tab| tab.entry(&task));
                    if entry.is_some_and(Entry::is_running) {
                        select(state, task);
                    } else {
                        effects.extend(super::reduce(state, TasksIntent::Run(task)));
                    }
                }
                QuickRunChoice::Group(group) => effects.extend(super::reduce(state, TasksIntent::RunGroup(group))),
            }
            effects
        }
    }
}

/// The entry of a group: its tasks under its name, and whether one of them runs.
fn group_entry(tab: &TabTasks, name: &str, is_running: bool) -> QuickRunEntry {
    let groups = tab.read.iter().flat_map(|read| read.groups.iter());
    let tasks = groups.filter(|group| group.name == name).flat_map(|group| &group.tasks);
    let names: Vec<&str> = tasks.map(|task| task.name.as_str()).collect();
    QuickRunEntry {
        name: name.to_owned(),
        under: names.join(", "),
        source: words::GROUPS.to_owned(),
        is_running,
        choice: QuickRunChoice::Group(name.to_owned()),
    }
}

/// The entries of the picker of a tab: the groups first, then the tasks by source, as the Tasks tab orders them, left
/// when their names and where they are from hold every word typed.
fn entries_of(tab: &TabTasks, typed: &str) -> Vec<QuickRunEntry> {
    let typed = typed.to_lowercase();
    let listed = items(tab).into_iter().filter_map(|item| match item {
        Item::Heading(heading) => match heading.kind {
            HeadingKind::Group { name, is_running, .. } => Some(group_entry(tab, &name, is_running)),
            HeadingKind::Source | HeadingKind::Groups => None,
        },
        // A command of a new worktree runs once, when the worktree is made.
        Item::Task(task, ..) if task.reference.is_of_new_worktree() => None,
        // A task is listed once, under its source, however many groups have it.
        Item::Task(task, _, source, None) => Some(QuickRunEntry {
            name: task.reference.name.clone(),
            under: shown_command(task),
            source,
            is_running: tab.entry(&task.reference).is_some_and(Entry::is_running),
            choice: QuickRunChoice::Task(task.reference.clone()),
        }),
        Item::Task(_, _, _, Some(_)) | Item::Missing(..) => None,
    });
    let holds = |entry: &QuickRunEntry| {
        let names = format!("{} {}", entry.name, entry.source).to_lowercase();
        typed.split_whitespace().all(|word| names.contains(word))
    };
    listed.filter(holds).collect()
}

/// The entries of the picker, made only while it is open.
#[must_use]
pub fn quick_run_entries(state: &State) -> Vec<QuickRunEntry> {
    let Some(open) = &state.tasks_tab.quick_run else {
        return Vec::new();
    };
    shown(state).map_or_else(Vec::new, |tab| entries_of(tab, &open.typed))
}

/// What the picker shows, with its entries.
#[must_use]
pub fn quick_run_state<'a>(state: &'a State, entries: &'a [QuickRunEntry]) -> QuickRunState<'a> {
    let open = state.tasks_tab.quick_run.as_ref();
    QuickRunState {
        is_open: open.is_some(),
        typed: open.map_or("", |open| open.typed.as_str()),
        entries,
        is_reading: shown(state).is_none_or(|tab| tab.read.is_none()),
    }
}
