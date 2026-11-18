//! What the Tasks tab is drawn from (`tasks.view`): the rows of its column, by source, as the search and the folds
//! leave them, how each task stands, and the task selected with its terminal.

use model::tasks::{StepState, Task, TaskEnd, TaskReference, TaskSource};
use model::trust::{Trust, is_trusted};
use views::panel::PanelTab;
use views::tasks::{Detail, HeadingKind, HeadingRow, Problem, Row, RowStatus, TaskList, TaskRow};
use views::terminal::{TerminalState, TerminalStatus};
use views::text::tasks_tab as words;

use super::super::panel;
use super::super::state::State;
use super::super::terminals::shown_tab;
use super::{Entry, EntryStatus, TabTasks, closing, running_count, selected_entry, shown};

/// The key of the heading **Groups**.
const GROUPS: &str = "groups";
/// The key of the heading **Workspace**.
const WORKSPACE: &str = "workspace";
/// The key of the heading **npm**.
const NPM: &str = "npm";

/// The key of the heading **New worktrees**.
pub(super) const NEW_WORKTREES: &str = "new worktrees";

/// The key of the heading of the commands of a new worktree, by its name.
pub(super) fn worktree_key(name: &str) -> String {
    format!("{NEW_WORKTREES}:{name}")
}

/// The key of the heading of a group, by its name.
pub(super) fn group_key(name: &str) -> String {
    format!("group:{name}")
}

/// A row of the column before the search and the folds: a heading, or a task with the names it is searched by besides
/// its own.
pub(super) enum Item<'a> {
    /// A heading, not folded yet.
    Heading(HeadingRow),
    /// A task, how far in it is, the names of the headings over it, and the group its row is in.
    Task(&'a Task, u8, String, Option<String>),
    /// A task of a group that the worktree shown does not have, with the name of the group.
    Missing(&'a TaskReference, String),
}

/// A heading of a source, by its key, its words and how far in it is.
fn heading(key: &str, name: &str, depth: u8) -> HeadingRow {
    HeadingRow {
        kind: HeadingKind::Source,
        key: key.to_owned(),
        name: name.to_owned(),
        depth,
        is_folded: false,
    }
}

/// **Groups** and the groups of the repository, each with its tasks in their order: a task the worktree shown does
/// not have keeps its row, which says so.
fn group_items(tab: &TabTasks) -> Vec<Item<'_>> {
    let Some(read) = &tab.read else {
        return Vec::new();
    };
    let mut items = vec![Item::Heading(HeadingRow {
        kind: HeadingKind::Groups,
        ..heading(GROUPS, words::GROUPS, 0)
    })];
    for group in &read.groups {
        let mut runs = group.tasks.iter().filter_map(|task| tab.entry(task));
        let kind = HeadingKind::Group {
            name: group.name.clone(),
            is_running: runs.any(Entry::is_running),
            is_selected: tab.group_selected.as_ref() == Some(&group.name),
        };
        items.push(Item::Heading(HeadingRow {
            kind,
            ..heading(&group_key(&group.name), &group.name, 1)
        }));
        let over = format!("{} {}", words::GROUPS, group.name);
        items.extend(group.tasks.iter().map(|reference| match read.tasks.find(reference) {
            Some(task) => Item::Task(task, 2, over.clone(), Some(group.name.clone())),
            None => Item::Missing(reference, group.name.clone()),
        }));
    }
    items
}

/// The rows of a tab, every heading and every task listed: **Groups** with the groups of the repository, when it has
/// groups or the worktree has tasks, then **Workspace** with the tasks of `.vscode/tasks.json` that are not hidden,
/// then **npm** with the scripts of each `package.json` under the name of its folder, then **New worktrees** with the
/// commands of each worktree made in the session under its name. A task that runs keeps its row under the heading of
/// its source even when the file no longer has it.
pub(super) fn items(tab: &TabTasks) -> Vec<Item<'_>> {
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
        items.push(Item::Heading(heading(WORKSPACE, words::WORKSPACE, 0)));
        let over = words::WORKSPACE.to_owned();
        items.extend(listed.into_iter().map(|task| Item::Task(task, 1, over.clone(), None)));
    }
    let packages = tasks.map(|tasks| tasks.packages.as_slice()).unwrap_or_default();
    let gone: Vec<&Task> = gone(|source| matches!(source, TaskSource::Npm(_))).collect();
    if packages.iter().any(|package| !package.scripts.is_empty()) || !gone.is_empty() {
        items.push(Item::Heading(heading(NPM, words::NPM, 0)));
    }
    for package in packages.iter().filter(|package| !package.scripts.is_empty()) {
        let key = format!("{NPM}:{}", package.folder);
        items.push(Item::Heading(heading(&key, &package.name, 1)));
        let over = format!("{} {}", words::NPM, package.name);
        items.extend(
            package
                .scripts
                .iter()
                .map(|task| Item::Task(task, 2, over.clone(), None)),
        );
    }
    items.extend(
        gone.into_iter()
            .map(|task| Item::Task(task, 1, words::NPM.to_owned(), None)),
    );
    let has_groups = tab.read.as_ref().is_some_and(|read| !read.groups.is_empty());
    let mut all = if items.is_empty() && !has_groups {
        Vec::new()
    } else {
        group_items(tab)
    };
    all.append(&mut items);
    all.extend(command_items(tab));
    all
}

