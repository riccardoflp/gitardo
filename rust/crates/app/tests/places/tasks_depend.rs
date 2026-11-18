//! Tasks that wait for others (`tasks.run`): the tasks of `dependsOn` run first, one after the other in `sequence`, a
//! task that failed keeping those that wait for it from running, a background task ready once it prints its
//! `endsPattern`, and a loop refused. Each test runs real commands in the terminals of the tasks.

use std::time::Duration;

use app::store::State;
use model::tasks::TaskReference;
use platform::accesskit::TreeUpdate;
use render::ThemeName;
use testkit::Repository;
use views::tasks::{RowStatus, TasksIntent};

use super::support::Window;
use super::tasks_tab::{Runner, WIDE, entry, printed_line, shell_task, standing, tab, with_tasks};

/// A task that adds a word to `order.txt` in the worktree, with more members of the task, as JSON.
fn adding(label: &str, word: &str, more: &str) -> String {
    let windows = format!("Add-Content -Path order.txt -Value {word}");
    let elsewhere = format!("echo {word} >> order.txt");
    shell_task(label, (&windows, &elsewhere), more)
}

/// The words the tasks added to `order.txt`, in their order.
fn order(repository: &Repository) -> Vec<String> {
    let text = std::fs::read_to_string(repository.path().join("order.txt")).unwrap_or_default();
    text.lines().map(|line| line.trim().to_owned()).collect()
}

/// Asks to run a task of `tasks.json`, without waiting for anything.
fn ask_to_run(runner: &Runner, state: &mut State, label: &str) {
    runner.ask(state, TasksIntent::Run(TaskReference::workspace(label)));
}

/// Where a line is in what the tasks were asked and told.
fn place(runner: &Runner, said: &str) -> usize {
    let log = runner.log.borrow();
    let found = log.iter().position(|line| line == said);
    found.unwrap_or_else(|| panic!("{said} is not in {log:?}"))
}

/// Whether something drawn holds some words.
fn holds(tree: &TreeUpdate, words: &str) -> bool {
    let mut nodes = tree.nodes.iter();
    nodes.any(|(_, node)| node.label().is_some_and(|label| label.contains(words)))
}

/// `all`, which depends on `first` and `second` in sequence, with what `first` runs on each system.
fn sequence(first: (&str, &str)) -> (Repository, State) {
    let depends = "\"dependsOn\": [\"first\", \"second\"], \"dependsOrder\": \"sequence\"";
    with_tasks(&[
        shell_task("first", first, ""),
        adding("second", "second", ""),
        adding("all", "all", depends),
    ])
}

#[test]
fn tasks_run__depends_on_two_tasks_in_sequence_runs_the_second_once_the_first_ended_with_exit_code_0() {
    let (repository, mut state) = sequence(("Add-Content -Path order.txt -Value first", "echo first >> order.txt"));
    let runner = Runner::new();
    runner.open(&mut state);
    ask_to_run(&runner, &mut state, "all");
    runner.until(&mut state, "the end of all", |state| {
        entry(state, "all").is_some_and(|entry| !entry.is_running())
    });
    assert_eq!(order(&repository), ["first", "second", "all"]);
    let id = |label: &str| entry(&state, label).unwrap().id;
    let (first, second, all) = (id("first"), id("second"), id("all"));
    assert!(place(&runner, &format!("ended {first}")) < place(&runner, &format!("start {second}")));
    assert!(place(&runner, &format!("ended {second}")) < place(&runner, &format!("start {all}")));
    for label in ["first", "second", "all"] {
        assert_eq!(standing(&state, label), Some(RowStatus::Succeeded), "{label}");
    }
    // The tab is shown at the task asked for, not at those it depends on.
    assert_eq!(tab(&state).unwrap().selected, Some(TaskReference::workspace("all")));
}

