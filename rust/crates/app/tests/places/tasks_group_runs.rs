//! The tasks of a group run and stopped from the Tasks tab (`tasks.run-group`): **Run all** on the heading of a group,
//! a task of it that runs already, **Stop all**, and a task the worktree does not have. Each test runs real commands
//! in the terminals of the tasks.

use app::store::State;
use model::tasks::TaskReference;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::tasks::{RowStatus, TasksIntent};

use super::support::{Window, has};
use super::tasks_groups::{group, write_groups};
use super::tasks_tab::{Runner, WIDE, entry, printed_line, standing, tab, waiting_task, with_tasks};

/// A repository whose tasks `one` and `two` print their names and wait for a line, with the group `Servers` of some
/// tasks, and its Tasks tab shown.
fn with_group(tasks: &[&str], runner: &Runner) -> (Repository, State) {
    let (repository, mut state) = with_tasks(&[waiting_task("one", "one"), waiting_task("two", "two")]);
    write_groups(&repository, &[group("Servers", tasks)]);
    runner.open(&mut state);
    (repository, state)
}

/// Waits until some tasks printed their names.
fn until_printed(runner: &Runner, state: &mut State, labels: &[&str]) {
    for label in labels {
        runner.until(state, &format!("what {label} prints"), |state| {
            printed_line(state, label, label)
        });
    }
}

/// Waits until no task runs.
fn until_none_runs(runner: &Runner, state: &mut State) {
    runner.until(state, "the end of every task", |state| {
        tab(state).is_some_and(|tab| tab.running().is_empty())
    });
}

/// Where a line is in what the tasks were asked and told.
fn place(runner: &Runner, said: &str) -> usize {
    let log = runner.log.borrow();
    let found = log.iter().position(|line| line == said);
    found.unwrap_or_else(|| panic!("{said} is not in {log:?}"))
}

#[test]
fn tasks_run_group__run_all_on_a_group_of_two_tasks_runs_both_the_first_started_first() {
    let runner = Runner::new();
    let (_repository, mut state) = with_group(&["one", "two"], &runner);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Run all") && !has(&tree, Role::Button, "Stop all"));
    runner.click(&mut state, &mut window, Role::Button, "Run all");
    until_printed(&runner, &mut state, &["one", "two"]);
    let (one, two) = (entry(&state, "one").unwrap(), entry(&state, "two").unwrap());
    assert!(
        one.is_running() && two.is_running(),
        "the second does not wait for the first to end"
    );
    assert!(place(&runner, &format!("start {}", one.id)) < place(&runner, &format!("start {}", two.id)));
    assert_eq!(tab(&state).unwrap().selected, Some(TaskReference::workspace("one")));
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::Button, "Stop all") && !has(&tree, Role::Button, "Run all"),
        "Stop all in place of Run all while a task of the group runs"
    );
    assert!(has(&tree, Role::Button, "Tasks (2)"));
    runner.ask(&mut state, TasksIntent::StopGroup(String::from("Servers")));
    until_none_runs(&runner, &mut state);
}

#[test]
fn tasks_run_group__a_task_of_the_group_already_running_is_not_run_again_nor_asked_about() {
    let runner = Runner::new();
    let (_repository, mut state) = with_group(&["one", "two"], &runner);
    runner.run_task(&mut state, "one");
    until_printed(&runner, &mut state, &["one"]);
    let first = entry(&state, "one").unwrap().id;
    runner.ask(&mut state, TasksIntent::RunGroup(String::from("Servers")));
    until_printed(&runner, &mut state, &["two"]);
    runner.ended(&mut state);
    assert_eq!(
        entry(&state, "one").unwrap().id,
        first,
        "the task that ran goes on in its terminal"
    );
    assert!(entry(&state, "one").unwrap().is_running());
    let log = runner.log.borrow().clone();
    assert_eq!(
        log.iter().filter(|line| line.starts_with("start ")).count(),
        2,
        "{log:?}"
    );
    assert!(!log.iter().any(|line| line.starts_with("stop ")), "{log:?}");
    runner.ask(&mut state, TasksIntent::StopGroup(String::from("Servers")));
    until_none_runs(&runner, &mut state);
}

#[test]
fn tasks_run_group__stop_all_ends_both() {
    let runner = Runner::new();
    let (_repository, mut state) = with_group(&["one", "two"], &runner);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.ask(&mut state, TasksIntent::RunGroup(String::from("Servers")));
    until_printed(&runner, &mut state, &["one", "two"]);
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::Button, "Stop all");
    until_none_runs(&runner, &mut state);
    for label in ["one", "two"] {
        assert_eq!(standing(&state, label), Some(RowStatus::Stopped), "{label}");
    }
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Run all"), "Run all again once none runs");
    // Stopped again, the group says that none of its tasks runs.
    runner.ask(&mut state, TasksIntent::StopGroup(String::from("Servers")));
    runner.until(&mut state, "the refusal", |state| state.operations.said.is_some());
    assert_eq!(
        state.operations.said.clone().unwrap().text,
        "No task of `Servers` is running."
    );
}

#[test]
fn tasks_run_group__a_task_of_the_group_missing_from_the_worktree_is_left_out_the_others_run_and_it_is_said() {
    let runner = Runner::new();
    let (_repository, mut state) = with_group(&["one", "gone", "two"], &runner);
    runner.ask(&mut state, TasksIntent::RunGroup(String::from("Servers")));
    until_printed(&runner, &mut state, &["one", "two"]);
    runner.until(&mut state, "what is said", |state| state.operations.said.is_some());
    assert_eq!(
        state.operations.said.clone().unwrap().text,
        "`gone` is not in this worktree."
    );
    assert!(entry(&state, "gone").is_none());
    assert_eq!(tab(&state).unwrap().running().len(), 2);
    runner.ask(&mut state, TasksIntent::StopGroup(String::from("Servers")));
    until_none_runs(&runner, &mut state);
}
