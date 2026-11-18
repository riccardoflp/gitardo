//! What the Tasks tab is drawn from (`tasks.view`): the rows of its column, by source, as the search and the folds
//! leave them, how each task stands, and the task selected with its terminal.

use model::tasks::{StepState, Task, TaskEnd, TaskReference, TaskSource};
use model::trust::{Trust, is_trusted};
use views::panel::PanelTab;
use views::tasks::{Detail, HeadingRow, Problem, Row, RowStatus, TaskList, TaskRow};
use views::terminal::{TerminalState, TerminalStatus};
use views::text::tasks_tab as words;

use super::super::panel;
use super::super::state::State;
use super::super::terminals::shown_tab;
use super::{Entry, EntryStatus, TabTasks, closing, running_count, selected_entry, shown};

/// The key of the heading **Workspace**.
const WORKSPACE: &str = "workspace";
/// The key of the heading **npm**.
const NPM: &str = "npm";

/// A row of the column before the search and the folds: a heading, or a task with the names it is searched by besides
/// its own.
enum Item<'a> {
    /// A heading, by its key, its words and how far in it is.
    Heading(String, String, u8),
    /// A task, how far in it is, and the names of the headings over it.
    Task(&'a Task, u8, String),
}

/// The rows of a tab, every heading and every task listed: **Workspace** with the tasks of `.vscode/tasks.json` that
/// are not hidden, then **npm** with the scripts of each `package.json` under the name of its folder. A task that
/// runs keeps its row under the heading of its source even when the file no longer has it.
fn items(tab: &TabTasks) -> Vec<Item<'_>> {
    let mut items = Vec::new();
    let tasks = tab.read.as_ref().map(|read| &read.tasks);
    let gone = |source: fn(&TaskSource) -> bool| {
        let running = tab.entries.iter().filter(|entry| entry.is_running());
        let of_source = running.filter(move |entry| source(&entry.task.reference.source));
        of_source
            .filter(move |entry| tasks.is_none_or(|tasks| tasks.find(&entry.task.reference).is_none()))
            .map(|entry| &entry.task)
    };
    let workspace = tasks.map(|tasks| tasks.workspace.as_slice()).unwrap_or_default();
    let listed = workspace.iter().filter(|task| !task.is_hidden);
    let listed: Vec<&Task> = listed.chain(gone(|source| *source == TaskSource::Workspace)).collect();
    if !listed.is_empty() {
        items.push(Item::Heading(WORKSPACE.to_owned(), words::WORKSPACE.to_owned(), 0));
        let over = words::WORKSPACE.to_owned();
        items.extend(listed.into_iter().map(|task| Item::Task(task, 1, over.clone())));
    }
    let packages = tasks.map(|tasks| tasks.packages.as_slice()).unwrap_or_default();
    let gone: Vec<&Task> = gone(|source| matches!(source, TaskSource::Npm(_))).collect();
    if packages.iter().any(|package| !package.scripts.is_empty()) || !gone.is_empty() {
        items.push(Item::Heading(NPM.to_owned(), words::NPM.to_owned(), 0));
    }
    for package in packages.iter().filter(|package| !package.scripts.is_empty()) {
        let key = format!("{NPM}:{}", package.folder);
        items.push(Item::Heading(key, package.name.clone(), 1));
        let over = format!("{} {}", words::NPM, package.name);
        items.extend(package.scripts.iter().map(|task| Item::Task(task, 2, over.clone())));
    }
    items.extend(gone.into_iter().map(|task| Item::Task(task, 1, words::NPM.to_owned())));
    items
}

/// The keys of every heading of a tab, which **Collapse all** folds.
pub(super) fn heading_keys(tab: &TabTasks) -> Vec<String> {
    let headings = items(tab).into_iter().filter_map(|item| match item {
        Item::Heading(key, ..) => Some(key),
        Item::Task(..) => None,
    });
    headings.collect()
}

/// How a task of a tab stands, as its row says it.
fn status(tab: &TabTasks, task: &TaskReference) -> RowStatus {
    let entry = tab.entry(task);
    if let Some(entry) = entry.filter(|entry| entry.is_running()) {
        return RowStatus::Running {
            is_ready: entry.is_ready,
        };
    }
    let is_waiting = |run: &model::tasks::Run| run.state(task) == Some(&StepState::Waiting);
    if tab.runs.iter().any(is_waiting) || tab.restarting.iter().any(|(restarted, _)| restarted == task) {
        return RowStatus::Waiting;
    }
    if tab.blocked.contains_key(task) {
        return RowStatus::DidNotRun;
    }
    match entry.map(|entry| &entry.status) {
        Some(EntryStatus::Ended(Ok(()))) => RowStatus::Succeeded,
        Some(EntryStatus::Ended(Err(TaskEnd::Code(code)))) => RowStatus::Failed(*code),
        Some(EntryStatus::Ended(Err(TaskEnd::Stopped))) => RowStatus::Stopped,
        Some(EntryStatus::Ended(Err(TaskEnd::NotStarted(_)))) => RowStatus::NotStarted,
        Some(EntryStatus::Starting | EntryStatus::Running) | None => RowStatus::NotRun,
    }
}

/// The command of a task as its row shows it: the script of a `package.json` as the file has it, the command line
/// otherwise.
fn shown_command(task: &Task) -> String {
    task.script.clone().unwrap_or_else(|| task.command_line())
}