#[test]
fn tasks_run__depends_on_two_tasks_in_sequence_does_not_run_the_second_when_the_first_ended_with_1() {
    let (repository, mut state) = sequence(("exit 1", "exit 1"));
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    ask_to_run(&runner, &mut state, "all");
    runner.until(&mut state, "the end of first", |state| {
        entry(state, "first").is_some_and(|entry| !entry.is_running())
    });
    runner.for_a_while(&mut state, Duration::from_millis(500));
    assert_eq!(standing(&state, "first"), Some(RowStatus::Failed(1)));
    for label in ["second", "all"] {
        assert!(entry(&state, label).is_none(), "{label} has no terminal");
        assert_eq!(standing(&state, label), Some(RowStatus::DidNotRun), "{label}");
    }
    assert_eq!(order(&repository), Vec::<String>::new(), "nothing ran after first");
    assert_eq!(
        runner
            .log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("start "))
            .count(),
        1
    );
    // The tab is shown at the task asked for, which says why it did not run where its terminal would be.
    let detail = app::store::tasks_tab::task_list(&state).detail;
    assert_eq!(detail.name, "all");
    assert_eq!(
        detail.said.as_deref(),
        Some("It did not run: `first` ended with exit code 1.")
    );
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(holds(&tree, "did not run"), "its row says that it did not run");
}

#[test]
fn tasks_run__a_task_that_depends_on_a_background_task_starts_once_that_one_prints_ready_while_it_keeps_running() {
    let background = "\"isBackground\": true, \"problemMatcher\": { \"pattern\": { \"regexp\": \"^x$\" }, \
                      \"background\": { \"beginsPattern\": \"starting\", \"endsPattern\": \"ready\" } }";
    let server = shell_task(
        "server",
        (
            "echo starting; Start-Sleep -Milliseconds 700; echo ready; Start-Sleep 600",
            "echo starting; sleep 1; echo ready; sleep 600",
        ),
        background,
    );
    let client = adding("client", "client", "\"dependsOn\": [\"server\"]");
    let (repository, mut state) = with_tasks(&[server, client]);
    let runner = Runner::new();
    runner.open(&mut state);
    ask_to_run(&runner, &mut state, "client");
    runner.until(&mut state, "what the server prints first", |state| {
        printed_line(state, "server", "starting")
    });
    assert!(
        entry(&state, "client").is_none(),
        "the client waits for the server to be ready"
    );
    assert_eq!(standing(&state, "client"), Some(RowStatus::Waiting));
    runner.until(&mut state, "the end of the client", |state| {
        entry(state, "client").is_some_and(|entry| !entry.is_running())
    });
    assert_eq!(order(&repository), ["client"]);
    let (server, client) = (entry(&state, "server").unwrap(), entry(&state, "client").unwrap());
    assert!(server.is_running(), "the server keeps running");
    assert!(place(&runner, &format!("ready {}", server.id)) < place(&runner, &format!("start {}", client.id)));
    assert_eq!(standing(&state, "server"), Some(RowStatus::Running { is_ready: true }));
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("server")));
    runner.until(&mut state, "the end of the server", |state| {
        !entry(state, "server").unwrap().is_running()
    });
}

#[test]
fn tasks_run__two_tasks_that_depend_on_each_other_run_nothing_and_the_loop_is_said() {
    let (repository, mut state) = with_tasks(&[
        adding("one", "one", "\"dependsOn\": [\"two\"]"),
        adding("two", "two", "\"dependsOn\": [\"one\"]"),
    ]);
    let runner = Runner::new();
    runner.open(&mut state);
    ask_to_run(&runner, &mut state, "one");
    runner.until(&mut state, "the refusal", |state| state.operations.said.is_some());
    let said = state.operations.said.clone().unwrap();
    assert_eq!(
        said.text,
        "Nothing ran: the tasks depend on each other in a loop, `one` → `two` → `one`."
    );
    assert!(said.is_error);
    assert!(runner.log.borrow().is_empty(), "{:?}", runner.log.borrow());
    assert!(tab(&state).unwrap().entries.is_empty(), "no task has a terminal");
    assert_eq!(order(&repository), Vec::<String>::new());
}
