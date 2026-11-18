//! The groups of the Tasks tab made and changed (`tasks.edit-groups`): a group made from the heading **Groups**, a
//! task dragged onto a group and to a place in it, the groups of another worktree of the repository, a task the
//! worktree no longer has, a name another group has, and the file the groups are kept in.

use std::time::Duration;

use app::store::{Message, State};
use features::tasks::groups_file::{groups_path, groups_text, parse_groups};
use model::contract::Answer;
use model::repo::Change;
use model::tasks::{GroupsAction, Shift, TaskGroup, TaskReference};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use views::tasks::TasksIntent;

use super::pointer::{middle, rect};
use super::showing;
use super::support::{Window, find, has};
use super::tasks_tab::{Runner, WIDE, shell_task, tab, with_tasks, write_tasks};

/// A task that prints its own name.
fn plain(label: &str) -> String {
    let command = format!("echo {label}");
    shell_task(label, (&command, &command), "")
}

/// A group of tasks of `tasks.json`, by their labels.
pub(super) fn group(name: &str, tasks: &[&str]) -> TaskGroup {
    TaskGroup {
        name: name.to_owned(),
        tasks: tasks.iter().map(|task| TaskReference::workspace(task)).collect(),
    }
}

/// Writes the file of the groups of a repository, as a change of the groups writes it.
pub(super) fn write_groups(repository: &Repository, groups: &[TaskGroup]) {
    let path = groups_path(&repository.path().join(".git"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, groups_text(groups)).unwrap();
}

/// The groups the file of a repository holds.
fn kept(repository: &Repository) -> Vec<TaskGroup> {
    let path = groups_path(&repository.path().join(".git"));
    let text = std::fs::read_to_string(path).ok();
    parse_groups(text.as_deref()).unwrap()
}

/// The groups the tab of a window lists, as last read.
pub(super) fn listed(state: &State) -> Vec<TaskGroup> {
    let read = tab(state).and_then(|tab| tab.read.as_ref());
    read.map(|read| read.groups.clone()).unwrap_or_default()
}

/// Asks a change of the groups, as the tab does.
fn change(runner: &Runner, state: &mut State, action: GroupsAction) {
    runner.ask(state, TasksIntent::Groups(action));
}

/// Waits until the tab lists some groups.
fn until_listed(runner: &Runner, state: &mut State, groups: &[TaskGroup]) {
    runner.until(state, &format!("the groups {groups:?}"), |state| {
        listed(state) == groups
    });
}

/// The middle of each row of a task of a name, from the top of the column.
fn rows_of(tree: &TreeUpdate, name: &str) -> Vec<Point> {
    let start = format!("{name}, ");
    let rows = tree.nodes.iter().filter(|(_, node)| {
        let is_named = |label: &str| label == name || label.starts_with(&start);
        node.role() == Role::ListBoxOption && node.label().is_some_and(is_named)
    });
    let mut places: Vec<Point> = rows.map(|(_, node)| middle(rect(node.bounds().unwrap()))).collect();
    places.sort_by(|one, other| one.y.total_cmp(&other.y));
    places
}

/// The middle of the left half of a heading, where no button of its own is.
fn heading_at(tree: &TreeUpdate, name: &str) -> Point {
    let bounds = rect(find(tree, Role::Button, name).bounds().unwrap());
    Point::new(bounds.x + bounds.width * 0.3, bounds.y + bounds.height * 0.5)
}

#[test]
fn tasks_edit_groups__new_group_named_servers_lists_an_empty_group_under_groups() {
    let (repository, mut state) = with_tasks(&[plain("build")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Groups"), "the heading of the groups");
    runner.click(&mut state, &mut window, Role::Button, "New group");
    runner.answer(&mut state, Answer::Text(String::from("Servers")));
    until_listed(&runner, &mut state, &[group("Servers", &[])]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    let (groups, servers) = (heading_at(&tree, "Groups"), heading_at(&tree, "Servers"));
    let workspace = heading_at(&tree, "Workspace");
    assert!(
        groups.y < servers.y && servers.y < workspace.y,
        "the group is under Groups, before the sources"
    );
    assert!(has(&tree, Role::Button, "Run all"), "its heading has Run all");
    assert_eq!(kept(&repository), [group("Servers", &[])]);
}

#[test]
fn tasks_edit_groups__a_task_dragged_onto_servers_is_listed_in_it_and_above_another_task_it_goes_before_it() {
    let (repository, mut state) = with_tasks(&[plain("build"), plain("test")]);
    write_groups(&repository, &[group("Servers", &[])]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    let (from, onto) = (rows_of(&tree, "build")[0], heading_at(&tree, "Servers"));
    runner.drag(&mut state, &mut window, from, onto);
    until_listed(&runner, &mut state, &[group("Servers", &["build"])]);
    // Its row in the group is the first of its name; `test`, dragged over it from under its source, goes before it.
    let tree = runner.draw(&mut state, &mut window, &[]);
    let (builds, tests) = (rows_of(&tree, "build"), rows_of(&tree, "test"));
    assert_eq!(
        (builds.len(), tests.len()),
        (2, 1),
        "build is in the group and under Workspace"
    );
    runner.drag(&mut state, &mut window, tests[0], builds[0]);
    until_listed(&runner, &mut state, &[group("Servers", &["test", "build"])]);
    // A task of the group dragged over another of the group goes before it too.
    let tree = runner.draw(&mut state, &mut window, &[]);
    let (from, onto) = (rows_of(&tree, "build")[0], rows_of(&tree, "test")[0]);
    runner.drag(&mut state, &mut window, from, onto);
    until_listed(&runner, &mut state, &[group("Servers", &["build", "test"])]);
    assert_eq!(kept(&repository), [group("Servers", &["build", "test"])]);
}

#[test]
fn tasks_edit_groups__the_groups_are_the_same_in_another_worktree_without_reading_anything_again_by_hand() {
    let (repository, mut state) = with_tasks(&[plain("build")]);
    let other = repository.path().join("other");
    let _ = repository.git(&["worktree", "add", "-b", "other", other.to_str().unwrap()]);
    let mut there = showing(other.clone());
    there.persisted.trusted_folders.push(other);
    let (runner, runner_there) = (Runner::new(), Runner::new());
    runner.open(&mut state);
    runner_there.open(&mut there);
    assert_eq!(listed(&there), Vec::new(), "no groups yet");
    change(
        &runner,
        &mut state,
        GroupsAction::New {
            task: Some(TaskReference::workspace("build")),
        },
    );
    runner.answer(&mut state, Answer::Text(String::from("Servers")));
    until_listed(&runner, &mut state, &[group("Servers", &["build"])]);
    // What the watcher of the other window tells of the file written in the common git folder.
    let changes = vec![Change::GitFolder("gitardo/task-groups.json".into())];
    let at = Duration::from_secs(10);
    runner_there.message(&mut there, Message::Changed { changes, at });
    until_listed(&runner_there, &mut there, &[group("Servers", &["build"])]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = runner_there.draw(&mut there, &mut window, &[]);
    assert!(
        has(&tree, Role::Button, "Servers"),
        "the group is listed in the other worktree"
    );
}

#[test]
fn tasks_edit_groups__a_task_removed_from_tasks_json_stays_in_its_group_dimmed_saying_it_is_not_in_this_worktree() {
    let (repository, mut state) = with_tasks(&[plain("build"), plain("test")]);
    write_groups(&repository, &[group("Servers", &["build", "test"])]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert_eq!(rows_of(&tree, "build").len(), 2);
    write_tasks(&repository, &[plain("test")]);
    let changes = vec![Change::WorkingTree(".vscode/tasks.json".into())];
    let at = Duration::from_secs(10);
    runner.message(&mut state, Message::Changed { changes, at });
    runner.until(&mut state, "the tasks read again", |state| {
        let read = tab(state).and_then(|tab| tab.read.as_ref());
        read.is_some_and(|read| read.tasks.workspace.len() == 1)
    });
    assert_eq!(
        listed(&state),
        [group("Servers", &["build", "test"])],
        "the group keeps it"
    );
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(
        &tree,
        Role::ListBoxOption,
        "build, `build` is not in this worktree."
    ));
    assert_eq!(rows_of(&tree, "build").len(), 1, "only the group lists it");
    let rows = app::store::tasks_tab::task_list(&state).rows;
    let missing = rows.iter().find_map(|row| match row {
        views::tasks::Row::Task(task) if task.name == "build" => Some(task),
        _ => None,
    });
    assert!(
        missing.unwrap().cannot_run.is_some(),
        "its row is dimmed and cannot be run"
    );
}

#[test]
fn tasks_edit_groups__renaming_a_group_to_the_name_of_another_is_refused_saying_so() {
    let (repository, mut state) = with_tasks(&[plain("build")]);
    let groups = [group("Servers", &["build"]), group("Checks", &[])];
    write_groups(&repository, &groups);
    let runner = Runner::new();
    runner.open(&mut state);
    change(
        &runner,
        &mut state,
        GroupsAction::Rename {
            group: String::from("Checks"),
        },
    );
    runner.answer(&mut state, Answer::Text(String::from("Servers")));
    runner.until(&mut state, "the refusal", |state| state.operations.said.is_some());
    let said = state.operations.said.clone().unwrap();
    assert_eq!(said.text, "There is already a group named `Servers`.");
    assert!(said.is_error);
    assert_eq!(kept(&repository), groups, "nothing was written");
    // `F2` renames the group whose heading was clicked last.
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = runner.draw(&mut state, &mut window, &[]);
    let intents = window.click_at(&state, heading_at(&tree, "Checks"), 1);
    runner.intents(&mut state, intents);
    let rename = ui::KeyEvent::press(ui::Key::Named(ui::NamedKey::Function(2)), ui::Modifiers::NONE);
    let _ = runner.draw(&mut state, &mut window, &[ui::InputEvent::KeyPressed(rename)]);
    runner.answer(&mut state, Answer::Text(String::from("Lints")));
    until_listed(
        &runner,
        &mut state,
        &[group("Servers", &["build"]), group("Lints", &[])],
    );
}

#[test]
fn tasks_edit_groups__the_file_written_by_a_change_is_read_back_with_the_same_groups_in_the_same_order() {
    let (repository, mut state) = with_tasks(&[plain("build"), plain("test")]);
    let runner = Runner::new();
    runner.open(&mut state);
    for (name, task) in [("Servers", "build"), ("Checks", "test")] {
        change(
            &runner,
            &mut state,
            GroupsAction::New {
                task: Some(TaskReference::workspace(task)),
            },
        );
        runner.answer(&mut state, Answer::Text(name.to_owned()));
        runner.until(&mut state, name, |state| {
            listed(state).iter().any(|group| group.name == name)
        });
    }
    change(
        &runner,
        &mut state,
        GroupsAction::ShiftGroup {
            group: String::from("Checks"),
            shift: Shift::Up,
        },
    );
    let groups = [group("Checks", &["test"]), group("Servers", &["build"])];
    until_listed(&runner, &mut state, &groups);
    assert_eq!(kept(&repository), groups);
    // Another window on the repository reads the same.
    let mut again = showing(repository.path().to_path_buf());
    let runner_again = Runner::new();
    runner_again.open(&mut again);
    assert_eq!(listed(&again), groups);
    // A group deleted asks first, and its tasks stay where they are.
    change(
        &runner,
        &mut state,
        GroupsAction::Delete {
            group: String::from("Servers"),
        },
    );
    runner.answer(&mut state, Answer::Confirmed);
    until_listed(&runner, &mut state, &[group("Checks", &["test"])]);
    let read = tab(&state).unwrap().read.as_ref().unwrap();
    assert!(read.tasks.find(&TaskReference::workspace("build")).is_some());
}