/// The rows of a tab as the column lists them: with words searched, the tasks whose name, source or group holds every
/// word, under their headings, whatever is folded; otherwise every heading, and the tasks not under a folded one.
fn rows(tab: &TabTasks) -> Vec<Row> {
    let search = tab.search.to_lowercase();
    let searched: Vec<&str> = search.split_whitespace().collect();
    let mut rows: Vec<Row> = Vec::new();
    let mut folded_from: Option<u8> = None;
    for item in items(tab) {
        match item {
            Item::Heading(key, name, depth) => {
                if folded_from.is_some_and(|from| depth > from) {
                    continue;
                }
                let is_folded = searched.is_empty() && tab.folded.contains(&key);
                folded_from = is_folded.then_some(depth);
                rows.push(Row::Heading(HeadingRow {
                    key,
                    name,
                    depth,
                    is_folded,
                }));
            }
            Item::Task(task, depth, over) => {
                let names = format!("{} {over}", task.reference.name).to_lowercase();
                if folded_from.is_some() || !searched.iter().all(|word| names.contains(word)) {
                    continue;
                }
                rows.push(Row::Task(TaskRow {
                    reference: task.reference.clone(),
                    name: task.reference.name.clone(),
                    command: shown_command(task),
                    copied: task.command_line(),
                    depth,
                    status: status(tab, &task.reference),
                    cannot_run: task.not_runnable().map(|why| why.said()),
                    line: task.line,
                    is_selected: tab.selected.as_ref() == Some(&task.reference),
                }));
            }
        }
    }
    if searched.is_empty() {
        rows
    } else {
        without_empty_headings(rows)
    }
}

/// The rows without the headings the search left no task under.
fn without_empty_headings(rows: Vec<Row>) -> Vec<Row> {
    let has_task_under = |index: usize, depth: u8| {
        let under = rows.iter().skip(index + 1);
        let mut under = under.take_while(|row| !matches!(row, Row::Heading(next) if next.depth <= depth));
        under.any(|row| matches!(row, Row::Task(_)))
    };
    let kept: Vec<bool> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| match row {
            Row::Heading(heading) => has_task_under(index, heading.depth),
            Row::Task(_) => true,
        })
        .collect();
    let rows = rows.into_iter().zip(kept);
    rows.filter_map(|(row, is_kept)| is_kept.then_some(row)).collect()
}

/// The two lines under what a task printed once it ended: how it ended, and how long it ran with how to run it again.
fn ended_lines(entry: &Entry) -> Option<(String, String)> {
    let EntryStatus::Ended(end) = &entry.status else {
        return None;
    };
    let again = format!("{}.", words::PRESS_ENTER);
    if let Err(TaskEnd::NotStarted(why)) = end {
        return Some((words::not_started(why), again));
    }
    let is_stopped = matches!(end, Err(TaskEnd::Stopped));
    let seconds = entry.took.unwrap_or_default().as_secs();
    Some((
        words::task_ended(entry.code, is_stopped),
        format!("{}. {again}", words::ran_for(seconds)),
    ))
}

/// What the right side shows of the task selected in a tab.
fn detail(tab: &TabTasks) -> Detail {
    let Some((reference, task)) = tab
        .selected
        .as_ref()
        .and_then(|selected| Some((selected, tab.task(selected)?)))
    else {
        return Detail::default();
    };
    let entry = tab.entry(reference);
    let cannot_run = task.not_runnable().map(|why| why.said());
    Detail {
        reference: Some(reference.clone()),
        name: reference.to_string(),
        command: task.command_line(),
        folder: model::tasks::folder_said(&task.folder, &tab.folder),
        cannot_run: cannot_run.is_some(),
        said: cannot_run.or_else(|| tab.blocked.get(reference).cloned()),
        has_terminal: entry.is_some(),
        is_running: entry.is_some_and(Entry::is_running),
        ended: entry.and_then(ended_lines),
    }
}

/// What the Tasks tab lists: only how many tasks run, and what closing asks, while the bottom panel does not show it.
#[must_use]
pub fn task_list(state: &State) -> TaskList {
    let look = panel::look(state);
    let mut list = TaskList {
        has_repository: shown_tab(state).is_some(),
        running: running_count(state),
        closing: closing::asked_about(state),
        ..TaskList::default()
    };
    let Some((_, folder)) = shown_tab(state).filter(|_| look.is_shown && look.tab == PanelTab::Tasks) else {
        return list;
    };
    list.is_trusted = is_trusted(&folder, &state.persisted.trusted_folders) == Trust::Trusted;
    let Some(tab) = shown(state) else {
        return list;
    };
    list.is_read = tab.read.is_some();
    list.rows = rows(tab);
    list.search.clone_from(&tab.search);
    list.width = tab.width;
    list.detail = detail(tab);
    if let Some(read) = &tab.read {
        list.problems = read
            .tasks
            .problems
            .iter()
            .map(|problem| Problem {
                said: problem.said(),
                file: problem.file.clone(),
            })
            .collect();
        list.problems.extend(read.problem.iter().map(|said| Problem {
            said: said.clone(),
            file: String::new(),
        }));
    }
    list
}

/// The terminal of the task selected, as the Tasks tab draws it.
#[must_use]
pub fn terminal_state(state: &State) -> TerminalState<'_> {
    let entry = selected_entry(state);
    let status = match entry.map(|entry| &entry.status) {
        None | Some(EntryStatus::Starting) => TerminalStatus::Starting,
        Some(EntryStatus::Running) => TerminalStatus::Running,
        // What is said of how a task ended, one that could not start too, is given with the two lines of its end.
        Some(EntryStatus::Ended(_)) => TerminalStatus::Ended(entry.and_then(|entry| entry.code)),
    };
    TerminalState {
        names: Vec::new(),
        shown: 0,
        screen: entry.and_then(|entry| entry.screen.as_ref()),
        status,
        size: state.tasks_tab.size,
        focus: state.panel.focus,
    }
}