/// **New worktrees** and, under the name of each worktree made in the session, its commands in the order they run.
fn command_items(tab: &TabTasks) -> Vec<Item<'_>> {
    let mut items = Vec::new();
    if !tab.commands.is_empty() {
        items.push(Item::Heading(heading(NEW_WORKTREES, words::NEW_WORKTREES, 0)));
    }
    for commands in &tab.commands {
        let key = worktree_key(&commands.worktree);
        items.push(Item::Heading(heading(&key, &commands.worktree, 1)));
        let over = format!("{} {}", words::NEW_WORKTREES, commands.worktree);
        let tasks = commands.tasks.iter();
        items.extend(tasks.map(|task| Item::Task(task, 2, over.clone(), None)));
    }
    items
}

/// The keys of every heading of a tab, which **Collapse all** folds.
pub(super) fn heading_keys(tab: &TabTasks) -> Vec<String> {
    let headings = items(tab).into_iter().filter_map(|item| match item {
        Item::Heading(heading) => Some(heading.key),
        Item::Task(..) | Item::Missing(..) => None,
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
pub(super) fn shown_command(task: &Task) -> String {
    task.script.clone().unwrap_or_else(|| task.command_line())
}

/// The row of a task of a tab, at a depth and in a group or under its source.
fn task_row(tab: &TabTasks, task: &Task, (depth, group): (u8, Option<String>)) -> TaskRow {
    TaskRow {
        reference: task.reference.clone(),
        group,
        name: task.reference.name.clone(),
        // The name of a command of a new worktree is its command already.
        command: if task.reference.is_of_new_worktree() {
            String::new()
        } else {
            shown_command(task)
        },
        copied: task.command_line(),
        depth,
        status: status(tab, &task.reference),
        cannot_run: task.not_runnable().map(|why| why.said()),
        line: task.line,
        is_selected: tab.selected.as_ref() == Some(&task.reference),
    }
}

/// The row of a task of a group that the worktree shown does not have: dimmed, saying so.
fn missing_row(tab: &TabTasks, reference: &TaskReference, group: String) -> TaskRow {
    TaskRow {
        reference: reference.clone(),
        group: Some(group),
        name: reference.name.clone(),
        command: String::new(),
        copied: String::new(),
        depth: 2,
        status: RowStatus::NotRun,
        cannot_run: Some(words::not_in_worktree(&reference.to_string())),
        line: None,
        is_selected: tab.selected.as_ref() == Some(reference),
    }
}

/// The rows of a tab as the column lists them: with words searched, the tasks whose name, source or group holds every
/// word, under their headings, whatever is folded; otherwise every heading, and the tasks not under a folded one.
fn rows(tab: &TabTasks) -> Vec<Row> {
    let search = tab.search.to_lowercase();
    let searched: Vec<&str> = search.split_whitespace().collect();
    let is_found = |name: &str, over: &str| {
        let names = format!("{name} {over}").to_lowercase();
        searched.iter().all(|word| names.contains(word))
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut folded_from: Option<u8> = None;
    for item in items(tab) {
        match item {
            Item::Heading(heading) => {
                if folded_from.is_some_and(|from| heading.depth > from) {
                    continue;
                }
                let is_folded = searched.is_empty() && tab.folded.contains(&heading.key);
                folded_from = is_folded.then_some(heading.depth);
                rows.push(Row::Heading(HeadingRow { is_folded, ..heading }));
            }
            Item::Task(task, depth, over, group) => {
                if folded_from.is_none() && is_found(&task.reference.name, &over) {
                    rows.push(Row::Task(task_row(tab, task, (depth, group))));
                }
            }
            Item::Missing(reference, group) => {
                if folded_from.is_none() && is_found(&reference.name, &format!("{} {group}", words::GROUPS)) {
                    rows.push(Row::Task(missing_row(tab, reference, group)));
                }
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
    // A command of a new worktree runs once: nothing runs it again.
    let again = if entry.task.reference.is_of_new_worktree() {
        String::new()
    } else {
        format!("{}.", words::PRESS_ENTER)
    };
    if let Err(TaskEnd::NotStarted(why)) = end {
        return Some((words::not_started(why), again));
    }
    let is_stopped = matches!(end, Err(TaskEnd::Stopped));
    let seconds = entry.took.unwrap_or_default().as_secs();
    Some((
        words::task_ended(entry.code, is_stopped),
        format!("{}. {again}", words::ran_for(seconds)).trim_end().to_owned(),
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
        // A command of a new worktree is named by its command already.
        command: if reference.is_of_new_worktree() {
            String::new()
        } else {
            task.command_line()
        },
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
    list.reveal = tab.revealed.as_ref().and_then(|(serial, key)| {
        let is_heading = |row: &Row| matches!(row, Row::Heading(heading) if heading.key == *key);
        Some((*serial, list.rows.iter().position(is_heading)?))
    });
    if let Some(read) = &tab.read {
        list.groups = read.groups.iter().map(|group| group.name.clone()).collect();
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
